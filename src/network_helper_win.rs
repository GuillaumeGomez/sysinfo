// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::OsString;
use std::os::windows::prelude::OsStringExt;
use std::ptr::null_mut;

use winapi::shared::winerror::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
use winapi::shared::ws2def::AF_UNSPEC;
use winapi::um::iphlpapi::GetAdaptersAddresses;
use winapi::um::iptypes::{
    GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST, PIP_ADAPTER_ADDRESSES,
};

use crate::common::MacAddr;

/// this iterator yields an interface name and address
pub(crate) struct InterfaceAddressIterator {
    /// The first item in the linked list
    buf: PIP_ADAPTER_ADDRESSES,
    /// The current adapter
    adapter: PIP_ADAPTER_ADDRESSES,
}

// Need a function to convert u16 pointer into String
// https://stackoverflow.com/a/48587463/8706476
unsafe fn u16_ptr_to_string(ptr: *const u16) -> OsString {
    let len = (0..).take_while(|&i| *ptr.offset(i) != 0).count();
    let slice = std::slice::from_raw_parts(ptr, len);

    OsString::from_wide(slice)
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
            if let Ok(interface_name) = u16_ptr_to_string((*adapter).FriendlyName).into_string() {
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
        let mut ret = ERROR_SUCCESS;

        // https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses#examples
        // Try to retrieve adapter information up to 3 times
        for _ in 0..3 {
            let buf = libc::malloc(size as _) as PIP_ADAPTER_ADDRESSES;
            // free memory on drop
            let iterator = InterfaceAddressIterator { buf, adapter: buf };
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
            if ret == ERROR_SUCCESS {
                return Ok(iterator);
            } else if ret != ERROR_BUFFER_OVERFLOW {
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
