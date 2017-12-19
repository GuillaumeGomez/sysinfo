// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

use std::mem::{size_of, zeroed};
use std::fmt::{self, Formatter, Debug};
//use std::os::raw;
use std::str;
//use std::env;

use libc::{c_uint, c_void, memcpy};

use winapi::shared::minwindef::{DWORD, FALSE, FILETIME/*, TRUE, USHORT*/};
use winapi::um::handleapi::CloseHandle;
use winapi::um::winnt::{
    DELETE, HANDLE, ULARGE_INTEGER, /*THREAD_GET_CONTEXT, THREAD_QUERY_INFORMATION, THREAD_SUSPEND_RESUME,*/
    /*, PWSTR*/
};
use winapi::um::processthreadsapi::{GetProcessTimes, OpenProcess, TerminateProcess};
use winapi::um::psapi::{
    K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
};
use winapi::um::sysinfoapi::GetSystemTimeAsFileTime;

/// Enum describing the different status of a process.
#[derive(Clone, Debug)]
pub enum ProcessStatus {
    /// Process being created by fork.
    Idle,
    /// Currently runnable.
    Run,
    /// Sleeping on an address.
    Sleep,
    /// Process debugging or suspension.
    Stop,
    /// Awaiting collection by parent.
    Zombie,
    /// Unknown.
    Unknown(u32),
}

impl From<u32> for ProcessStatus {
    fn from(status: u32) -> ProcessStatus {
        match status {
            1 => ProcessStatus::Idle,
            2 => ProcessStatus::Run,
            3 => ProcessStatus::Sleep,
            4 => ProcessStatus::Stop,
            5 => ProcessStatus::Zombie,
            x => ProcessStatus::Unknown(x),
        }
    }
}

impl ProcessStatus {
    /// Used to display `ProcessStatus`.
    pub fn to_string(&self) -> &str {
        match *self {
            ProcessStatus::Idle       => "Idle",
            ProcessStatus::Run        => "Runnable",
            ProcessStatus::Sleep      => "Sleeping",
            ProcessStatus::Stop       => "Stopped",
            ProcessStatus::Zombie     => "Zombie",
            ProcessStatus::Unknown(_) => "Unknown",
        }
    }
}

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Struct containing a process' information.
#[derive(Clone)]
pub struct Process {
    /// name of the program
    pub name: String,
    /// command line
    pub cmd: String,
    /// path to the executable
    pub exe: String,
    /// pid of the processus
    pub pid: u32,
    /// Environment of the process.
    ///
    /// Always empty except for current process.
    pub environ: Vec<String>,
    /// current working directory
    pub cwd: String,
    /// path of the root directory
    pub root: String,
    /// memory usage (in kB)
    pub memory: u64,
    handle: HANDLE,
    old_cpu: u64,
    old_sys_cpu: u64,
    old_user_cpu: u64,
    /// time of process launch (in seconds)
    pub start_time: u64,
    updated: bool,
    /// total cpu usage
    pub cpu_usage: f32,
}

impl Process {
    /// Create a new process only containing the given information.
    #[doc(hidden)]
    pub fn new(handle: HANDLE, pid: u32, start_time: u64, name: String) -> Process {
        //let mut env = Vec::new();
        Process {
            handle: handle,
            name: name.clone(),
            pid: pid,
            cmd: unsafe { get_cmd_line(handle) },
            environ: unsafe { get_proc_env(handle, pid, &name) },
            exe: String::new(),
            cwd: String::new(),
            root: String::new(),
            memory: 0,
            cpu_usage: 0.,
            old_cpu: 0,
            old_sys_cpu: 0,
            old_user_cpu: 0,
            updated: true,
            start_time: start_time,
        }
    }

    /// Sends the given `signal` to the process.
    pub fn kill(&self, signal: ::Signal) -> bool {
        unsafe {
            let handle = OpenProcess(DELETE, FALSE, self.pid);
            if handle.is_null() {
                false
            } else {
                let killed = TerminateProcess(handle, signal as c_uint) != 0;
                CloseHandle(handle);
                killed
            }
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            if self.handle.is_null() {
                return
            }
            CloseHandle(self.handle);
        }
    }
}

