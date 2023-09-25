// Take a look at the license at the top of the repository in the LICENSE file.

use std::ptr::null_mut;

use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
use windows::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
    IP_ADAPTER_ADDRESSES_LH,
};
use windows::Win32::Networking::WinSock::AF_UNSPEC;

use crate::common::MacAddr;

/// this iterator yields an interface name and address
pub(crate) struct InterfaceAddressIterator {
    /// The first item in the linked list
    buf: *mut IP_ADAPTER_ADDRESSES_LH,
    /// The current adapter
    adapter: *mut IP_ADAPTER_ADDRESSES_LH,
}

impl InterfaceAddressIterator {
    fn new() -> Self {
        Self {
            buf: null_mut(),
            adapter: null_mut(),
        }
    }
    unsafe fn realloc(mut self, size: libc::size_t) -> Result<Self, String> {
        let new_buf = libc::realloc(self.buf as _, size) as *mut IP_ADAPTER_ADDRESSES_LH;
        if new_buf.is_null() {
            // insufficient memory available
            // https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/malloc?view=msvc-170#return-value
            // malloc is not documented to set the last-error code
            Err("failed to allocate memory for IP_ADAPTER_ADDRESSES".to_string())
        } else {
            self.buf = new_buf;
            self.adapter = new_buf;
            Ok(self)
        }
    }
}

impl Iterator for InterfaceAddressIterator {
    type Item = (String, MacAddr);

    fn next(&mut self) -> Option<Self::Item> {
        if self.adapter.is_null() {
            return None;
        }
        unsafe {
            let adapter = self.adapter;
            // Move to the next adapter
            self.adapter = (*adapter).Next;
            if let Ok(interface_name) = (*adapter).FriendlyName.to_string() {
                // take the first 6 bytes and return the MAC address instead
                let [mac @ .., _, _] = (*adapter).PhysicalAddress;
                Some((interface_name, MacAddr(mac)))
            } else {
                // Not sure whether error can occur when parsing adapter name.
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
        let mut ret = ERROR_SUCCESS.0;
        let mut iterator = InterfaceAddressIterator::new();

        // https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses#examples
        // Try to retrieve adapter information up to 3 times
        for _ in 0..3 {
            iterator = iterator.realloc(size as _)?;
            ret = GetAdaptersAddresses(
                AF_UNSPEC.0.into(),
                GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_DNS_SERVER,
                None,
                Some(iterator.buf),
                &mut size,
            );
            if ret == ERROR_SUCCESS.0 {
                return Ok(iterator);
            } else if ret != ERROR_BUFFER_OVERFLOW.0 {
                break;
            }
            // if the given memory size is too small to hold the adapter information,
            // the SizePointer returned will point to the required size of the buffer,
            // and we should continue.
            // Otherwise, break the loop and check the return code again
        }

        Err(format!("GetAdaptersAddresses() failed with code {ret}"))
    }
}
