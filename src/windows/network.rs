// Take a look at the license at the top of the repository in the LICENSE file.

use crate::common::{InterfaceAddress, MacAddr};
use crate::{NetworkExt, NetworksExt, NetworksIter};

use std::collections::{hash_map, HashMap};
use std::ffi::OsString;
use std::net::Ipv4Addr;
use std::os::windows::prelude::OsStringExt;
use std::ptr::null_mut;

use winapi::shared::ifdef::{MediaConnectStateDisconnected, NET_LUID};
use winapi::shared::minwindef::ULONG;
use winapi::shared::netioapi::{
    ConvertLengthToIpv4Mask, FreeMibTable, GetIfEntry2, GetIfTable2, MIB_IF_ROW2, PMIB_IF_TABLE2,
};
use winapi::shared::ntdef::NULL;
use winapi::shared::winerror::{ERROR_SUCCESS, NO_ERROR};
use winapi::shared::ws2def::{AF_INET, AF_UNSPEC, SOCKADDR_IN};
use winapi::um::iphlpapi::GetAdaptersAddresses;
use winapi::um::iptypes::{
    GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
    PIP_ADAPTER_ADDRESSES, PIP_ADAPTER_UNICAST_ADDRESS,
};
use winapi::um::winsock2::htonl;

macro_rules! old_and_new {
    ($ty_:expr, $name:ident, $old:ident, $new_val:expr) => {{
        $ty_.$old = $ty_.$name;
        $ty_.$name = $new_val;
    }};
}

#[doc = include_str!("../../md_doc/networks.md")]
pub struct Networks {
    interfaces: HashMap<String, NetworkData>,
}

impl Networks {
    pub(crate) fn new() -> Networks {
        Networks {
            interfaces: HashMap::new(),
        }
    }

