use std::fmt::{self, Formatter, Debug};
use libc::{c_int};

pub struct Processus {
    pub cmd: String, // command line
    pub exe: String, // path to the executable
    pub pid: i64, // pid of the processus
    pub environ: Vec<String>, // environment of the processus
    pub cwd: String, // current working directory
    pub root: String, // path of the root directory
    pub memory: u64, // memory usage
    utime: u64,
    stime: u64,
    pub cpu_usage: f32 // total cpu usage
}

impl Processus {
    pub fn new(pid: i64) -> Processus {
        Processus {
            pid: pid,
            cmd: String::new(),
            environ: Vec::new(),
            exe: String::new(),
            cwd: String::new(),
            root: String::new(),
            memory: 0,
            cpu_usage: 0f32,
            utime: 0,
            stime: 0
        }
    }

    pub fn kill(&self, signal: ::Signal) -> bool {
        unsafe { ::ffi::kill(self.pid as c_int, signal as c_int) == 0 }
    }
}

impl Debug for Processus {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "pid: {}\n", self.pid);
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

pub fn set_time(p: &mut Processus, utime: u64, stime: u64) {
    p.utime = utime;
    p.stime = stime;
}

pub fn compute_cpu_usage(p: &mut Processus, old_utime: u64, old_stime: u64, total_time: u64, old_total_time: u64) {
    p.cpu_usage = 100f32 * (p.utime - old_utime) as f32 / (total_time - old_total_time) as f32;
}

pub fn get_raw_processus_times(p: &Processus) -> (u64, u64) {
    (p.utime, p.stime)
}