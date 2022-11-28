// Take a look at the license at the top of the repository in the LICENSE file.
use core::ffi;
use libc::sockaddr_dl;
use std::collections::{hash_map, HashMap};
use std::mem::MaybeUninit;
use std::ptr::null_mut;
use std::convert::TryFrom;

use super::utils;
use crate::common::MacAddress;
use crate::{NetworkExt, NetworksExt, NetworksIter};

macro_rules! old_and_new {
    ($ty_:expr, $name:ident, $old:ident, $data:expr) => {{
        $ty_.$old = $ty_.$name;
        $ty_.$name = $data.$name;
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
}

impl NetworksExt for Networks {
    fn iter(&self) -> NetworksIter {
        NetworksIter::new(self.interfaces.iter())
    }

    fn refresh_networks_list(&mut self) {
        unsafe {
            self.refresh_interfaces(true);
        }
        // Remove interfaces which are gone.
        self.interfaces.retain(|_, n| n.updated);
    }

    fn refresh(&mut self) {
        unsafe {
            self.refresh_interfaces(false);
        }
    }
}

pub fn get_interface_address() -> HashMap::<String, MacAddress> {
    let mut address = HashMap::<String, MacAddress>::new();
    unsafe {
        let mut ifap = null_mut();
        if libc::getifaddrs(&mut ifap) == 0 {
            while !ifap.is_null() {
                match (*((*ifap).ifa_addr)).sa_family as libc::c_int {
                    libc::AF_LINK => {
                        let addr = (*ifap).ifa_addr as *const sockaddr_dl;
                        if let Ok(addr) = MacAddress::try_from(&*addr) {
                            let name = String::from_utf8_lossy(ffi::CStr::from_ptr((*ifap).ifa_name).to_bytes()).to_string();
                            address.insert(name, addr);
                        }
                    },
                    _ => { }
                }
                ifap = (*ifap).ifa_next;
            }
            libc::freeifaddrs(ifap);
        }
    }
    address
}


impl Networks {
    unsafe fn refresh_interfaces(&mut self, refresh_all: bool) {
        let mut nb_interfaces: libc::c_int = 0;
        if !utils::get_sys_value(
            &[
                libc::CTL_NET,
                libc::PF_LINK,
                libc::NETLINK_GENERIC,
                libc::IFMIB_SYSTEM,
                libc::IFMIB_IFCOUNT,
            ],
            &mut nb_interfaces,
        ) {
            return;
        }
        if refresh_all {
            // We don't need to update this value if we're not updating all interfaces.
            for interface in self.interfaces.values_mut() {
                interface.updated = false;
            }
        }
        let mut addresses = get_interface_address();
        let mut data: libc::ifmibdata = MaybeUninit::zeroed().assume_init();
        for row in 1..nb_interfaces {
            let mib = [
                libc::CTL_NET,
                libc::PF_LINK,
                libc::NETLINK_GENERIC,
                libc::IFMIB_IFDATA,
                row,
                libc::IFDATA_GENERAL,
            ];

            if !utils::get_sys_value(&mib, &mut data) {
                continue;
            }
            if let Some(name) = utils::c_buf_to_string(&data.ifmd_name) {
                let data = &data.ifmd_data;
                let mac_addr = addresses.remove(&name).unwrap_or_else(|| MacAddress::new());
                match self.interfaces.entry(name) {
                    hash_map::Entry::Occupied(mut e) => {
                        let mut interface = e.get_mut();

                        old_and_new!(interface, ifi_ibytes, old_ifi_ibytes, data);
                        old_and_new!(interface, ifi_obytes, old_ifi_obytes, data);
                        old_and_new!(interface, ifi_ipackets, old_ifi_ipackets, data);
                        old_and_new!(interface, ifi_opackets, old_ifi_opackets, data);
                        old_and_new!(interface, ifi_ierrors, old_ifi_ierrors, data);
                        old_and_new!(interface, ifi_oerrors, old_ifi_oerrors, data);
                        interface.updated = true;
                        interface.mac_addr = mac_addr;
                    }
                    hash_map::Entry::Vacant(e) => {
                        if !refresh_all {
                            // This is simply a refresh, we don't want to add new interfaces!
                            continue;
                        }
                        e.insert(NetworkData {
                            ifi_ibytes: data.ifi_ibytes,
                            old_ifi_ibytes: 0,
                            ifi_obytes: data.ifi_obytes,
                            old_ifi_obytes: 0,
                            ifi_ipackets: data.ifi_ipackets,
                            old_ifi_ipackets: 0,
                            ifi_opackets: data.ifi_opackets,
                            old_ifi_opackets: 0,
                            ifi_ierrors: data.ifi_ierrors,
                            old_ifi_ierrors: 0,
                            ifi_oerrors: data.ifi_oerrors,
                            old_ifi_oerrors: 0,
                            updated: true,
                            mac_addr,
                        });
                    }
                }
            }
        }
    }
}

#[doc = include_str!("../../md_doc/network_data.md")]
pub struct NetworkData {
    /// Total number of bytes received over interface.
    ifi_ibytes: u64,
    old_ifi_ibytes: u64,
    /// Total number of bytes transmitted over interface.
    ifi_obytes: u64,
    old_ifi_obytes: u64,
    /// Total number of packets received.
    ifi_ipackets: u64,
    old_ifi_ipackets: u64,
    /// Total number of packets transmitted.
    ifi_opackets: u64,
    old_ifi_opackets: u64,
    /// Shows the total number of packets received with error. This includes
    /// too-long-frames errors, ring-buffer overflow errors, CRC errors,
    /// frame alignment errors, fifo overruns, and missed packets.
    ifi_ierrors: u64,
    old_ifi_ierrors: u64,
    /// similar to `ifi_ierrors`
    ifi_oerrors: u64,
    old_ifi_oerrors: u64,
    /// Whether or not the above data has been updated during refresh
    updated: bool,
    mac_addr: MacAddress,
}

impl NetworkExt for NetworkData {
    fn received(&self) -> u64 {
        self.ifi_ibytes.saturating_sub(self.old_ifi_ibytes)
    }

    fn total_received(&self) -> u64 {
        self.ifi_ibytes
    }

    fn transmitted(&self) -> u64 {
        self.ifi_obytes.saturating_sub(self.old_ifi_obytes)
    }

    fn total_transmitted(&self) -> u64 {
        self.ifi_obytes
    }

    fn packets_received(&self) -> u64 {
        self.ifi_ipackets.saturating_sub(self.old_ifi_ipackets)
    }

    fn total_packets_received(&self) -> u64 {
        self.ifi_ipackets
    }

    fn packets_transmitted(&self) -> u64 {
        self.ifi_opackets.saturating_sub(self.old_ifi_opackets)
    }

    fn total_packets_transmitted(&self) -> u64 {
        self.ifi_opackets
    }

    fn errors_on_received(&self) -> u64 {
        self.ifi_ierrors.saturating_sub(self.old_ifi_ierrors)
    }

    fn total_errors_on_received(&self) -> u64 {
        self.ifi_ierrors
    }

    fn errors_on_transmitted(&self) -> u64 {
        self.ifi_oerrors.saturating_sub(self.old_ifi_oerrors)
    }

    fn total_errors_on_transmitted(&self) -> u64 {
        self.ifi_oerrors
    }

    fn mac_address(&self) -> &MacAddress {
        &self.mac_addr
    }
}

impl TryFrom<&sockaddr_dl> for MacAddress {
    type Error = String;

    fn try_from(value: &sockaddr_dl) -> Result<Self, Self::Error> {
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
