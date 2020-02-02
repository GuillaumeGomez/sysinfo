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

/// Network interfaces.
///
/// ```no_run
/// use sysinfo::{NetworksExt, System, SystemExt};
///
/// let s = System::new();
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
}

impl NetworksExt for Networks {
    fn iter<'a>(&'a self) -> NetworksIter<'a> {
        NetworksIter::new(self.interfaces.iter())
    }

    #[allow(clippy::cast_ptr_alignment)]
    fn refresh_interfaces_list(&mut self) {
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
                    let ibytes = (*if2m).ifm_data.ifi_ibytes;
                    let obytes = (*if2m).ifm_data.ifi_obytes;
                    let interface = self.interfaces.entry(name).or_insert_with(|| NetworkData {
                        old_in: ibytes,
                        current_in: ibytes,
                        old_out: obytes,
                        current_out: obytes,
                    });
                    interface.old_in = interface.current_in;
                    interface.current_in = ibytes;
                    interface.old_out = interface.current_out;
                    interface.current_out = obytes;
                }
            }
        }
    }

    fn refresh(&mut self) {
        self.refresh_interfaces_list();
    }
}

/// Contains network information.
pub struct NetworkData {
    old_in: u64,
    old_out: u64,
    current_in: u64,
    current_out: u64,
}

impl NetworkExt for NetworkData {
    fn get_income(&self) -> u64 {
        self.current_in - self.old_in
    }

    fn get_outcome(&self) -> u64 {
        self.current_out - self.old_out
    }

    fn get_total_income(&self) -> u64 {
        self.current_in
    }

    fn get_total_outcome(&self) -> u64 {
        self.current_out
    }
}
