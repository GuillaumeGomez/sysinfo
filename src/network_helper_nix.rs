// Take a look at the license at the top of the repository in the LICENSE file.

use crate::common::MacAddr;
use std::ptr::null_mut;

/// This iterator yields an interface name and address.
pub(crate) struct InterfaceAddressIterator {
    /// Pointer to the current `ifaddrs` struct.
    ifap: *mut libc::ifaddrs,
    /// Pointer to the first element in linked list.
    buf: *mut libc::ifaddrs,
}

impl Iterator for InterfaceAddressIterator {
    type Item = (String, MacAddr);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            while !self.ifap.is_null() {
                // advance the pointer until a MAC address is found
                let ifap = self.ifap;
                self.ifap = (*ifap).ifa_next;

                if let Some(addr) = parse_interface_address(ifap) {
                    // libc::IFNAMSIZ + 6
                    // This size refers to ./apple/network.rs:75
                    let mut name = vec![0u8; libc::IFNAMSIZ + 6];
                    libc::strcpy(name.as_mut_ptr() as _, (*ifap).ifa_name);
                    name.set_len(libc::strlen((*ifap).ifa_name));
                    let name = String::from_utf8_unchecked(name);

                    return Some((name, addr));
                }
            }
            None
        }
    }
}

impl Drop for InterfaceAddressIterator {
    fn drop(&mut self) {
        unsafe {
            libc::freeifaddrs(self.buf);
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "ios"))]
impl From<&libc::sockaddr_dl> for MacAddr {
    fn from(value: &libc::sockaddr_dl) -> Self {
        let sdl_data = value.sdl_data;
        // interface name length, NO trailing 0
        let sdl_nlen = value.sdl_nlen as usize;
        // make sure that it is never out of bound
        if sdl_nlen + 5 < 12 {
            MacAddr([
                sdl_data[sdl_nlen] as u8,
                sdl_data[sdl_nlen + 1] as u8,
                sdl_data[sdl_nlen + 2] as u8,
                sdl_data[sdl_nlen + 3] as u8,
                sdl_data[sdl_nlen + 4] as u8,
                sdl_data[sdl_nlen + 5] as u8,
            ])
        } else {
            MacAddr::UNSPECIFIED
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "ios"))]
unsafe fn parse_interface_address(ifap: *const libc::ifaddrs) -> Option<MacAddr> {
    let sock_addr = (*ifap).ifa_addr;
    if sock_addr.is_null() {
        return None;
    }
    match (*sock_addr).sa_family as libc::c_int {
        libc::AF_LINK => {
            let addr = sock_addr as *const libc::sockaddr_dl;
            Some(MacAddr::from(&*addr))
        }
        _ => None,
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
unsafe fn parse_interface_address(ifap: *const libc::ifaddrs) -> Option<MacAddr> {
    use libc::sockaddr_ll;

    let sock_addr = (*ifap).ifa_addr;
    if sock_addr.is_null() {
        return None;
    }
    match (*sock_addr).sa_family as libc::c_int {
        libc::AF_PACKET => {
            let addr = sock_addr as *const sockaddr_ll;
            // Take the first 6 bytes
            let [addr @ .., _, _] = (*addr).sll_addr;
            Some(MacAddr(addr))
        }
        _ => None,
    }
}

/// Return an iterator on (interface_name, address) pairs
pub(crate) fn get_interface_address() -> Result<InterfaceAddressIterator, String> {
    let mut ifap = null_mut();
    unsafe {
        if retry_eintr!(libc::getifaddrs(&mut ifap)) == 0 && !ifap.is_null() {
            Ok(InterfaceAddressIterator { ifap, buf: ifap })
        } else {
            Err("failed to call getifaddrs()".to_string())
        }
    }
}