    fn refresh_interfaces_addresses(&mut self) {
        if let Ok(iter) = get_interface_address() {
            for (name, ifa) in iter {
                if let Some(interface) = self.interfaces.get_mut(&name) {
                    match ifa {
                        InterfaceAddress::MAC(mac_addr) => {
                            interface.mac_addr = mac_addr;
                        }
                        InterfaceAddress::IPv4(addr, mask) => {
                            interface.ipv4_addr = addr;
                            interface.ipv4_mask = mask;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

impl NetworksExt for Networks {
    #[allow(clippy::needless_lifetimes)]
    fn iter<'a>(&'a self) -> NetworksIter<'a> {
        NetworksIter::new(self.interfaces.iter())
    }

    fn refresh_networks_list(&mut self) {
        let mut table: PMIB_IF_TABLE2 = std::ptr::null_mut();

        unsafe {
            if GetIfTable2(&mut table) != NO_ERROR {
                return;
            }

            for (_, data) in self.interfaces.iter_mut() {
                data.updated = false;
            }

            // In here, this is tricky: we have to filter out the software interfaces to only keep
            // the hardware ones. To do so, we first check the connection potential speed (if 0, not
            // interesting), then we check its state: if not open, not interesting either. And finally,
            // we count the members of a same group: if there is more than 1, then it's software level.
            let mut groups = HashMap::new();
            let mut indexes = Vec::new();
            let ptr = (*table).Table.as_ptr();
            for i in 0..(*table).NumEntries {
                let ptr = &*ptr.offset(i as _);
                if (ptr.TransmitLinkSpeed == 0 && ptr.ReceiveLinkSpeed == 0)
                    || ptr.MediaConnectState == MediaConnectStateDisconnected
                    || ptr.PhysicalAddressLength == 0
                {
                    continue;
                }
                let id = vec![
                    ptr.InterfaceGuid.Data2,
                    ptr.InterfaceGuid.Data3,
                    ptr.InterfaceGuid.Data4[0] as _,
                    ptr.InterfaceGuid.Data4[1] as _,
                    ptr.InterfaceGuid.Data4[2] as _,
                    ptr.InterfaceGuid.Data4[3] as _,
                    ptr.InterfaceGuid.Data4[4] as _,
                    ptr.InterfaceGuid.Data4[5] as _,
                    ptr.InterfaceGuid.Data4[6] as _,
                    ptr.InterfaceGuid.Data4[7] as _,
                ];
                let entry = groups.entry(id.clone()).or_insert(0);
                *entry += 1;
                if *entry > 1 {
                    continue;
                }
                indexes.push((i, id));
            }
            for (i, id) in indexes {
                let ptr = &*ptr.offset(i as _);
                if *groups.get(&id).unwrap_or(&0) > 1 {
                    continue;
                }
                let mut pos = 0;
                for x in ptr.Alias.iter() {
                    if *x == 0 {
                        break;
                    }
                    pos += 1;
                }
                let interface_name = match String::from_utf16(&ptr.Alias[..pos]) {
                    Ok(s) => s,
                    _ => continue,
                };
                match self.interfaces.entry(interface_name) {
                    hash_map::Entry::Occupied(mut e) => {
                        let mut interface = e.get_mut();
                        old_and_new!(interface, current_out, old_out, ptr.OutOctets);
                        old_and_new!(interface, current_in, old_in, ptr.InOctets);
                        old_and_new!(
                            interface,
                            packets_in,
                            old_packets_in,
                            ptr.InUcastPkts.saturating_add(ptr.InNUcastPkts)
                        );
                        old_and_new!(
                            interface,
                            packets_out,
                            old_packets_out,
                            ptr.OutUcastPkts.saturating_add(ptr.OutNUcastPkts)
                        );
                        old_and_new!(interface, errors_in, old_errors_in, ptr.InErrors);
                        old_and_new!(interface, errors_out, old_errors_out, ptr.OutErrors);
                        interface.updated = true;
                    }
                    hash_map::Entry::Vacant(e) => {
                        let packets_in = ptr.InUcastPkts.saturating_add(ptr.InNUcastPkts);
                        let packets_out = ptr.OutUcastPkts.saturating_add(ptr.OutNUcastPkts);

                        e.insert(NetworkData {
                            id: ptr.InterfaceLuid,
                            current_out: ptr.OutOctets,
                            old_out: ptr.OutOctets,
                            current_in: ptr.InOctets,
                            old_in: ptr.InOctets,
                            packets_in,
                            old_packets_in: packets_in,
                            packets_out,
                            old_packets_out: packets_out,
                            errors_in: ptr.InErrors,
                            old_errors_in: ptr.InErrors,
                            errors_out: ptr.OutErrors,
                            old_errors_out: ptr.OutErrors,
                            mac_addr: MacAddr::UNSPECIFIED,
                            ipv4_addr: Ipv4Addr::UNSPECIFIED,
                            ipv4_mask: Ipv4Addr::UNSPECIFIED,
                            updated: true,
                        });
                    }
                }
            }
            FreeMibTable(table as _);
        }
        // Remove interfaces which are gone.
        self.interfaces.retain(|_, d| d.updated);
        // Refresh all interfaces' addresses.
        self.refresh_interfaces_addresses();
    }

    fn refresh(&mut self) {
        let entry = std::mem::MaybeUninit::<MIB_IF_ROW2>::zeroed();

        unsafe {
            let mut entry = entry.assume_init();
            for (_, interface) in self.interfaces.iter_mut() {
                entry.InterfaceLuid = interface.id;
                entry.InterfaceIndex = 0; // to prevent the function to pick this one as index
                if GetIfEntry2(&mut entry) != NO_ERROR {
                    continue;
                }
                old_and_new!(interface, current_out, old_out, entry.OutOctets);
                old_and_new!(interface, current_in, old_in, entry.InOctets);
                old_and_new!(
                    interface,
                    packets_in,
                    old_packets_in,
                    entry.InUcastPkts.saturating_add(entry.InNUcastPkts)
                );
                old_and_new!(
                    interface,
                    packets_out,
                    old_packets_out,
                    entry.OutUcastPkts.saturating_add(entry.OutNUcastPkts)
                );
                old_and_new!(interface, errors_in, old_errors_in, entry.InErrors);
                old_and_new!(interface, errors_out, old_errors_out, entry.OutErrors);
            }
        }
    }
}

#[doc = include_str!("../../md_doc/network_data.md")]
pub struct NetworkData {
    id: NET_LUID,
    current_out: u64,
    old_out: u64,
    current_in: u64,
    old_in: u64,
    packets_in: u64,
    old_packets_in: u64,
    packets_out: u64,
    old_packets_out: u64,
    errors_in: u64,
    old_errors_in: u64,
    errors_out: u64,
    old_errors_out: u64,
    updated: bool,
    mac_addr: MacAddr,
    ipv4_addr: Ipv4Addr,
    ipv4_mask: Ipv4Addr,
}

impl NetworkExt for NetworkData {
    fn received(&self) -> u64 {
        self.current_in.saturating_sub(self.old_in)
    }

    fn total_received(&self) -> u64 {
        self.current_in
    }

    fn transmitted(&self) -> u64 {
        self.current_out.saturating_sub(self.old_out)
    }

    fn total_transmitted(&self) -> u64 {
        self.current_out
    }

    fn packets_received(&self) -> u64 {
        self.packets_in.saturating_sub(self.old_packets_in)
    }

    fn total_packets_received(&self) -> u64 {
        self.packets_in
    }

    fn packets_transmitted(&self) -> u64 {
        self.packets_out.saturating_sub(self.old_packets_out)
    }

    fn total_packets_transmitted(&self) -> u64 {
        self.packets_out
    }

    fn errors_on_received(&self) -> u64 {
        self.errors_in.saturating_sub(self.old_errors_in)
    }

    fn total_errors_on_received(&self) -> u64 {
        self.errors_in
    }

    fn errors_on_transmitted(&self) -> u64 {
        self.errors_out.saturating_sub(self.old_errors_out)
    }

    fn total_errors_on_transmitted(&self) -> u64 {
        self.errors_out
    }

    fn mac_address(&self) -> MacAddr {
        self.mac_addr
    }

    fn ipv4_address(&self) -> Ipv4Addr {
        self.ipv4_addr
    }

    fn ipv4_netmask(&self) -> Ipv4Addr {
        self.ipv4_mask
    }
}

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
                    // and return the MAC address instead
                    let [mac @ .., _, _] = (*adapter).PhysicalAddress;
                    Some((
                        interface_name,
                        InterfaceAddress::MAC(MacAddr::from(mac)),
                    ))
                } else {
                    // otherwise, generate an IP adddress
                    let address = self.unicast_address;
                    self.unicast_address = (*address).Next;
                    if self.unicast_address.is_null() {
                        // if we have visited all unicast addresses, move to next adapter
                        self.adapter = (*adapter).Next;
                    }
                    // FIXME: should we perform a null check on (*address).Address.lpSockaddr
                    let sock_addr = (*address).Address.lpSockaddr;

                    match (*sock_addr).sa_family as _ {
                        AF_INET => {
                            let sock_addr = sock_addr as *const SOCKADDR_IN;
                            let sock_addr = (*sock_addr).sin_addr.S_un.S_addr();
                            let mut subnet_mask = 0 as ULONG;
                            // only avaialbe on vista and later
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
            libc::malloc(self.buf as _);
        }
    }
}

fn get_interface_address() -> Result<InterfaceAddressIterator, String> {
    unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses#remarks
        // A 15k buffer is recommended
        let mut size: u32 = 16 * 1024;
        let buf = libc::malloc(size as usize) as PIP_ADAPTER_ADDRESSES;
        if buf.is_null() {
            // TODO: more details
            return Err("malloc failed".to_string());
        }

        let ret = GetAdaptersAddresses(
            AF_UNSPEC as u32,
            GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_DNS_SERVER,
            NULL,
            buf,
            &mut size,
        );

        if ret != ERROR_SUCCESS {
            return Err("GetAdaptersAddresses() failed".to_string());
        }

        Ok(InterfaceAddressIterator {
            buf,
            adapter: buf,
            unicast_address: null_mut(),
        })
    }
}
