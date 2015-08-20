// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use std::fmt::{self, Formatter, Debug};
use libc::{c_int};

#[derive(Clone)]
pub struct Process {
    pub name: String, // name of the program
    pub cmd: String, // command line
    pub exe: String, // path to the executable
    pub pid: i64, // pid of the processus
    pub environ: Vec<String>, // environment of the processus
    pub cwd: String, // current working directory
    pub root: String, // path of the root directory
    pub memory: u64, // memory usage
    utime: u64,
    cutime: u64,
    stime: u64,
    cstime: u64,
    minflt: u64,
    old_utime: u64,
    old_cutime: u64,
    old_stime: u64,
    old_cstime: u64,
    old_minflt: u64,
    updated: bool,
    pub cpu_usage: f32, // total cpu usage
}

impl Process {
    pub fn new(pid: i64) -> Process {
        Process {
            name: String::new(),
            pid: pid,
            cmd: String::new(),
            environ: Vec::new(),
            exe: String::new(),
            cwd: String::new(),
            root: String::new(),
            memory: 0,
            cpu_usage: 0f32,
            utime: 0,
            cutime: 0,
            stime: 0,
            cstime: 0,
            minflt: 0,
            old_utime: 0,
            old_cutime: 0,
            old_stime: 0,
            old_cstime: 0,
            old_minflt: 0,
            updated: true,
        }
    }

    pub fn kill(&self, signal: ::Signal) -> bool {
        unsafe { ::ffi::kill(self.pid as c_int, signal as c_int) == 0 }
    }
}

#[allow(unused_must_use)]
impl Debug for Process {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "pid: {}\n", self.pid);
        write!(f, "name: {}\n", self.name);
        write!(f, "environment:");
        for var in self.environ.iter() {
            write!(f, "\n\t{}", var);
        }
        write!(f, "command: {}\n", self.cmd);
        write!(f, "executable path: {}\n", self.exe);
        write!(f, "current working directory: {}\n", self.cwd);
        write!(f, "memory usage: {} kB\n", self.memory);
        write!(f, "cpu usage: {}%\n", self.cpu_usage);
        write!(f, "root path: {}", self.root)
    }
}

pub fn set_time(p: &mut Process, minflt: u64, utime: u64, stime: u64, cutime: u64, cstime: u64) {
    //println!("---> {} {} {}", p.utime, p.stime, p.minflt);
    p.old_utime = p.utime;
    p.old_cutime = p.cutime;
    p.old_stime = p.stime;
    p.old_cstime = p.cstime;
    p.old_minflt = p.minflt;
    p.utime = utime;
    p.cutime = cutime;
    p.stime = stime;
    p.cstime = cstime;
    p.minflt = minflt;
    p.updated = true;
    //println!("({}, {}) / ({}, {})", p.utime, p.cutime, p.old_utime, p.old_cutime);
}

pub fn compute_cpu_usage(p: &mut Process, old_utime: u64, old_stime: u64, total_time: f32) {
    /*p.cpu_usage = ((p.utime - old_utime) as f32 / (total_time - old_total_time) as f32
        + (p.stime - old_stime) as f32 / (total_time - old_total_time) as f32) * 25f32;*/
    //p.cpu_usage = 100f32 * (p.utime + p.stime - p.old_utime - p.old_stime) as f32 / (total_time - old_total_time) as f32;
    p.cpu_usage = 100f32 * (p.utime + p.cutime + p.minflt - p.old_minflt - p.old_utime - p.old_cutime) as f32 / total_time;
    p.updated = false;
    //println!("({} + {} + {} - {} - {} - {}) / {} = {}", p.utime, p.stime, p.minflt, p.old_utime, p.old_stime, p.old_minflt, total_time, p.cpu_usage);
    /*if p.cpu_usage > 0f32 {
        println!("{} {} / {}", p.utime - p.old_utime, p.cutime - p.old_cutime, total_time);
    }*/
    /*if p.cpu_usage > 0f32 {
        println!("100 * ({} - {}) / ({} - {}) = {}", p.utime, old_utime, total_time, old_total_time, p.cpu_usage);
    }*/
}

pub fn get_raw_process_times(p: &Process) -> (u64, u64) {
    (p.utime, p.stime)
}

pub fn has_been_updated(p: &Process) -> bool {
    p.updated
}