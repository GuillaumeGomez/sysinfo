//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use libc::{
    c_int, c_uchar, c_ushort, c_void, mach_msg_type_number_t, natural_t, processor_flavor_t,
    processor_info_array_t,
};

// Reexport items defined in either macos or ios ffi module.
pub use crate::sys::inner::ffi::*;

extern "C" {
    //pub fn task_for_pid(host: u32, pid: pid_t, task: *mut task_t) -> u32;
    pub fn mach_task_self() -> u32;
    pub fn mach_host_self() -> u32;
    //pub fn task_info(host_info: u32, t: u32, c: *mut c_void, x: *mut u32) -> u32;
    pub fn host_statistics64(
        host_info: u32,
        x: u32,
        y: *mut c_void,
        z: *const u32,
    ) -> kern_return_t;
    pub fn host_processor_info(
        host: u32,
        flavor: processor_flavor_t,
        out_processor_count: *mut natural_t,
        out_processor_info: *mut processor_info_array_t,
        out_processor_infoCnt: *mut mach_msg_type_number_t,
    ) -> kern_return_t;
    //pub fn host_statistics(host_priv: u32, flavor: u32, host_info: *mut c_void,
    //                       host_count: *const u32) -> u32;
    // pub fn proc_pidpath(pid: i32, buf: *mut i8, bufsize: u32) -> i32;
    // pub fn proc_name(pid: i32, buf: *mut i8, bufsize: u32) -> i32;
    pub fn vm_deallocate(target_task: u32, address: *mut i32, size: u32) -> kern_return_t;

    #[allow(deprecated)]
    pub static vm_page_size: libc::vm_size_t;
}

// TODO: waiting for https://github.com/rust-lang/libc/pull/678
cfg_if::cfg_if! {
    if #[cfg(any(target_arch = "arm", target_arch = "x86"))] {
        pub type timeval32 = libc::timeval;
    } else {
        use libc::timeval32;
    }
}

// TODO: waiting for https://github.com/rust-lang/libc/pull/678
#[cfg_attr(feature = "debug", derive(Debug, Eq, Hash, PartialEq))]
#[repr(C)]
pub struct if_data64 {
    pub ifi_type: c_uchar,
    pub ifi_typelen: c_uchar,
    pub ifi_physical: c_uchar,
    pub ifi_addrlen: c_uchar,
    pub ifi_hdrlen: c_uchar,
    pub ifi_recvquota: c_uchar,
    pub ifi_xmitquota: c_uchar,
    pub ifi_unused1: c_uchar,
    pub ifi_mtu: u32,
    pub ifi_metric: u32,
    pub ifi_baudrate: u64,
    pub ifi_ipackets: u64,
    pub ifi_ierrors: u64,
    pub ifi_opackets: u64,
    pub ifi_oerrors: u64,
    pub ifi_collisions: u64,
    pub ifi_ibytes: u64,
    pub ifi_obytes: u64,
    pub ifi_imcasts: u64,
    pub ifi_omcasts: u64,
    pub ifi_iqdrops: u64,
    pub ifi_noproto: u64,
    pub ifi_recvtiming: u32,
    pub ifi_xmittiming: u32,
    pub ifi_lastchange: timeval32,
}

// TODO: waiting for https://github.com/rust-lang/libc/pull/678
#[cfg_attr(feature = "debug", derive(Debug, Eq, Hash, PartialEq))]
#[repr(C)]
pub struct if_msghdr2 {
    pub ifm_msglen: c_ushort,
    pub ifm_version: c_uchar,
    pub ifm_type: c_uchar,
    pub ifm_addrs: c_int,
    pub ifm_flags: c_int,
    pub ifm_index: c_ushort,
    pub ifm_snd_len: c_int,
    pub ifm_snd_maxlen: c_int,
    pub ifm_snd_drops: c_int,
    pub ifm_timer: c_int,
    pub ifm_data: if_data64,
}

