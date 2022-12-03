// Take a look at the license at the top of the repository in the LICENSE file.

#[allow(unused)]
use crate::common::{InterfaceAddress, MacAddr};
use std::net::Ipv4Addr;
use std::ptr::null_mut;

/// this iterator yields an interface name and address
pub(crate) struct InterfaceAddressIterator {
    /// Pointer to the current ifaddrs struct
    ifap: *mut libc::ifaddrs,
    /// Pointer to the first element in linked list
    buf: *mut libc::ifaddrs,
}

impl Iterator for InterfaceAddressIterator {
    type Item = (String, InterfaceAddress);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let ifap = self.ifap;
            if !ifap.is_null() {
                // don't forget to move on next pointer
                self.ifap = (*ifap).ifa_next;
                // libc::IFNAMSIZ + 6
                // This size refers to ./apple/network.rs:75
                let mut name = vec![0u8; libc::IFNAMSIZ + 6];
                libc::strcpy(name.as_mut_ptr() as _, (*ifap).ifa_name);
                name.set_len(libc::strlen((*ifap).ifa_name));
                let name = String::from_utf8_unchecked(name);

                Some((name, parse_interface_address(ifap)))
            } else {
                None
            }
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
            MacAddr::from([
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

unsafe fn get_raw_ipv4(sock_addr: *const libc::sockaddr) -> u32 {
    let sock_addr = sock_addr as *const libc::sockaddr_in;
    (*sock_addr).sin_addr.s_addr
}

unsafe fn get_ipv4_interface_address(ifap: *const libc::ifaddrs) -> InterfaceAddress {
    let sock_addr = (*ifap).ifa_addr;
    let address = get_raw_ipv4(sock_addr);
    let netmask = get_raw_ipv4((*ifap).ifa_netmask);

    InterfaceAddress::IPv4(
        Ipv4Addr::from(address.to_be()),
        Ipv4Addr::from(netmask.to_be()),
        // Ipv4Addr::from((address & netmask | (!netmask)).to_be())
    )
}

#[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "ios"))]
unsafe fn parse_interface_address(ifap: *const libc::ifaddrs) -> InterfaceAddress {
    let sock_addr = (*ifap).ifa_addr;
    match (*sock_addr).sa_family as libc::c_int {
        libc::AF_LINK => {
            let addr = sock_addr as *const libc::sockaddr_dl;
            InterfaceAddress::MAC(MacAddr::from(&*addr))
        }
        libc::AF_INET => get_ipv4_interface_address(ifap),
        _ => InterfaceAddress::NotImplemented,
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
unsafe fn parse_interface_address(ifap: *const libc::ifaddrs) -> InterfaceAddress {
    use libc::sockaddr_ll;

    let sock_addr = (*ifap).ifa_addr;
    match (*sock_addr).sa_family as libc::c_int {
        libc::AF_PACKET => {
            let addr = sock_addr as *const sockaddr_ll;
            // Take the first 6 bytes
            let [addr @ .., _, _] = (*addr).sll_addr;
            let addr = MacAddr::from(addr);
            InterfaceAddress::MAC(addr)
        }
        libc::AF_INET => get_ipv4_interface_address(ifap),
        _ => InterfaceAddress::NotImplemented,
    }
}

/// Return an iterator on (interface_name, address) pairs
pub(crate) fn get_interface_address() -> Result<InterfaceAddressIterator, String> {
    let mut ifap = null_mut();
    unsafe {
        if libc::getifaddrs(&mut ifap) == 0 && !ifap.is_null() {
            Ok(InterfaceAddressIterator { ifap, buf: ifap })
        } else {
            Err("failed to call getifaddrs()".to_string())
        }
    }
}