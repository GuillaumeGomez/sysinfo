// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use libc::{c_int, c_char, c_void, gid_t, uid_t, c_uint, size_t};

extern "C" {
    pub fn kill(pid: c_int, signal: c_int) -> c_int;

    pub fn proc_pidinfo(pid: c_int, flavor: c_int, arg: u64, buffer: *mut c_void,
                        buffersize: c_int) -> c_int;
    pub fn proc_listallpids(buffer: *mut c_void, buffersize: c_int) -> c_int;
    //pub fn proc_name(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;
    //pub fn proc_regionfilename(pid: c_int, address: u64, buffer: *mut c_void,
    //                           buffersize: u32) -> c_int;
    //pub fn proc_pidpath(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;
    pub fn sysctl(name: *mut c_int, namelen: c_uint, oldp: *mut c_void, oldlenp: *mut size_t,
                  newp: *mut c_void, newlen: size_t) -> c_int;

    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    pub fn strcmp(s1: *const c_char, s2: *const c_char) -> i32;
    pub fn sprintf(s: *mut c_char, c: *const c_char, ...) -> i32;

    pub fn IOMasterPort(a: i32, b: *mut mach_port_t) -> i32;
    pub fn IOServiceMatching(a: *const c_char) -> *mut c_void;
    pub fn IOServiceGetMatchingServices(a: mach_port_t, b: *mut c_void, c: *mut io_iterator_t) -> i32;
    pub fn IOIteratorNext(iterator: io_iterator_t) -> io_object_t;
    pub fn IOObjectRelease(obj: io_object_t) -> i32;
    pub fn IOServiceOpen(device: io_object_t, a: u32, t: u32, x: *mut io_connect_t) -> i32;
    pub fn IOServiceClose(a: io_connect_t) -> i32;
    pub fn IOConnectCallStructMethod(connection: mach_port_t, 
                                     selector: u32, 
                                     inputStruct: *mut KeyData_t, 
                                     inputStructCnt: size_t, 
                                     outputStruct: *mut KeyData_t, 
                                     outputStructCnt: *mut size_t) -> i32;

    pub fn mach_absolute_time() -> u64;
    //pub fn task_for_pid(host: u32, pid: pid_t, task: *mut task_t) -> u32;
    pub fn mach_task_self() -> u32;
    pub fn mach_host_self() -> u32;
    //pub fn task_info(host_info: u32, t: u32, c: *mut c_void, x: *mut u32) -> u32;
    pub fn host_statistics64(host_info: u32, x: u32, y: *mut c_void, z: *const u32) -> u32;
    pub fn host_processor_info(host_info: u32, t: u32, num_cpu_u: *mut u32,
                               cpu_info: *mut *mut i32, num_cpu_info: *mut u32) -> u32;
    //pub fn host_statistics(host_priv: u32, flavor: u32, host_info: *mut c_void,
    //                       host_count: *const u32) -> u32;
    pub fn vm_deallocate(target_task: u32, address: *mut i32, size: u32) -> u32;
}

pub const CTL_KERN: c_int = 1;
pub const KERN_ARGMAX: c_int = 8;
//pub const KERN_PROC: c_int = 14;
//pub const KERN_PROCARGS: c_int = 38;
pub const KERN_PROCARGS2: c_int = 49;

pub const PROC_PIDTASKALLINFO: i32 = 2;
pub const PROC_PIDTASKINFO: i32 = 4;
pub const PROC_PIDTHREADINFO: i32 = 5;

pub const MAXCOMLEN: usize = 16; // MAXCOMLEN;
//pub const MAXPATHLEN: usize = 4 * 1024;
//pub const PROC_PIDPATHINFO_MAXSIZE: usize = 4 * MAXPATHLEN;
const MAXTHREADNAMESIZE: usize = 64; // MAXTHREADNAMESIZE

#[repr(C)]
pub struct proc_taskinfo {
    pub pti_virtual_size: u64, /* virtual memory size (bytes) */
    pub pti_resident_size: u64, /* resident memory size (bytes) */
    pub pti_total_user: u64, /* total time */
    pub pti_total_system: u64,
    pub pti_threads_user: u64, /* existing threads only */
    pub pti_threads_system: u64,
    pub pti_policy: i32, /* default policy for new threads */
    pub pti_faults: i32, /* number of page faults */
    pub pti_pageins: i32, /* number of actual pageins */
    pub pti_cow_faults: i32, /* number of copy-on-write faults */
    pub pti_messages_sent: i32, /* number of messages sent */
    pub pti_messages_received: i32, /* number of messages received */
    pub pti_syscalls_mach: i32, /* number of mach system calls */
    pub pti_syscalls_unix: i32, /* number of unix system calls */
    pub pti_csw: i32, /* number of context switches */
    pub pti_threadnum: i32, /* number of threads in the task */
    pub pti_numrunning: i32, /* number of running threads */
    pub pti_priority: i32, /* task priority */
}

#[repr(C)]
pub struct proc_bsdinfo {
    pub pbi_flags: u32,
    pub pbi_status: u32,
    pub pbi_xstatus: u32,
    pub pbi_pid: u32,
    pub pbi_ppid: u32,
    pub pbi_uid: uid_t,
    pub pbi_gid: gid_t,
    pub pbi_ruid: uid_t,
    pub pbi_rgid: gid_t,
    pub pbi_svuid: uid_t,
    pub pbi_svgid: gid_t,
    pub rfu_1: u32,
    pub pbi_comm: [u8; MAXCOMLEN],
    pub pbi_name: [u8; 2 * MAXCOMLEN],
    pub pbi_nfiles: u32,
    pub pbi_pgid: u32,
    pub pbi_pjobc: u32,
    pub e_tdev: u32,
    pub e_tpgid: u32,
    pub pbi_nice: i32,
    pub pbi_start_tvsec: u64,
    pub pbi_start_tvusec: u64,
}

