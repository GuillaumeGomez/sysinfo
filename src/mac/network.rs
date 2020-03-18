//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use libc::{self, c_char, CTL_NET, NET_RT_IFLIST2, PF_ROUTE, RTM_IFINFO2};

use std::collections::HashMap;
use std::ptr::null_mut;
use sys::ffi;

use NetworkExt;
use NetworksExt;
use NetworksIter;

macro_rules! old_and_new {
    ($ty_:expr, $name:ident, $old:ident, $new_val:expr) => {{
        $ty_.$old = $ty_.$name;
        $ty_.$name = $new_val;
    }};
}

/// Network interfaces.
///
/// ```no_run
/// use sysinfo::{NetworksExt, System, SystemExt};
///
/// let s = System::new_all();
/// let networks = s.get_networks();
/// ```
pub struct Networks {
    interfaces: HashMap<String, NetworkData>,
}

impl Networks {
    pub(crate) fn new() -> Self {
        Networks {
            interfaces: HashMap::new(),
        }
    }

    #[allow(clippy::cast_ptr_alignment)]
    fn update_networks(&mut self) {
        let mib = &mut [CTL_NET, PF_ROUTE, 0, 0, NET_RT_IFLIST2, 0];
        let mut len = 0;
        if unsafe { libc::sysctl(mib.as_mut_ptr(), 6, null_mut(), &mut len, null_mut(), 0) } < 0 {
            // TODO: might be nice to put an error in here...
            return;
        }
        let mut buf = Vec::with_capacity(len);
        unsafe {
            buf.set_len(len);
            if libc::sysctl(
                mib.as_mut_ptr(),
                6,
                buf.as_mut_ptr(),
                &mut len,
                null_mut(),
                0,
            ) < 0
            {
                // TODO: might be nice to put an error in here...
                return;
            }
        }
        let buf = buf.as_ptr() as *const c_char;
        let lim = unsafe { buf.add(len) };
        let mut next = buf;
        while next < lim {
            unsafe {
                let ifm = next as *const libc::if_msghdr;
                next = next.offset((*ifm).ifm_msglen as isize);
                if (*ifm).ifm_type == RTM_IFINFO2 as u8 {
                    // The interface (line description) name stored at ifname will be returned in
                    // the default coded character set identifier (CCSID) currently in effect for
                    // the job. If this is not a single byte CCSID, then storage greater than
                    // IFNAMSIZ (16) bytes may be needed. 22 bytes is large enough for all CCSIDs.
                    let mut name = vec![0u8; libc::IFNAMSIZ + 6];

                    let if2m: *const ffi::if_msghdr2 = ifm as *const ffi::if_msghdr2;
                    let pname =
                        libc::if_indextoname((*if2m).ifm_index as _, name.as_mut_ptr() as _);
                    if pname.is_null() {
                        continue;
                    }
                    name.set_len(libc::strlen(pname));
                    let name = String::from_utf8_unchecked(name);
                    let interface = self.interfaces.entry(name).or_insert_with(|| NetworkData {
                        current_in: (*if2m).ifm_data.ifi_ibytes,
                        old_in: 0,
                        current_out: (*if2m).ifm_data.ifi_obytes,
                        old_out: 0,
                        packets_in: (*if2m).ifm_data.ifi_ipackets,
                        old_packets_in: 0,
                        packets_out: (*if2m).ifm_data.ifi_opackets,
                        old_packets_out: 0,
                        errors_in: (*if2m).ifm_data.ifi_ierrors,
                        old_errors_in: 0,
                        errors_out: (*if2m).ifm_data.ifi_oerrors,
                        old_errors_out: 0,
                        updated: true,
                    });
                    old_and_new!(interface, current_out, old_out, (*if2m).ifm_data.ifi_obytes);
                    old_and_new!(interface, current_in, old_in, (*if2m).ifm_data.ifi_ibytes);
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
            }
        }
    }
}

impl NetworksExt for Networks {
    fn iter<'a>(&'a self) -> NetworksIter<'a> {
        NetworksIter::new(self.interfaces.iter())
    }

    fn refresh_networks_list(&mut self) {
        for (_, data) in self.interfaces.iter_mut() {
            data.updated = false;
        }
        self.update_networks();
        self.interfaces.retain(|_, data| data.updated);
    }

    fn refresh(&mut self) {
        self.update_networks();
    }
}

/// Contains network information.
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
}

impl NetworkExt for NetworkData {
    fn get_received(&self) -> u64 {
        self.current_in - self.old_in
    }

    fn get_total_received(&self) -> u64 {
        self.current_in
    }

    fn get_transmitted(&self) -> u64 {
        self.current_out - self.old_out
    }

    fn get_total_transmitted(&self) -> u64 {
        self.current_out
    }

    fn get_packets_received(&self) -> u64 {
        self.packets_in - self.old_packets_in
    }

    fn get_total_packets_received(&self) -> u64 {
        self.packets_in
    }

    fn get_packets_transmitted(&self) -> u64 {
        self.packets_out - self.old_packets_out
    }

    fn get_total_packets_transmitted(&self) -> u64 {
        self.packets_out
    }

    fn get_errors_on_received(&self) -> u64 {
        self.errors_in - self.old_errors_in
    }

    fn get_total_errors_on_received(&self) -> u64 {
        self.errors_in
    }

    fn get_errors_on_transmitted(&self) -> u64 {
        self.errors_out - self.old_errors_out
    }

    fn get_total_errors_on_transmitted(&self) -> u64 {
        self.errors_out
    }
}
