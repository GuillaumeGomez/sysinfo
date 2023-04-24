// Take a look at the license at the top of the repository in the LICENSE file.

use libc::{self, c_char, if_msghdr2, CTL_NET, NET_RT_IFLIST2, PF_ROUTE, RTM_IFINFO2};

use std::collections::{hash_map, HashMap};
use std::ptr::null_mut;

use crate::common::MacAddr;
use crate::network::refresh_networks_addresses;
use crate::{NetworkExt, NetworksExt, NetworksIter};

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
    pub(crate) fn new() -> Self {
        Networks {
            interfaces: HashMap::new(),
        }
    }

    #[allow(unknown_lints)]
    #[allow(clippy::cast_ptr_alignment)]
    #[allow(clippy::uninit_vec)]
    fn update_networks(&mut self) {
        let mib = &mut [CTL_NET, PF_ROUTE, 0, 0, NET_RT_IFLIST2, 0];
        let mut len = 0;
        unsafe {
            if libc::sysctl(
                mib.as_mut_ptr(),
                mib.len() as _,
                null_mut(),
                &mut len,
                null_mut(),
                0,
            ) < 0
            {
                // TODO: might be nice to put an error in here...
                return;
            }
            let mut buf = Vec::with_capacity(len);
            buf.set_len(len);
            if libc::sysctl(
                mib.as_mut_ptr(),
                mib.len() as _,
                buf.as_mut_ptr(),
                &mut len,
                null_mut(),
                0,
            ) < 0
            {
                // TODO: might be nice to put an error in here...
                return;
            }
            let buf = buf.as_ptr() as *const c_char;
            let lim = buf.add(len);
            let mut next = buf;
            while next < lim {
                let ifm = next as *const libc::if_msghdr;
                next = next.offset((*ifm).ifm_msglen as isize);
                if (*ifm).ifm_type == RTM_IFINFO2 as u8 {
                    // The interface (line description) name stored at ifname will be returned in
                    // the default coded character set identifier (CCSID) currently in effect for
                    // the job. If this is not a single byte CCSID, then storage greater than
                    // IFNAMSIZ (16) bytes may be needed. 22 bytes is large enough for all CCSIDs.
                    let mut name = vec![0u8; libc::IFNAMSIZ + 6];

                    let if2m: *const if_msghdr2 = ifm as *const if_msghdr2;
                    let pname =
                        libc::if_indextoname((*if2m).ifm_index as _, name.as_mut_ptr() as _);
                    if pname.is_null() {
                        continue;
                    }
                    name.set_len(libc::strlen(pname));
                    let name = String::from_utf8_unchecked(name);
                    match self.interfaces.entry(name) {
                        hash_map::Entry::Occupied(mut e) => {
                            let mut interface = e.get_mut();
                            old_and_new!(
                                interface,
                                current_out,
                                old_out,
                                (*if2m).ifm_data.ifi_obytes
                            );
                            old_and_new!(
                                interface,
                                current_in,
                                old_in,
                                (*if2m).ifm_data.ifi_ibytes
                            );
                            old_and_new!(
                                interface,
                                packets_in,
                                old_packets_in,
                                (*if2m).ifm_data.ifi_ipackets
                            );
                            old_and_new!(
                                interface,
                                packets_out,
                                old_packets_out,
                                (*if2m).ifm_data.ifi_opackets
                            );
                            old_and_new!(
                                interface,
                                errors_in,
                                old_errors_in,
                                (*if2m).ifm_data.ifi_ierrors
                            );
                            old_and_new!(
                                interface,
                                errors_out,
                                old_errors_out,
                                (*if2m).ifm_data.ifi_oerrors
                            );
                            interface.updated = true;
                        }
                        hash_map::Entry::Vacant(e) => {
                            let current_in = (*if2m).ifm_data.ifi_ibytes;
                            let current_out = (*if2m).ifm_data.ifi_obytes;
                            let packets_in = (*if2m).ifm_data.ifi_ipackets;
                            let packets_out = (*if2m).ifm_data.ifi_opackets;
                            let errors_in = (*if2m).ifm_data.ifi_ierrors;
                            let errors_out = (*if2m).ifm_data.ifi_oerrors;

                            e.insert(NetworkData {
                                current_in,
                                old_in: current_in,
                                current_out,
                                old_out: current_out,
                                packets_in,
                                old_packets_in: packets_in,
                                packets_out,
                                old_packets_out: packets_out,
                                errors_in,
                                old_errors_in: errors_in,
                                errors_out,
                                old_errors_out: errors_out,
                                updated: true,
                                mac_addr: MacAddr::UNSPECIFIED,
                            });
                        }
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
        for (_, data) in self.interfaces.iter_mut() {
            data.updated = false;
        }
        self.update_networks();
        self.interfaces.retain(|_, data| data.updated);
        refresh_networks_addresses(&mut self.interfaces);
    }

    fn refresh(&mut self) {
        self.update_networks();
    }
}

#[doc = include_str!("../../md_doc/network_data.md")]
#[derive(PartialEq, Eq)]
pub struct NetworkData {
    current_in: u64,
    old_in: u64,
    current_out: u64,
    old_out: u64,
    packets_in: u64,
    old_packets_in: u64,
    packets_out: u64,
    old_packets_out: u64,
    errors_in: u64,
    old_errors_in: u64,
    errors_out: u64,
    old_errors_out: u64,
    updated: bool,
    /// MAC address
    pub(crate) mac_addr: MacAddr,
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
}