#[repr(C)]
pub struct proc_taskallinfo {
    pub pbsd: proc_bsdinfo,
    pub ptinfo: proc_taskinfo,
}

#[repr(C)]
pub struct proc_threadinfo {
    pub pth_user_time: u64, /* user run time */
    pub pth_system_time: u64, /* system run time */
    pub pth_cpu_usage: i32, /* scaled cpu usage percentage */
    pub pth_policy: i32, /* scheduling policy in effect */
    pub pth_run_state: i32, /* run state (see below) */
    pub pth_flags: i32, /* various flags (see below) */
    pub pth_sleep_time: i32, /* number of seconds that thread */
    pub pth_curpri: i32, /* cur priority */
    pub pth_priority: i32, /*  priority */
    pub pth_maxpriority: i32, /* max priority */
    pub pth_name: [u8; MAXTHREADNAMESIZE], /* thread name, if any */
}

#[allow(non_camel_case_types)]
pub type policy_t = i32;
#[allow(non_camel_case_types)]
pub type integer_t = i32;
#[allow(non_camel_case_types)]
pub type time_t = i64;
#[allow(non_camel_case_types)]
pub type suseconds_t = i32;
#[allow(non_camel_case_types)]
pub type mach_vm_size_t = u64;
#[allow(non_camel_case_types)]
pub type task_t = u32;
#[allow(non_camel_case_types)]
pub type pid_t = i32;
#[allow(non_camel_case_types)]
pub type natural_t = u32;
#[allow(non_camel_case_types)]
pub type mach_port_t = u32;
#[allow(non_camel_case_types)]
pub type io_object_t = mach_port_t;
#[allow(non_camel_case_types)]
pub type io_iterator_t = io_object_t;
#[allow(non_camel_case_types)]
pub type io_connect_t = io_object_t;
#[allow(non_camel_case_types)]
pub type boolean_t = c_uint;

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

#[repr(C)]
pub struct Val_t {
    pub key: [i8; 5],
    pub data_size: u32,
    pub data_type: [i8; 5], // UInt32Char_t
    pub bytes: [i8; 32], // SMCBytes_t
}

#[repr(C)]
pub struct KeyData_vers_t {
    pub major: u8,
    pub minor: u8,
    pub build: u8,
    pub reserved: [u8; 1],
    pub release: u16,
}

#[repr(C)]
pub struct KeyData_pLimitData_t {
    pub version: u16,
    pub length: u16,
    pub cpu_plimit: u32,
    pub gpu_plimit: u32,
    pub mem_plimit: u32,
}

#[repr(C)]
pub struct KeyData_keyInfo_t {
    pub data_size: u32,
    pub data_type: u32,
    pub data_attributes: u8,
}

#[repr(C)]
pub struct KeyData_t {
    pub key: u32,
    pub vers: KeyData_vers_t,
    pub p_limit_data: KeyData_pLimitData_t,
    pub key_info: KeyData_keyInfo_t,
    pub result: u8,
    pub status: u8,
    pub data8: u8,
    pub data32: u32,
    pub bytes: [i8; 32], // SMCBytes_t
}

#[repr(C)]
pub struct xsw_usage {
    pub xsu_total: u64,
    pub xsu_avail: u64,
    pub xsu_used: u64,
    pub xsu_pagesize: u32,
    pub xsu_encrypted: boolean_t,
}

//pub const HOST_CPU_LOAD_INFO_COUNT: usize = 4;
//pub const HOST_CPU_LOAD_INFO: u32 = 3;
pub const KERN_SUCCESS: u32 = 0;

pub const HW_NCPU: u32 = 3;
pub const CTL_HW: u32 = 6;
pub const CTL_VM: u32 = 2;
pub const VM_SWAPUSAGE: u32 = 5;
pub const PROCESSOR_CPU_LOAD_INFO: u32 = 2;
pub const CPU_STATE_USER: u32 = 0;
pub const CPU_STATE_SYSTEM: u32 = 1;
pub const CPU_STATE_IDLE: u32 = 2;
pub const CPU_STATE_NICE: u32 = 3;
pub const CPU_STATE_MAX: usize = 4;
pub const HW_MEMSIZE: u32 = 24;

//pub const TASK_THREAD_TIMES_INFO: u32 = 3;
//pub const TASK_THREAD_TIMES_INFO_COUNT: u32 = 4;
//pub const TASK_BASIC_INFO_64: u32 = 5;
//pub const TASK_BASIC_INFO_64_COUNT: u32 = 10;
pub const HOST_VM_INFO64: u32 = 4;
pub const HOST_VM_INFO64_COUNT: u32 = 38;

pub const MACH_PORT_NULL: i32 = 0;
pub const KERNEL_INDEX_SMC: i32 = 2;
pub const SMC_CMD_READ_KEYINFO: u8 = 9;
pub const SMC_CMD_READ_BYTES: u8 = 5;

pub const KIO_RETURN_SUCCESS: i32 = 0;
