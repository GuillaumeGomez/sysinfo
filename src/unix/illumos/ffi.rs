// Take a look at the license at the top of the repository in the LICENSE file.

#[cfg(any(feature = "disk", feature = "network", feature = "system"))]
use libc::c_void;
use libc::{c_char, c_int};

#[cfg(feature = "system")]
pub(crate) const SC_LIST: c_int = 2;
#[cfg(feature = "system")]
pub(crate) const SC_GETNSWP: c_int = 4;
#[cfg(feature = "system")]
pub(crate) const ST_INDEL: c_int = 0x01;

#[repr(C)]
#[cfg(feature = "system")]
pub(crate) struct SwapEntry {
    pub(crate) path: *mut c_char,
    pub(crate) start: libc::off_t,
    pub(crate) length: libc::off_t,
    pub(crate) pages: libc::c_long,
    pub(crate) free: libc::c_long,
    pub(crate) flags: c_int,
}

#[repr(C)]
#[cfg(feature = "system")]
pub(crate) struct SwapTable {
    pub(crate) count: c_int,
    pub(crate) entries: [SwapEntry; 0],
}

#[cfg(feature = "system")]
const _: [(); 48] = [(); std::mem::size_of::<SwapEntry>()];
#[cfg(feature = "system")]
const _: [(); 8] = [(); std::mem::size_of::<SwapTable>()];

#[repr(C)]
#[cfg(any(feature = "disk", feature = "network", feature = "system"))]
pub(crate) struct KstatCtl {
    _private: [u8; 0],
}

#[repr(C)]
#[cfg(any(feature = "disk", feature = "network", feature = "system"))]
pub(crate) struct Kstat {
    _private: [u8; 0],
}

#[repr(C)]
#[cfg(any(feature = "disk", feature = "gpu"))]
pub(crate) struct DevInfoNode {
    _private: [u8; 0],
}

#[cfg(any(feature = "disk", feature = "gpu"))]
pub(crate) const DINFO_PROP: u32 = 0xdf04;

#[cfg(feature = "gpu")]
pub(crate) const DINFO_SUBTREE: u32 = 0xdf01;

#[cfg(any(feature = "disk", feature = "gpu"))]
pub(crate) const DDI_DEV_T_ANY: libc::dev_t = (-2_i64) as libc::dev_t;

#[repr(C)]
#[derive(Copy, Clone)]
#[cfg(any(feature = "network", feature = "system"))]
pub(crate) struct KstatString {
    pub(crate) ptr: *const c_char,
    pub(crate) len: u32,
    pub(crate) _padding: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
#[cfg(any(feature = "network", feature = "system"))]
pub(crate) union KstatValue {
    pub(crate) chars: [c_char; 16],
    pub(crate) i32_: i32,
    pub(crate) u32_: u32,
    pub(crate) i64_: i64,
    pub(crate) u64_: u64,
    pub(crate) string: KstatString,
}

#[repr(C)]
#[cfg(any(feature = "network", feature = "system"))]
pub(crate) struct KstatNamed {
    pub(crate) name: [c_char; 31],
    pub(crate) data_type: u8,
    pub(crate) value: KstatValue,
}

#[cfg(feature = "system")]
pub(crate) const KSTAT_DATA_CHAR: u8 = 0;
#[cfg(any(feature = "network", feature = "system"))]
pub(crate) const KSTAT_DATA_INT32: u8 = 1;
#[cfg(any(feature = "network", feature = "system"))]
pub(crate) const KSTAT_DATA_UINT32: u8 = 2;
#[cfg(any(feature = "network", feature = "system"))]
pub(crate) const KSTAT_DATA_INT64: u8 = 3;
#[cfg(any(feature = "network", feature = "system"))]
pub(crate) const KSTAT_DATA_UINT64: u8 = 4;
#[cfg(feature = "system")]
pub(crate) const KSTAT_DATA_STRING: u8 = 9;

#[repr(C)]
#[cfg(feature = "disk")]
pub(crate) struct KstatIo {
    pub(crate) nread: u64,
    pub(crate) nwritten: u64,
    pub(crate) reads: u32,
    pub(crate) writes: u32,
    pub(crate) wtime: i64,
    pub(crate) wlentime: i64,
    pub(crate) wlastupdate: i64,
    pub(crate) rtime: i64,
    pub(crate) rlentime: i64,
    pub(crate) rlastupdate: i64,
    pub(crate) wcnt: u32,
    pub(crate) rcnt: u32,
}

#[cfg(feature = "disk")]
const _: [(); 80] = [(); std::mem::size_of::<KstatIo>()];

#[cfg(any(feature = "disk", feature = "network", feature = "system"))]
#[link(name = "kstat")]
unsafe extern "C" {
    pub(crate) fn kstat_open() -> *mut KstatCtl;
    pub(crate) fn kstat_close(ctl: *mut KstatCtl) -> c_int;
    pub(crate) fn kstat_lookup(
        ctl: *mut KstatCtl,
        module: *const c_char,
        instance: c_int,
        name: *const c_char,
    ) -> *mut Kstat;
    pub(crate) fn kstat_read(ctl: *mut KstatCtl, stat: *mut Kstat, data: *mut c_void) -> c_int;
    #[cfg(any(feature = "network", feature = "system"))]
    pub(crate) fn kstat_data_lookup(stat: *mut Kstat, name: *const c_char) -> *mut c_void;
}

#[cfg(any(feature = "disk", feature = "gpu"))]
#[link(name = "devinfo")]
unsafe extern "C" {
    pub(crate) fn di_init(path: *const c_char, flags: u32) -> *mut DevInfoNode;
    pub(crate) fn di_fini(node: *mut DevInfoNode);
    #[cfg(feature = "gpu")]
    pub(crate) fn di_child_node(node: *mut DevInfoNode) -> *mut DevInfoNode;
    #[cfg(feature = "gpu")]
    pub(crate) fn di_sibling_node(node: *mut DevInfoNode) -> *mut DevInfoNode;
    #[cfg(feature = "disk")]
    pub(crate) fn di_driver_name(node: *mut DevInfoNode) -> *const c_char;
    #[cfg(feature = "disk")]
    pub(crate) fn di_instance(node: *mut DevInfoNode) -> c_int;
    pub(crate) fn di_prop_lookup_ints(
        dev: libc::dev_t,
        node: *mut DevInfoNode,
        name: *const c_char,
        data: *mut *mut c_int,
    ) -> c_int;
    #[cfg(feature = "gpu")]
    pub(crate) fn di_prop_lookup_strings(
        dev: libc::dev_t,
        node: *mut DevInfoNode,
        name: *const c_char,
        data: *mut *mut c_char,
    ) -> c_int;
}

unsafe extern "C" {
    #[cfg(feature = "system")]
    pub(crate) fn swapctl(command: c_int, argument: *mut c_void) -> c_int;
}