#[repr(C)]
pub struct __DADisk(c_void);
#[repr(C)]
pub struct __DASession(c_void);

// #[allow(non_camel_case_types)]
// pub type io_name_t = [u8; 128];
// #[allow(non_camel_case_types)]
// pub type io_registry_entry_t = io_object_t;

//#[allow(non_camel_case_types)]
//pub type policy_t = i32;
#[allow(non_camel_case_types)]
//pub type integer_t = i32;
//#[allow(non_camel_case_types)]
//pub type time_t = i64;
//#[allow(non_camel_case_types)]
//pub type suseconds_t = i32;
//#[allow(non_camel_case_types)]
//pub type mach_vm_size_t = u64;
//#[allow(non_camel_case_types)]
//pub type task_t = u32;
//#[allow(non_camel_case_types)]
//pub type pid_t = i32;
#[allow(non_camel_case_types)]
pub type kern_return_t = c_int;
// pub type IOOptionBits = u32;

/*#[repr(C)]
pub struct task_thread_times_info {
    pub user_time: time_value,
    pub system_time: time_value,
}*/

/*#[repr(C)]
pub struct task_basic_info_64 {
    pub suspend_count: integer_t,
    pub virtual_size: mach_vm_size_t,
    pub resident_size: mach_vm_size_t,
    pub user_time: time_value_t,
    pub system_time: time_value_t,
    pub policy: policy_t,
}*/

#[cfg_attr(feature = "debug", derive(Debug, Eq, Hash, PartialEq))]
#[repr(C)]
pub struct vm_statistics64 {
    pub free_count: natural_t,
    pub active_count: natural_t,
    pub inactive_count: natural_t,
    pub wire_count: natural_t,
    pub zero_fill_count: u64,
    pub reactivations: u64,
    pub pageins: u64,
    pub pageouts: u64,
    pub faults: u64,
    pub cow_faults: u64,
    pub lookups: u64,
    pub hits: u64,
    pub purges: u64,
    pub purgeable_count: natural_t,
    pub speculative_count: natural_t,
    pub decompressions: u64,
    pub compressions: u64,
    pub swapins: u64,
    pub swapouts: u64,
    pub compressor_page_count: natural_t,
    pub throttled_count: natural_t,
    pub external_page_count: natural_t,
    pub internal_page_count: natural_t,
    pub total_uncompressed_pages_in_compressor: u64,
}

#[cfg_attr(feature = "debug", derive(Eq, Hash, PartialEq))]
#[derive(Clone)]
#[repr(C)]
pub struct Val_t {
    pub key: [i8; 5],
    pub data_size: u32,
    pub data_type: [i8; 5], // UInt32Char_t
    pub bytes: [i8; 32],    // SMCBytes_t
}

#[cfg_attr(feature = "debug", derive(Debug, Eq, Hash, PartialEq))]
#[repr(C)]
pub struct xsw_usage {
    pub xsu_total: u64,
    pub xsu_avail: u64,
    pub xsu_used: u64,
    pub xsu_pagesize: u32,
    pub xsu_encrypted: libc::boolean_t,
}

// https://github.com/andrewdavidmackenzie/libproc-rs/blob/master/src/libproc/pid_rusage.rs
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

//pub const HOST_CPU_LOAD_INFO_COUNT: usize = 4;
//pub const HOST_CPU_LOAD_INFO: u32 = 3;
pub const KERN_SUCCESS: kern_return_t = 0;

//pub const TASK_THREAD_TIMES_INFO: u32 = 3;
//pub const TASK_THREAD_TIMES_INFO_COUNT: u32 = 4;
//pub const TASK_BASIC_INFO_64: u32 = 5;
//pub const TASK_BASIC_INFO_64_COUNT: u32 = 10;
pub const HOST_VM_INFO64: u32 = 4;
pub const HOST_VM_INFO64_COUNT: u32 = 38;
