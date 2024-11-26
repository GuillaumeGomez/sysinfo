// Take a look at the license at the top of the repository in the LICENSE file.

#![allow(non_camel_case_types)]

use libc::{bintime, c_char, c_int, c_uint, c_void, kvm_t};

// definitions come from:
// https://github.com/freebsd/freebsd-src/blob/main/lib/libdevstat/devstat.h
// https://github.com/freebsd/freebsd-src/blob/main/sys/sys/devicestat.h

pub type devstat_priority = c_int;
pub type devstat_support_flags = c_int;
pub type devstat_type_flags = c_int;

#[repr(C)]
pub(crate) struct tailq {
    pub(crate) stqe_next: *mut devstat,
}

#[repr(C)]
pub(crate) struct devstat {
    pub(crate) sequence0: c_uint,
    pub(crate) allocated: c_int,
    pub(crate) start_count: c_uint,
    pub(crate) end_count: c_uint,
    pub(crate) busy_from: bintime,
    pub(crate) dev_links: tailq,
    pub(crate) device_number: u32,
    pub(crate) device_name: [c_char; DEVSTAT_NAME_LEN],
    pub(crate) unit_number: c_int,
    pub(crate) bytes: [u64; DEVSTAT_N_TRANS_FLAGS],
    pub(crate) operations: [u64; DEVSTAT_N_TRANS_FLAGS],
    pub(crate) duration: [bintime; DEVSTAT_N_TRANS_FLAGS],
    pub(crate) busy_time: bintime,
    pub(crate) creation_time: bintime,
    pub(crate) block_size: u32,
    pub(crate) tag_types: [u64; 3],
    pub(crate) flags: devstat_support_flags,
    pub(crate) device_type: devstat_type_flags,
    pub(crate) priority: devstat_priority,
    pub(crate) id: *const c_void,
    pub(crate) sequence1: c_uint,
}

// #[repr(C)]
// pub(crate) struct devinfo {
//     pub(crate) devices: *mut devstat,
//     pub(crate) mem_ptr: *mut u8,
//     pub(crate) generation: c_long,
//     pub(crate) numdevs: c_int,
// }

// #[repr(C)]
// pub(crate) struct statinfo {
//     pub(crate) cp_time: [c_long; CPUSTATES as usize],
//     pub(crate) tk_nin: c_long,
//     pub(crate) tk_nout: c_long,
//     pub(crate) dinfo: *mut devinfo,
//     pub(crate) snap_time: c_long_double,
// }

pub(crate) const DEVSTAT_N_TRANS_FLAGS: usize = 4;
pub(crate) const DEVSTAT_NAME_LEN: usize = 16;
pub(crate) const DEVSTAT_READ: usize = 0x01;
pub(crate) const DEVSTAT_WRITE: usize = 0x02;

// pub(crate) const DSM_NONE: c_int = 0;
// pub(crate) const DSM_TOTAL_BYTES_READ: c_int = 2;
// pub(crate) const DSM_TOTAL_BYTES_WRITE: c_int = 3;

extern "C" {
    pub(crate) fn devstat_getversion(kd: *mut kvm_t) -> c_int;
    // pub(crate) fn devstat_getdevs(kd: *mut kvm_t, stats: *mut statinfo) -> c_int;
    // pub(crate) fn devstat_compute_statistics(current: *mut devstat, previous: *mut devstat, etime: c_long_double, ...) -> c_int;
}

#[link(name = "geom")]
extern "C" {
    pub(crate) fn geom_stats_open() -> c_int;
    pub(crate) fn geom_stats_snapshot_get() -> *mut c_void;
    pub(crate) fn geom_stats_snapshot_next(arg: *mut c_void) -> *mut devstat;
    pub(crate) fn geom_stats_snapshot_reset(arg: *mut c_void);
    pub(crate) fn geom_stats_snapshot_free(arg: *mut c_void);
}