#[allow(unused_must_use)]
impl Debug for Process {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "pid: {}\n", self.pid);
        write!(f, "name: {}\n", self.name);
        write!(f, "environment:\n");
        for var in self.environ.iter() {
        if var.len() > 0 {
                write!(f, "\t{}\n", var);
            }
        }
        write!(f, "command: {}\n", self.cmd);
        write!(f, "executable path: {}\n", self.exe);
        write!(f, "current working directory: {}\n", self.cwd);
        write!(f, "memory usage: {} kB\n", self.memory);
        write!(f, "cpu usage: {}%\n", self.cpu_usage);
        write!(f, "root path: {}", self.root)
    }
}

unsafe fn get_cmd_line(_handle: HANDLE) -> String {
    /*let mut pinfo: ffi::PROCESS_BASIC_INFORMATION = ::std::mem::zeroed();
    if ffi::NtQueryInformationProcess(handle,
                                           0, // ProcessBasicInformation
                                           &mut pinfo,
                                           size_of::<ffi::PROCESS_BASIC_INFORMATION>(),
                                           ::std::ptr::null_mut()) <= 0x7FFFFFFF {
        return String::new();
    }
    let ppeb: ffi::PPEB = pinfo.PebBaseAddress;
    let mut ppeb_copy: ffi::PEB = ::std::mem::zeroed();
    if kernel32::ReadProcessMemory(handle,
                                   ppeb as *mut raw::c_void,
                                   &mut ppeb_copy as *mut ffi::PEB as *mut raw::c_void,
                                   size_of::<ffi::PPEB>() as SIZE_T,
                                   ::std::ptr::null_mut()) != TRUE {
        return String::new();
    }

    let proc_param: ffi::PRTL_USER_PROCESS_PARAMETERS = ppeb_copy.ProcessParameters;
    let rtl_proc_param_copy: ffi::RTL_USER_PROCESS_PARAMETERS = ::std::mem::zeroed();
    if kernel32::ReadProcessMemory(handle,
                                   proc_param as *mut ffi::PRTL_USER_PROCESS_PARAMETERS *mut raw::c_void,
                                   &mut rtl_proc_param_copy as *mut ffi::RTL_USER_PROCESS_PARAMETERS as *mut raw::c_void,
                                   size_of::<ffi::RTL_USER_PROCESS_PARAMETERS>() as SIZE_T,
                                   ::std::ptr::null_mut()) != TRUE {
        return String::new();
    }
    let len: usize = rtl_proc_param_copy.CommandLine.Length as usize;
    let mut buffer_copy: Vec<u8> = Vec::with_capacity(len);
    buffer_copy.set_len(len);
    if kernel32::ReadProcessMemory(handle,
                                   rtl_proc_param_copy.CommandLine.Buffer as *mut raw::c_void,
                                   buffer_copy.as_mut_ptr() as *mut raw::c_void,
                                   len as SIZE_T,
                                   ::std::ptr::null_mut()) == TRUE {
        println!("{:?}", str::from_utf8_unchecked(buffer_copy.as_slice()));
        str::from_utf8_unchecked(buffer_copy.as_slice()).to_owned()
    } else {
        String::new()
    }*/
    String::new()
}

