// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::OsString;
use std::net::Ipv4Addr;
use std::os::windows::prelude::OsStringExt;
use std::ptr::null_mut;

use winapi::shared::netioapi::ConvertLengthToIpv4Mask;
use winapi::shared::ntdef::ULONG;
use winapi::shared::winerror::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
use winapi::shared::ws2def::{AF_INET, AF_UNSPEC, SOCKADDR_IN};
use winapi::um::iphlpapi::GetAdaptersAddresses;
use winapi::um::iptypes::{
    GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
    PIP_ADAPTER_ADDRESSES, PIP_ADAPTER_UNICAST_ADDRESS,
};
use winapi::um::winsock2::htonl;

use crate::common::{InterfaceAddress, MacAddr};

/// this iterator yields an interface name and address
pub(crate) struct InterfaceAddressIterator {
    /// The first item in the linked list
    buf: PIP_ADAPTER_ADDRESSES,
    /// The current adapter
    adapter: PIP_ADAPTER_ADDRESSES,
    /// IP addresses grouped by current adapter
    unicast_address: PIP_ADAPTER_UNICAST_ADDRESS,
}

// Need a function to convert u16 pointer into String
// https://stackoverflow.com/a/48587463/8706476
unsafe fn u16_ptr_to_string(ptr: *const u16) -> OsString {
    let len = (0..).take_while(|&i| *ptr.offset(i) != 0).count();
    let slice = std::slice::from_raw_parts(ptr, len);

    OsString::from_wide(slice)
}

impl Iterator for InterfaceAddressIterator {
    type Item = (String, InterfaceAddress);

    fn next(&mut self) -> Option<Self::Item> {
        if self.adapter.is_null() {
            return None;
        }
        unsafe {
            let adapter = self.adapter;
            if let Ok(interface_name) = u16_ptr_to_string((*adapter).FriendlyName).into_string() {
                if self.unicast_address.is_null() {
                    // if we found that this interface has not been visited yet,
                    // set unicast address,
                    self.unicast_address = (*adapter).FirstUnicastAddress;
                    // take the first 6 bytes and return the MAC address instead
                    let [mac @ .., _, _] = (*adapter).PhysicalAddress;
                    Some((interface_name, InterfaceAddress::MAC(MacAddr::from(mac))))
                } else {
                    // otherwise, generate an IP address
                    let address = self.unicast_address;
                    self.unicast_address = (*address).Next;
                    if self.unicast_address.is_null() {
                        // if we have visited all unicast addresses, move to next adapter
                        self.adapter = (*adapter).Next;
                    }
                    let sock_addr = (*address).Address.lpSockaddr;
                    if sock_addr.is_null() {
                        // if sock_addr is null, move to next iteration
                        self.next()
                    } else {
                        match (*sock_addr).sa_family as _ {
                            AF_INET => {
                                let sock_addr = sock_addr as *const SOCKADDR_IN;
                                let sock_addr = (*sock_addr).sin_addr.S_un.S_addr();
                                let mut subnet_mask = 0 as ULONG;
                                // only available on vista and later
                                // https://learn.microsoft.com/zh-cn/windows/win32/api/netioapi/nf-netioapi-convertlengthtoipv4mask
                                ConvertLengthToIpv4Mask(
                                    (*address).OnLinkPrefixLength as _,
                                    &mut subnet_mask as _,
                                );

                                Some((
                                    interface_name,
                                    InterfaceAddress::IPv4(
                                        Ipv4Addr::from(htonl(*sock_addr)),
                                        Ipv4Addr::from(htonl(subnet_mask)),
                                    ),
                                ))
                            }
                            _ => Some((interface_name, InterfaceAddress::NotImplemented)),
                        }
                    }
                }
            } else {
                // Not sure whether error can occur when parsing adapter name.
                // If we met an error, move to the next adapter
                self.adapter = (*adapter).Next;
                self.next()
            }
        }
    }
}

impl Drop for InterfaceAddressIterator {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.buf as _);
        }
    }
}

pub(crate) fn get_interface_address() -> Result<InterfaceAddressIterator, String> {
    unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses#remarks
        // A 15k buffer is recommended
        let mut size: u32 = 15 * 1024;
        let mut buf = null_mut();
        let mut ret = ERROR_SUCCESS;

        // https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses#examples
        // Try to retreive adapter information up to 3 times
        for _ in 0..3 {
            buf = libc::malloc(size as usize) as PIP_ADAPTER_ADDRESSES;
            if buf.is_null() {
                // insufficient memory available
                // https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/malloc?view=msvc-170#return-value
                // malloc is not documented to set the last-error code
                return Err("failed to allocate memory for IP_ADAPTER_ADDRESSES".to_string());
            }

            ret = GetAdaptersAddresses(
                AF_UNSPEC as u32,
                GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_DNS_SERVER,
                null_mut(),
                buf,
                &mut size,
            );

            if ret == ERROR_BUFFER_OVERFLOW {
                // if the given memory size is too small to hold the adapter information,
                // the SizePointer returned will point to the required size of the buffer,
                // and we should continue.
                libc::free(buf as _);
                buf = null_mut();
            } else {
                // Otherwise, break the loop and check the return code again
                break;
            }
        }

        if ret == ERROR_SUCCESS && !buf.is_null() {
            Ok(InterfaceAddressIterator {
                buf,
                adapter: buf,
                unicast_address: null_mut(),
            })
        } else {
            Err(format!("GetAdaptersAddresses() failed with code {}", ret))
        }
    }
}
