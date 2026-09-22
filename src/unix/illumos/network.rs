// Take a look at the license at the top of the repository in the LICENSE file.

use std::collections::{HashMap, hash_map};
use std::ffi::{CStr, CString, OsString};
use std::os::unix::ffi::OsStringExt;

use crate::network::refresh_networks_addresses;
use crate::{Error, InterfaceOperationalState, MacAddr, NetworkData};

use super::NetworkDataInner;
use super::kstat::{KstatReader, KstatRecord};

const LINK_MODULE: &CStr = c"link";
const LOOPBACK_MODULE: &CStr = c"lo";
const UNIX_MODULE: &CStr = c"unix";
const LIFNAMSIZ: usize = 32;
// _IOWR('i', 122, struct lifreq) and _IOWR('i', 192, struct lifreq).
const SIOCGLIFMTU: libc::c_int = 0xc078_697a_u32 as libc::c_int;
const SIOCGLIFHWADDR: libc::c_int = 0xc078_69c0_u32 as libc::c_int;

#[repr(C)]
struct LifReq {
    name: [libc::c_char; LIFNAMSIZ],
    address_length: libc::c_int,
    interface_type: libc::c_uint,
    data: [u8; 336],
}

const _: [(); 376] = [(); std::mem::size_of::<LifReq>()];

impl LifReq {
    fn new(name: &[u8]) -> Self {
        let mut request = Self {
            name: [0; LIFNAMSIZ],
            address_length: 0,
            interface_type: 0,
            data: [0; 336],
        };
        for (output, input) in request.name[..LIFNAMSIZ - 1].iter_mut().zip(name) {
            *output = *input as libc::c_char;
        }
        request
    }
}

pub(crate) struct NetworksInner {
    pub(crate) interfaces: HashMap<OsString, NetworkData>,
}

impl NetworksInner {
    pub(crate) fn new() -> Result<Self, Error> {
        Ok(Self {
            interfaces: HashMap::new(),
        })
    }

    pub(crate) fn list(&self) -> &HashMap<OsString, NetworkData> {
        &self.interfaces
    }

    pub(crate) fn refresh(&mut self, remove_not_listed_interfaces: bool) {
        for interface in self.interfaces.values_mut() {
            interface.inner.updated = false;
        }
        let Some(kstat) = KstatReader::new() else {
            return;
        };

        let mut addresses = std::ptr::null_mut();
        if unsafe { libc::getifaddrs(&mut addresses) } != 0 || addresses.is_null() {
            return;
        }
        let original = addresses;
        while !addresses.is_null() {
            let address = unsafe { &*addresses };
            addresses = address.ifa_next;
            if address.ifa_name.is_null() {
                continue;
            }
            let bytes = unsafe { CStr::from_ptr(address.ifa_name) }.to_bytes();
            let bytes = bytes.split(|byte| *byte == b':').next().unwrap_or_default();
            if bytes.is_empty() {
                continue;
            }
            let name = OsString::from_vec(bytes.to_vec());
            if self
                .interfaces
                .get(&name)
                .is_some_and(|interface| interface.inner.updated)
            {
                continue;
            }

            let Ok(c_name) = CString::new(bytes) else {
                continue;
            };
            let record = [LINK_MODULE, LOOPBACK_MODULE, UNIX_MODULE]
                .into_iter()
                .find_map(|module| kstat.lookup(Some(module), -1, Some(&c_name)));
            let flags = address.ifa_flags as libc::c_int;
            let state = if flags & libc::IFF_UP != 0 && flags & libc::IFF_RUNNING != 0 {
                InterfaceOperationalState::Up
            } else {
                InterfaceOperationalState::Down
            };
            let counters = record.as_ref().map(read_counters).unwrap_or_default();
            let mtu = read_mtu(bytes);
            let mac_addr = read_mac_addr(bytes);

            match self.interfaces.entry(name) {
                hash_map::Entry::Occupied(mut entry) => {
                    let inner = &mut entry.get_mut().inner;
                    inner.old_ifi_ibytes = inner.ifi_ibytes;
                    inner.old_ifi_obytes = inner.ifi_obytes;
                    inner.old_ifi_ipackets = inner.ifi_ipackets;
                    inner.old_ifi_opackets = inner.ifi_opackets;
                    inner.old_ifi_ierrors = inner.ifi_ierrors;
                    inner.old_ifi_oerrors = inner.ifi_oerrors;
                    inner.ifi_ibytes = counters.received;
                    inner.ifi_obytes = counters.transmitted;
                    inner.ifi_ipackets = counters.packets_received;
                    inner.ifi_opackets = counters.packets_transmitted;
                    inner.ifi_ierrors = counters.errors_received;
                    inner.ifi_oerrors = counters.errors_transmitted;
                    inner.operational_state = state;
                    inner.mtu = mtu;
                    inner.mac_addr = mac_addr;
                    inner.updated = true;
                }
                hash_map::Entry::Vacant(entry) => {
                    entry.insert(NetworkData {
                        inner: NetworkDataInner {
                            ifi_ibytes: counters.received,
                            old_ifi_ibytes: 0,
                            ifi_obytes: counters.transmitted,
                            old_ifi_obytes: 0,
                            ifi_ipackets: counters.packets_received,
                            old_ifi_ipackets: 0,
                            ifi_opackets: counters.packets_transmitted,
                            old_ifi_opackets: 0,
                            ifi_ierrors: counters.errors_received,
                            old_ifi_ierrors: 0,
                            ifi_oerrors: counters.errors_transmitted,
                            old_ifi_oerrors: 0,
                            updated: true,
                            mac_addr,
                            ip_networks: Vec::new(),
                            mtu,
                            operational_state: state,
                        },
                    });
                }
            }
        }
        unsafe { libc::freeifaddrs(original) };

        if remove_not_listed_interfaces {
            self.interfaces
                .retain(|_, interface| interface.inner.updated);
        }
        refresh_networks_addresses(&mut self.interfaces);
    }
}