unsafe fn get_proc_env(_handle: HANDLE, _pid: u32, _name: &str) -> Vec<String> {
    let ret = Vec::new();
    /*if name.starts_with("conhost.exe") {
        return ret;
    }
    println!("current pid: {}", kernel32::GetCurrentProcessId());
    if kernel32::GetCurrentProcessId() == pid {
        println!("current proc!");
        for (key, value) in env::vars() {
            ret.push(format!("{}={}", key, value));
        }
        return ret;
    }
    println!("1");
    let snapshot_handle = kernel32::CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
    if !snapshot_handle.is_null() {
        println!("2");
        let mut target_thread: THREADENTRY32 = zeroed();
        target_thread.dwSize = size_of::<THREADENTRY32>() as DWORD;
        if kernel32::Thread32First(snapshot_handle, &mut target_thread) == TRUE {
            println!("3");
            loop {
                if target_thread.th32OwnerProcessID == pid {
                    println!("4");
                    let thread_handle = kernel32::OpenThread(THREAD_SUSPEND_RESUME | THREAD_QUERY_INFORMATION | THREAD_GET_CONTEXT,
                                                             FALSE,
                                                             target_thread.th32ThreadID);
                    if !thread_handle.is_null() {
                        println!("5 -> {}", pid);
                        if kernel32::SuspendThread(thread_handle) != DWORD::max_value() {
                            println!("6");
                            let mut context = zeroed();
                            if kernel32::GetThreadContext(thread_handle, &mut context) != 0 {
                                println!("7 --> {:?}", context);
                                let mut x = vec![0u8; 10];
                                if kernel32::ReadProcessMemory(handle,
                                                               context.MxCsr as usize as *mut winapi::c_void,
                                                               x.as_mut_ptr() as *mut winapi::c_void,
                                                               x.len() as u64,
                                                               ::std::ptr::null_mut()) != 0 {
                                    for y in x {
                                        print!("{}", y as char);
                                    }
                                    println!("");
                                } else {
                                    println!("failure... {:?}", kernel32::GetLastError());
                                }
                            } else {
                                println!("-> {:?}", kernel32::GetLastError());
                            }
                            kernel32::ResumeThread(thread_handle);
                        }
                        kernel32::CloseHandle(thread_handle);
                    }
                    break;
                }
                if kernel32::Thread32Next(snapshot_handle, &mut target_thread) != TRUE {
                    break;
                }
            }
        }
        kernel32::CloseHandle(snapshot_handle);
    }*/
    ret
}

pub fn compute_cpu_usage(p: &mut Process, nb_processors: u64) {
    unsafe {
        let mut now: ULARGE_INTEGER = ::std::mem::zeroed();
        let mut sys: ULARGE_INTEGER = ::std::mem::zeroed();
        let mut user: ULARGE_INTEGER = ::std::mem::zeroed();
        let mut ftime: FILETIME = zeroed();
        let mut fsys: FILETIME = zeroed();
        let mut fuser: FILETIME = zeroed();

        GetSystemTimeAsFileTime(&mut ftime);
        memcpy(&mut now as *mut ULARGE_INTEGER as *mut c_void,
               &mut ftime as *mut FILETIME as *mut c_void,
               size_of::<FILETIME>());

        GetProcessTimes(p.handle,
                        &mut ftime as *mut FILETIME,
                        &mut ftime as *mut FILETIME,
                        &mut fsys as *mut FILETIME,
                        &mut fuser as *mut FILETIME);
        memcpy(&mut sys as *mut ULARGE_INTEGER as *mut c_void,
               &mut fsys as *mut FILETIME as *mut c_void,
               size_of::<FILETIME>());
        memcpy(&mut user as *mut ULARGE_INTEGER as *mut c_void,
               &mut fuser as *mut FILETIME as *mut c_void,
               size_of::<FILETIME>());
        p.cpu_usage = ((*sys.QuadPart() - p.old_sys_cpu) as f32 + (*user.QuadPart() - p.old_user_cpu) as f32)
            / (*now.QuadPart() - p.old_cpu) as f32 / nb_processors as f32 * 100.;
        p.old_cpu = *now.QuadPart();
        p.old_user_cpu = *user.QuadPart();
        p.old_sys_cpu = *sys.QuadPart();
    }
    p.updated = false;
}

// COMMON PART
//
// Need to be moved into a "common" file to avoid duplication.

pub fn has_been_updated(p: &Process) -> bool {
    p.updated
}

pub fn update_proc_info(p: &mut Process) {
    update_memory(p);
    p.updated = true;
}

pub fn update_memory(p: &mut Process) {
    unsafe {
        let mut pmc: PROCESS_MEMORY_COUNTERS_EX = zeroed();
        if K32GetProcessMemoryInfo(p.handle,
                                   &mut pmc as *mut PROCESS_MEMORY_COUNTERS_EX as *mut c_void as *mut PROCESS_MEMORY_COUNTERS,
                                   size_of::<PROCESS_MEMORY_COUNTERS_EX>() as DWORD) != 0 {
            p.memory = (pmc.PrivateUsage as u64) >> 10u64; // / 1024;
        }
    }
}
