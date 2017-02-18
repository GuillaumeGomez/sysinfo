// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use std::fmt::{self, Formatter, Debug};
use libc::{c_int, gid_t, kill, pid_t, uid_t};

use ::ProcessExt;

/// Struct containing a process' information.
#[derive(Clone)]
pub enum ProcessStatus {
    /// Waiting in uninterruptible disk sleep.
    Idle,
    /// Running.
    Run,
    /// Sleeping in an interruptible waiting.
    Sleep,
    /// Stopped (on a signal) or (before Linux 2.6.33) trace stopped.
    Stop,
    /// Zombie.
    Zombie,
    /// Tracing stop (Linux 2.6.33 onward).
    Tracing,
    /// Dead.
    Dead,
    /// Wakekill (Linux 2.6.33 to 3.13 only).
    Wakekill,
    /// Waking (Linux 2.6.33 to 3.13 only).
    Waking,
    /// Parked (Linux 3.9 to 3.13 only).
    Parked,
}

pub trait ProcessStatusInput<T> : Clone {
    fn new(status:T) -> Option<ProcessStatus>;
}

impl ProcessStatusInput<u32> for ProcessStatus { 
    fn new(status:u32) -> Option<ProcessStatus> {
        match status {
            1 => Some(ProcessStatus::Idle),
            2 => Some(ProcessStatus::Run),
            3 => Some(ProcessStatus::Sleep),
            4 => Some(ProcessStatus::Stop),
            5 => Some(ProcessStatus::Zombie),
            _ => None,
        }
    }
}

impl ProcessStatusInput<char> for ProcessStatus {
    fn new(status:char) -> Option<ProcessStatus> {
        match status {
            'R' => Some(ProcessStatus::Run),
            'S' => Some(ProcessStatus::Sleep),
            'D' => Some(ProcessStatus::Idle),
            'Z' => Some(ProcessStatus::Zombie),
            'T' => Some(ProcessStatus::Stop),
            't' => Some(ProcessStatus::Tracing),
            'X' => Some(ProcessStatus::Dead),
            'x' => Some(ProcessStatus::Dead),
            'K' => Some(ProcessStatus::Wakekill),
            'W' => Some(ProcessStatus::Waking),
            'P' => Some(ProcessStatus::Parked),
            _   => None,
        }
    }
}

impl ProcessStatus {
    pub fn string(&self) -> &str {
        match *self {
            ProcessStatus::Idle     => "Idle",
            ProcessStatus::Run      => "Runnable",
            ProcessStatus::Sleep    => "Sleeping",
            ProcessStatus::Stop     => "Stopped",
            ProcessStatus::Zombie   => "Zombie",
            ProcessStatus::Tracing  => "Tracing",
            ProcessStatus::Dead     => "Dead",
            ProcessStatus::Wakekill => "Wakekill",
            ProcessStatus::Waking   => "Waking",
            ProcessStatus::Parked   => "Parked",
        }
    }
}

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.string())
    }
}

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
            Some(ref v) => v.string(),
            None        => "Unknown",
        });
        write!(f, "root path: {}", self.root)
    }
}

pub fn compute_cpu_usage(p: &mut Process, nb_processors: u64, total_time: f32) {
    p.cpu_usage = ((p.utime - p.old_utime + p.stime - p.old_stime) * nb_processors * 100) as f32 / total_time;
    p.updated = false;
}

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
