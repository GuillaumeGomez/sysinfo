// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

use libc::{self, c_char, CTL_NET, NET_RT_IFLIST2, PF_ROUTE, RTM_IFINFO2};
use std::ptr::null_mut;
use sys::ffi;

use NetworkExt;

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
}

pub fn new() -> NetworkData {
    NetworkData {
        old_in: 0,
        old_out: 0,
        current_in: 0,
        current_out: 0,
    }
}

#[allow(clippy::cast_ptr_alignment)]
pub fn update_network(n: &mut NetworkData) {
    let mib = &mut [CTL_NET, PF_ROUTE, 0, 0, NET_RT_IFLIST2, 0];
    let mut len = 0;
    if unsafe { libc::sysctl(mib.as_mut_ptr(), 6, null_mut(), &mut len, null_mut(), 0) } < 0 {
        // TODO: might be nice to put an error in here...
        return
    }
    let mut buf = Vec::with_capacity(len);
    unsafe {
        buf.set_len(len);
        if libc::sysctl(mib.as_mut_ptr(), 6, buf.as_mut_ptr(), &mut len, null_mut(), 0) < 0 {
            // TODO: might be nice to put an error in here...
            return
        }
    }
    let buf = buf.as_ptr() as *const c_char;
    let lim = unsafe { buf.add(len) };
    let mut next = buf;
    let mut totalibytes = 0u64;
    let mut totalobytes = 0u64;
    while next < lim {
        unsafe {
            let ifm = next as *const libc::if_msghdr;
            next = next.offset((*ifm).ifm_msglen as isize);
            if (*ifm).ifm_type == RTM_IFINFO2 as u8 {
                let if2m: *const ffi::if_msghdr2 = ifm as *const ffi::if_msghdr2;
                totalibytes += (*if2m).ifm_data.ifi_ibytes;
                totalobytes += (*if2m).ifm_data.ifi_obytes;
            }
        }
    }
    n.old_in = n.current_in;
    n.current_in = totalibytes;
    n.old_out = n.current_out;
    n.current_out = totalobytes;
}
