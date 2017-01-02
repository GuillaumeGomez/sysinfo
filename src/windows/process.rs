// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

use std::mem::{size_of, zeroed};
use std::fmt::{self, Formatter, Debug};
use libc::{c_uint, c_void, memcpy};

use kernel32::{self, K32GetProcessMemoryInfo};
use winapi;
use winapi::winnt::{HANDLE, ULARGE_INTEGER};
use winapi::psapi::{PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX};
use winapi::minwindef::{DWORD, FILETIME};

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
    /// environment of the processus
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
    #[doc(hidden)]
    pub fn new(handle: HANDLE, pid: u32, start_time: u64) -> Process {
        Process {
            handle: handle,
            name: String::new(),
            pid: pid,
            cmd: String::new(),
            environ: Vec::new(),
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

    pub fn kill(&self, signal: ::Signal) -> bool {
        unsafe {
            let handle = kernel32::OpenProcess(winapi::winnt::DELETE, winapi::minwindef::FALSE, self.pid);
            if handle.is_null() {
                false
            } else {
                let killed = kernel32::TerminateProcess(handle, signal as c_uint) != 0;
                kernel32::CloseHandle(handle);
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
            kernel32::CloseHandle(self.handle);
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

pub fn compute_cpu_usage(p: &mut Process, nb_processors: u64) {
    let mut now: ULARGE_INTEGER = 0;
    let mut sys: ULARGE_INTEGER = 0;
    let mut user: ULARGE_INTEGER = 0;
    unsafe {
        let mut ftime: FILETIME = zeroed();
        let mut fsys: FILETIME = zeroed();
        let mut fuser: FILETIME = zeroed();

        kernel32::GetSystemTimeAsFileTime(&mut ftime);
        memcpy(&mut now as *mut ULARGE_INTEGER as *mut c_void,
               &mut ftime as *mut FILETIME as *mut c_void,
               size_of::<FILETIME>());

        kernel32::GetProcessTimes(p.handle,
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
    }
    p.cpu_usage = ((sys - p.old_sys_cpu) as f32 + (user - p.old_user_cpu) as f32)
        / (now - p.old_cpu) as f32 / nb_processors as f32 * 100.;
    p.old_cpu = now;
    p.old_user_cpu = user;
    p.old_sys_cpu = sys;
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
            p.memory = auto_cast!(pmc.PrivateUsage, u64) >> 10; // / 1024;
        }
    }
}
