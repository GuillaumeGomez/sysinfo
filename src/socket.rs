///
use std::{fmt, str::FromStr, ptr::null_mut, net::Ipv4Addr};

/// MAC address for network interface
#[derive(PartialEq, Eq, Copy, Clone)]
pub struct MacAddress {
    data: [u8; 6],
}

impl MacAddress {
    pub const UNSPECIFIED: Self = MacAddress::new();

    #[inline]
    pub(crate) const fn new() -> Self {
        Self { data: [0; 6] }
    }

    pub fn data(&self) -> &[u8; 6] {
        &self.data
    }
}

impl From<[u8; 6]> for MacAddress {
    fn from(data: [u8; 6]) -> Self {
        Self { data }
    }
}

impl FromStr for MacAddress {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s
            .split(":")
            .filter_map(|s| u8::from_str_radix(s, 16).ok())
            .collect::<Vec<u8>>();
        if bytes.len() == 6 {
            let mut data = [0; 6];
            for (index, byte) in bytes.iter().enumerate() {
                data[index] = *byte;
            }
            return Ok(MacAddress { data });
        }
        Err("invalid MAC address syntax".to_string())
    }
}

impl std::fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self
            .data
            .iter()
            .map(|b| format!("{:02x}", *b))
            .collect::<Vec<String>>()
            .join(":");
        f.write_str(&s)
    }
}
pub(crate) enum IFAddress {
    MAC(MacAddress),
    // IPv4 address and subnet mask
    IPv4(Ipv4Addr, Ipv4Addr),
}

pub(crate) struct IFAddressIter {
    ifap: *mut libc::ifaddrs,
}


impl Iterator for IFAddressIter {
    // this iterator yields an interface name and the Result containing IFAddress
    type Item = (String, Result<IFAddress, String>);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let ifap = self.ifap;
            if !ifap.is_null() {
                self.ifap = (*ifap).ifa_next;
                // libc::IFNAMSIZ + 6 -> ./apple/network.rs:75
                let mut name = vec![0u8; libc::IFNAMSIZ + 6];
                let ifa_name = libc::strcpy(name.as_mut_ptr() as _, (*ifap).ifa_name);
                if ifa_name.is_null() {
                    return Some((
                        String::new(),
                        Err("failed to parse interface name".to_string()),
                    ));
                }
                name.set_len(libc::strlen(ifa_name));
                let name = String::from_utf8_unchecked(name);

                Some((name, parse_interface_address(ifap)))
            } else {
                None
            }
        }
    }
}

impl Drop for IFAddressIter {
    fn drop(&mut self) {
        unsafe {
            libc::freeifaddrs(self.ifap);
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
impl std::convert::TryFrom<&libc::sockaddr_dl> for MacAddress {
    type Error = String;

    fn try_from(value: &libc::sockaddr_dl) -> Result<Self, Self::Error> {
        let sdl_data = value.sdl_data;
        // interface name length, NO trailing 0
        let sdl_nlen = value.sdl_nlen as usize;
        // make sure that it is never out of bound
        if sdl_nlen + 5 < 12 {
            Ok(MacAddress::from([
                sdl_data[sdl_nlen] as u8,
                sdl_data[sdl_nlen + 1] as u8,
                sdl_data[sdl_nlen + 2] as u8,
                sdl_data[sdl_nlen + 3] as u8,
                sdl_data[sdl_nlen + 4] as u8,
                sdl_data[sdl_nlen + 5] as u8,
            ]))
        } else {
            Err("invalid sockaddr_dl".to_string())
        }
    }
}


unsafe fn get_raw_ipv4(sock_addr: *const libc::sockaddr) -> u32 {
    let sock_addr = sock_addr as *const libc::sockaddr_in;
    (*sock_addr).sin_addr.s_addr
}

unsafe fn get_ipv4_interface_address(ifap: *const libc::ifaddrs) -> IFAddress {
    let sock_addr = (*ifap).ifa_addr;
    let address = get_raw_ipv4(sock_addr);
    let netmask = get_raw_ipv4((*ifap).ifa_netmask);
    
    IFAddress::IPv4(
        Ipv4Addr::from(address.to_be()), 
        Ipv4Addr::from(netmask.to_be()), 
        // Ipv4Addr::from((address & netmask | (!netmask)).to_be())
    )
}


#[cfg(any(target_os = "macos", target_os = "freebsd"))]
unsafe fn parse_interface_address(ifap: *const libc::ifaddrs) -> Result<IFAddress, String> {
    use std::convert::TryFrom;

    let sock_addr = (*ifap).ifa_addr;
    match (*sock_addr).sa_family as libc::c_int {
        libc::AF_LINK => {
            let addr = sock_addr as *const libc::sockaddr_dl;
            let addr = MacAddress::try_from(&*addr)?;
            Ok(IFAddress::MAC(addr))
        },
        libc::AF_INET => {
            Ok(get_ipv4_interface_address(ifap))
        }
        _ => { Err("not implemented".to_string()) }
    }
}


#[cfg(any(target_os = "linux"))]
unsafe fn parse_interface_address(ifap: *const libc::ifaddrs) -> Result<IFAddress, String> {
    use libc::sockaddr_ll;
    let sock_addr = (*ifap).ifa_addr;
    match (*sock_addr).sa_family as libc::c_int {
        libc::AF_PACKET => {
            let addr = sock_addr as *const sockaddr_ll;
            // Take the first 6 bytes
            let [ref addr @ .., _, __] = (*addr).sll_addr;
            let addr = MacAddress::from(addr.clone());
            Ok(IFAddress::MAC(addr))
        },
        libc::AF_INET => {
            Ok(get_ipv4_interface_address(ifap))
        }
        _ => { Err("not implemented".to_string()) }
    }
}


#[allow(unused)]
pub(crate) fn get_interface_address() -> Result<IFAddressIter, String> {
    let mut ifap = null_mut();
    unsafe {
        if libc::getifaddrs(&mut ifap) == 0 {
            return Ok(IFAddressIter { ifap });
        }
    }
    Err("failed to call getifaddrs".to_string())
}


#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use super::{MacAddress,IFAddress, get_interface_address};

    #[test]
    fn from_str_mac_address() {
        let mac = MacAddress::from_str("e5:5d:59:e9:6e:b5").unwrap();
        let mac = mac.data();
        assert_eq!(mac[0], 0xe5);
        assert_eq!(mac[1], 0x5d);
        assert_eq!(mac[2], 0x59);
        assert_eq!(mac[3], 0xe9);
        assert_eq!(mac[4], 0x6e);
        assert_eq!(mac[5], 0xb5);
    }

    #[test]
    fn test_get_interface_address() {
        if let Ok(iterator) = get_interface_address() {
            for (name, ifa) in iterator {
                if let Ok(ifa) = ifa {
                    match ifa {
                        IFAddress::MAC(mac_addr) => {
                            println!("name: {} - {}", name, mac_addr)
                        },
                        IFAddress::IPv4(addr, netmask, broadcast) => {
                            println!("name: {} - {} / {} / {}", name, addr, netmask, broadcast)
                        },
                        _ => {

                        }
                    }
                }
            }
        }
    }
}

