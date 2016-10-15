// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use libc::c_int;
#[cfg(target_os = "macos")]
use libc::{c_char, c_void, gid_t, uid_t, c_uint, size_t};

extern "C" {
    pub fn kill(pid: c_int, signal: c_int) -> c_int;
}

// -----------------
// -- MAC OS PART --
// -----------------

#[cfg(target_os = "macos")]
extern "C" {
    pub fn proc_pidinfo(pid: c_int, flavor: c_int, arg: u64, buffer: *mut c_void,
                        buffersize: c_int) -> c_int;
    pub fn proc_listallpids(buffer: *mut c_void, buffersize: c_int) -> c_int;
    pub fn proc_name(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;
    pub fn proc_regionfilename(pid: c_int, address: u64, buffer: *mut c_void,
                               buffersize: u32) -> c_int;
    pub fn proc_pidpath(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;
    pub fn sysctl(name: *mut c_int, namelen: c_uint, oldp: *mut c_void, oldlenp: *mut size_t,
                  newp: *mut c_void, newlen: size_t) -> c_int;
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;

    pub fn perror(c: *const c_char);
    pub fn getpid() -> i32;

    pub fn mach_absolute_time() -> u64;
    pub fn task_for_pid(host: u32, pid: pid_t, task: *mut task_t) -> u32;
    pub fn mach_task_self() -> u32;
    pub fn mach_host_self() -> u32;
    pub fn task_info(host_info: u32, t: u32, c: *mut c_void, x: *mut u32) -> u32;
    pub fn host_processor_info(host_info: u32, t: u32, num_cpu_u: *mut u32,
                               cpu_info: *mut *mut i32, num_cpu_info: *mut u32) -> u32;
    pub fn host_statistics(host_priv: u32, flavor: u32, host_info: *mut c_void,
                           host_count: *const u32) -> u32;
    pub fn vm_deallocate(target_task: u32, address: *mut i32, size: u32) -> u32;
}

#[cfg(target_os = "macos")]
pub const CTL_KERN: c_int = 1;
#[cfg(target_os = "macos")]
pub const KERN_ARGMAX: c_int = 8;
#[cfg(target_os = "macos")]
pub const KERN_PROC: c_int = 14;
#[cfg(target_os = "macos")]
pub const KERN_PROCARGS: c_int = 38;
#[cfg(target_os = "macos")]
pub const KERN_PROCARGS2: c_int = 49;

#[cfg(target_os = "macos")]
pub const PROC_PIDTASKALLINFO: i32 = 2;
#[cfg(target_os = "macos")]
pub const PROC_PIDTASKINFO: i32 = 4;
#[cfg(target_os = "macos")]
pub const PROC_PIDTHREADINFO: i32 = 5;

#[cfg(target_os = "macos")]
pub const MAXCOMLEN: usize = 16; // MAXCOMLEN;
#[cfg(target_os = "macos")]
pub const MAXPATHLEN: usize = 4 * 1024;
#[cfg(target_os = "macos")]
pub const PROC_PIDPATHINFO_MAXSIZE: usize = 4 * MAXPATHLEN;
#[cfg(target_os = "macos")]
const MAXTHREADNAMESIZE: usize = 64; // MAXTHREADNAMESIZE

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
#[repr(C)]
pub struct proc_bsdinfo {
    pub pbi_flags: u32,
    pub pbi_status: u32,
    pub pbi_xstatus: u32,
    pub pbi_pid: u32,
    pub ppbi_pid: u32,
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

#[cfg(target_os = "macos")]
#[repr(C)]
pub struct proc_taskallinfo {
    pub pbsd: proc_bsdinfo,
    pub ptinfo: proc_taskinfo,
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
pub type policy_t = i32;
#[cfg(target_os = "macos")]
pub type integer_t = i32;
#[cfg(target_os = "macos")]
pub type time_t = i64;
#[cfg(target_os = "macos")]
pub type suseconds_t = i32;
#[cfg(target_os = "macos")]
pub type mach_vm_size_t = u64;
#[cfg(target_os = "macos")]
pub type task_t = u32;
#[cfg(target_os = "macos")]
pub type pid_t = i32;
#[cfg(target_os = "macos")]
pub type time_value_t = time_value;

#[cfg(target_os = "macos")]
#[repr(C)]
pub struct timeval {
    pub tv_sec: time_t,
    pub tv_usec: suseconds_t,
}

#[cfg(target_os = "macos")]
impl timeval {
    pub fn to_microseconds(&self) -> u64 {
        let mut ret = self.tv_sec as u64;
        ret *= 1000000;
        ret + self.tv_usec as u64
    }
}

#[cfg(target_os = "macos")]
#[repr(C)]
pub struct time_value {
    pub seconds: integer_t,
    pub micro_seconds: integer_t,
}

#[cfg(target_os = "macos")]
impl time_value {
    pub fn to_timeval(&self) -> timeval {
        timeval {
            tv_sec: self.seconds as time_t,
            tv_usec: self.micro_seconds as suseconds_t,
        }
    }
}

#[cfg(target_os = "macos")]
#[repr(C)]
pub struct task_thread_times_info {
    pub user_time: time_value,
    pub system_time: time_value,
}

#[cfg(target_os = "macos")]
#[repr(C)]
pub struct task_basic_info_64 {
    pub suspend_count: integer_t,
    pub virtual_size: mach_vm_size_t,
    pub resident_size: mach_vm_size_t,
    pub user_time: time_value_t,
    pub system_time: time_value_t,
    pub policy: policy_t,
}

#[cfg(target_os = "macos")]
pub const HOST_CPU_LOAD_INFO_COUNT: usize = 4;
#[cfg(target_os = "macos")]
pub const HOST_CPU_LOAD_INFO: u32 = 3;
#[cfg(target_os = "macos")]
pub const KERN_SUCCESS: u32 = 0;

#[cfg(target_os = "macos")]
pub const HW_NCPU: u32 = 3;
#[cfg(target_os = "macos")]
pub const CTL_HW: u32 = 6;
#[cfg(target_os = "macos")]
pub const PROCESSOR_CPU_LOAD_INFO: u32 = 2;
#[cfg(target_os = "macos")]
pub const CPU_STATE_USER: u32 = 0;
#[cfg(target_os = "macos")]
pub const CPU_STATE_SYSTEM: u32 = 1;
#[cfg(target_os = "macos")]
pub const CPU_STATE_IDLE: u32 = 2;
#[cfg(target_os = "macos")]
pub const CPU_STATE_NICE: u32 = 3;
#[cfg(target_os = "macos")]
pub const CPU_STATE_MAX: usize = 4;

#[cfg(target_os = "macos")]
pub const TASK_THREAD_TIMES_INFO: u32 = 3;
#[cfg(target_os = "macos")]
pub const TASK_THREAD_TIMES_INFO_COUNT: u32 = 4;
#[cfg(target_os = "macos")]
pub const TASK_BASIC_INFO_64: u32 = 5;
#[cfg(target_os = "macos")]
pub const TASK_BASIC_INFO_64_COUNT: u32 = 10;

#[cfg(target_os = "macos")]
#[repr(C)]
pub struct host_cpu_load_info_data_t {
    pub cpu_ticks: [u64; CPU_STATE_MAX],
}