fn read_mtu(name: &[u8]) -> u64 {
    let mut request = LifReq::new(name);
    let socket = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if socket < 0 {
        return 0;
    }
    let result = unsafe { libc::ioctl(socket, SIOCGLIFMTU, &mut request) };
    unsafe {
        libc::close(socket);
    }
    if result == 0 {
        u32::from_ne_bytes(request.data[..4].try_into().unwrap()) as u64
    } else {
        0
    }
}

fn read_mac_addr(name: &[u8]) -> MacAddr {
    let mut request = LifReq::new(name);
    let socket = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if socket < 0 {
        return MacAddr::UNSPECIFIED;
    }
    let result = unsafe { libc::ioctl(socket, SIOCGLIFHWADDR, &mut request) };
    unsafe {
        libc::close(socket);
    }
    if result != 0 {
        return MacAddr::UNSPECIFIED;
    }

    let name_length = request.data[5] as usize;
    let address_length = request.data[6] as usize;
    let start = 8usize.saturating_add(name_length);
    let Some(address) = request
        .data
        .get(start..start.saturating_add(address_length))
    else {
        return MacAddr::UNSPECIFIED;
    };
    let Ok(address) = <[u8; 6]>::try_from(address) else {
        return MacAddr::UNSPECIFIED;
    };
    MacAddr(address)
}

#[derive(Default)]
struct Counters {
    received: u64,
    transmitted: u64,
    packets_received: u64,
    packets_transmitted: u64,
    errors_received: u64,
    errors_transmitted: u64,
}

fn read_counters(record: &KstatRecord<'_>) -> Counters {
    Counters {
        received: record
            .integer("rbytes64")
            .or_else(|| record.integer("rbytes"))
            .unwrap_or(0),
        transmitted: record
            .integer("obytes64")
            .or_else(|| record.integer("obytes"))
            .unwrap_or(0),
        packets_received: record
            .integer("ipackets64")
            .or_else(|| record.integer("ipackets"))
            .unwrap_or(0),
        packets_transmitted: record
            .integer("opackets64")
            .or_else(|| record.integer("opackets"))
            .unwrap_or(0),
        errors_received: record.integer("ierrors").unwrap_or(0),
        errors_transmitted: record.integer("oerrors").unwrap_or(0),
    }
}
