//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use libc::{c_int, c_void, kern_return_t};

// Reexport items defined in either macos or ios ffi module.
pub use crate::sys::inner::ffi::*;

extern "C" {
    // FIXME: https://github.com/rust-lang/libc/pull/2401
    pub fn vm_deallocate(target_task: u32, address: *mut i32, size: u32) -> kern_return_t;

    #[cfg(not(feature = "apple-sandbox"))]
    #[allow(deprecated)]
    // FIXME: https://github.com/rust-lang/libc/pull/2401
    pub static vm_page_size: libc::vm_size_t;
}

#[repr(C)]
pub struct __DADisk(c_void);
#[repr(C)]
pub struct __DASession(c_void);

// #[allow(non_camel_case_types)]
// pub type io_name_t = [u8; 128];
// #[allow(non_camel_case_types)]
// pub type io_registry_entry_t = io_object_t;

// pub type IOOptionBits = u32;

#[cfg_attr(feature = "debug", derive(Eq, Hash, PartialEq))]
#[derive(Clone)]
#[repr(C)]
pub struct Val_t {
    pub key: [i8; 5],
    pub data_size: u32,
    pub data_type: [i8; 5], // UInt32Char_t
    pub bytes: [i8; 32],    // SMCBytes_t
}

// FIXME: https://github.com/rust-lang/libc/pull/2401
#[derive(Debug, Default)]
#[repr(C)]
pub struct RUsageInfoV2 {
    pub ri_uuid: [u8; 16],
    pub ri_user_time: u64,
    pub ri_system_time: u64,
    pub ri_pkg_idle_wkups: u64,
    pub ri_interrupt_wkups: u64,
    pub ri_pageins: u64,
    pub ri_wired_size: u64,
    pub ri_resident_size: u64,
    pub ri_phys_footprint: u64,
    pub ri_proc_start_abstime: u64,
    pub ri_proc_exit_abstime: u64,
    pub ri_child_user_time: u64,
    pub ri_child_system_time: u64,
    pub ri_child_pkg_idle_wkups: u64,
    pub ri_child_interrupt_wkups: u64,
    pub ri_child_pageins: u64,
    pub ri_child_elapsed_abstime: u64,
    pub ri_diskio_bytesread: u64,
    pub ri_diskio_byteswritten: u64,
}

// FIXME: https://github.com/rust-lang/libc/pull/2401
pub const HOST_VM_INFO64_COUNT: u32 = 38;
// FIXME: https://github.com/rust-lang/libc/pull/2401
pub const RUSAGE_INFO_V2: c_int = 2;
