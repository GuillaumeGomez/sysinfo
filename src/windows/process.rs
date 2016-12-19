// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

use std::mem::{size_of, zeroed};
use std::fmt::{self, Formatter, Debug};
use libc::{c_uint, c_void};

use kernel32::{self, K32GetProcessMemoryInfo};
use winapi;
use winapi::winnt::HANDLE;
use winapi::psapi::{PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX};
use winapi::minwindef::DWORD;

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
    utime: u64,
    stime: u64,
    old_utime: u64,
    old_stime: u64,
    /// time of process launch (in seconds)
    pub start_time: u64,
    updated: bool,
    /// total cpu usage
    pub cpu_usage: f32,
}

impl Process {
    pub fn new(pid: u32, start_time: u64) -> Process {
        Process {
            name: String::new(),
            pid: pid,
            cmd: String::new(),
            environ: Vec::new(),
            exe: String::new(),
            cwd: String::new(),
            root: String::new(),
            memory: 0,
            cpu_usage: 0.,
            utime: 0,
            stime: 0,
            old_utime: 0,
            old_stime: 0,
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

pub fn compute_cpu_usage(p: &mut Process, nb_processors: u64, total_time: f32) {
    p.cpu_usage = ((p.utime - p.old_utime + p.stime - p.old_stime) * nb_processors * 100) as f32 / total_time;
    p.updated = false;
}

// COMMON PART
//
// Need to be moved into a "common" file to avoid duplication.

pub fn set_time(p: &mut Process, utime: u64, stime: u64) {
    p.old_utime = p.utime;
    p.old_stime = p.stime;
    p.utime = utime;
    p.stime = stime;
    p.updated = true;
}

pub fn has_been_updated(p: &Process) -> bool {
    p.updated
}

pub fn update_proc_info(p: &mut Process, handle: HANDLE) {
    update_memory(p, handle);
    p.updated = true;
}

pub fn update_memory(p: &mut Process, handle: HANDLE) {
    unsafe {
        let mut pmc: PROCESS_MEMORY_COUNTERS_EX = zeroed();
        if K32GetProcessMemoryInfo(handle,
                                   &mut pmc as *mut PROCESS_MEMORY_COUNTERS_EX as *mut c_void as *mut PROCESS_MEMORY_COUNTERS,
                                   size_of::<PROCESS_MEMORY_COUNTERS_EX>() as DWORD) != 0 {
            p.memory = pmc.PrivateUsage >> 10; // / 1024;
        }
    }
}
