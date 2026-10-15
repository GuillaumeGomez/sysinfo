// Take a look at the license at the top of the repository in the LICENSE file.

#![allow(non_camel_case_types, dead_code)]

use libc::{c_int, c_void};

#[cfg(feature = "pci")]
use libc::{c_char, c_ulong};

// definitions come from:
// https://github.com/freebsd/freebsd-src/blob/main/lib/libdevstat/devstat.h
// https://github.com/freebsd/freebsd-src/blob/main/sys/sys/devicestat.h

// #[repr(C)]
// pub(crate) struct statinfo {
//     pub(crate) cp_time: [c_long; CPUSTATES as usize],
//     pub(crate) tk_nin: c_long,
//     pub(crate) tk_nout: c_long,
//     pub(crate) dinfo: *mut devinfo,
//     pub(crate) snap_time: c_long_double,
// }

pub(crate) const DEVSTAT_READ: usize = 0x01;
pub(crate) const DEVSTAT_WRITE: usize = 0x02;

// pub(crate) const DSM_NONE: c_int = 0;
// pub(crate) const DSM_TOTAL_BYTES_READ: c_int = 2;
// pub(crate) const DSM_TOTAL_BYTES_WRITE: c_int = 3;

// extern "C" {
//     pub(crate) fn devstat_compute_statistics(current: *mut devstat, previous: *mut devstat, etime: c_long_double, ...) -> c_int;
// }

#[link(name = "geom")]
unsafe extern "C" {
    pub(crate) fn geom_stats_open() -> c_int;
    pub(crate) fn geom_stats_snapshot_get() -> *mut c_void;
    pub(crate) fn geom_stats_snapshot_next(arg: *mut c_void) -> *mut libc::devstat;
    pub(crate) fn geom_stats_snapshot_reset(arg: *mut c_void);
    pub(crate) fn geom_stats_snapshot_free(arg: *mut c_void);
}

//  https://github.com/freebsd/freebsd-src/blob/4a12e28f330c2fd002f31c6a70f88994fe902f8f/sys/sys/pciio.h
//  Note: FreeBSD 15.0.0 changed the struct `pci_conf` as well as `PCIOCGETCONF`, so main branch does not work.
#[cfg(feature = "pci")]
const PCI_MAXNAMELEN: usize = 16;

#[repr(C)]
#[cfg(feature = "pci")]
pub(crate) struct pci_conf_io {
    pub pat_buf_len: u32,
    pub num_patterns: u32,
    pub patterns: *mut c_void, //  struct pci_match_conf; not used
    pub match_buf_len: u32,
    pub num_matches: u32,
    pub matches: *mut pci_conf,
    pub offset: u32,
    pub generation: u32,
    pub status: c_int,
}

#[repr(C)]
#[cfg(feature = "pci")]
pub(crate) struct pci_conf {
    pub(crate) pc_sel: pcisel,
    pub(crate) pc_hdr: u8,
    pub(crate) pc_subvendor: u16,
    pub(crate) pc_subdevice: u16,
    pub(crate) pc_vendor: u16,
    pub(crate) pc_device: u16,
    pub(crate) pc_class: u8,
    pub(crate) pc_subclass: u8,
    pub(crate) pc_progif: u8,
    pub(crate) pc_revid: u8,
    pub(crate) pd_name: [c_char; PCI_MAXNAMELEN + 1],
    pub(crate) pd_unit: c_ulong,
}

#[repr(C)]
#[cfg(feature = "pci")]
pub struct pcisel {
    pub(crate) pc_domain: u32,
    pub(crate) pc_bus: u8,
    pub(crate) pc_dev: u8,
    pub(crate) pc_func: u8,
}
