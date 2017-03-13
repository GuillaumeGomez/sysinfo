// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use std::fmt::{self, Formatter, Debug};
use libc::{c_int, gid_t, kill, pid_t, uid_t};

use ::ProcessExt;

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
    /// Name of the program.
    pub name: String,
    /// Command line, split into arguments.
    pub cmd: Vec<String>,
    /// Path to the executable.
    pub exe: String,
    /// Pid of the process.
    pub pid: pid_t,
    /// Pid of the parent process.
    pub parent: Option<pid_t>,
    /// Environment of the process.
    pub environ: Vec<String>,
    /// Current working directory.
    pub cwd: String,
    /// Path of the root directory.
    pub root: String,
    /// Memory usage (in kB).
    pub memory: u64,
    utime: u64,
    stime: u64,
    old_utime: u64,
    old_stime: u64,
    /// Time of process launch (in seconds).
    pub start_time: u64,
    updated: bool,
    /// Total cpu usage.
    pub cpu_usage: f32,
    /// User id of the process owner.
    pub uid: uid_t,
    /// Group id of the process owner.
    pub gid: gid_t,
    /// status of process (idle, run, zombie, etc)
    pub status: Option<ProcessStatus>,
}

impl ProcessExt for Process {
    fn new(pid: pid_t, parent: Option<pid_t>, start_time: u64) -> Process {
        Process {
            name: String::new(),
            pid: pid,
            parent: parent,
            cmd: Vec::new(),
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
            uid: 0,
            gid: 0,
            status: None,
        }
    }

    fn kill(&self, signal: ::Signal) -> bool {
        unsafe { kill(self.pid, signal as c_int) == 0 }
    }
}

#[allow(unused_must_use)]
impl Debug for Process {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        writeln!(f, "pid: {}", self.pid);
        writeln!(f, "parent: {:?}", self.parent);
        writeln!(f, "name: {}", self.name);
        writeln!(f, "environment:");
        for var in &self.environ {
            if !var.is_empty() {
                writeln!(f, "\t{}", var);
            }
        }
        writeln!(f, "command:");
        for arg in &self.cmd {
            writeln!(f, "\t{}", arg);
        }
        writeln!(f, "executable path: {}", self.exe);
        writeln!(f, "current working directory: {}", self.cwd);
        writeln!(f, "owner/group: {}:{}", self.uid, self.gid);
        writeln!(f, "memory usage: {} kB", self.memory);
        writeln!(f, "cpu usage: {}%", self.cpu_usage);
        writeln!(f, "status: {}", match self.status {
            Some(ref v) => v.to_string(),
            None        => "Unknown",
        });
        write!(f, "root path: {}", self.root)
    }
}

pub fn compute_cpu_usage(p: &mut Process, time: u64, task_time: u64) {
    let system_time_delta = task_time - p.old_utime;
    let time_delta = time - p.old_stime;
    p.old_utime = task_time;
    p.old_stime = time;
    p.cpu_usage = if time_delta == 0 {
        0f32
    } else {
        (system_time_delta as f64 * 100f64 / time_delta as f64) as f32
    };
    p.updated = true;
}

/*pub fn set_time(p: &mut Process, utime: u64, stime: u64) {
    p.old_utime = p.utime;
    p.old_stime = p.stime;
    p.utime = utime;
    p.stime = stime;
    p.updated = true;
}*/

pub fn has_been_updated(p: &mut Process) -> bool {
    let old = p.updated;
    p.updated = false;
    old
}
