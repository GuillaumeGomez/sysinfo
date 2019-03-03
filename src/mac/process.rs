// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use std::fmt::{self, Formatter, Debug};
use std::path::{Path, PathBuf};

use libc::{c_int, gid_t, kill, uid_t};

use Pid;
use ::ProcessExt;

/// Enum describing the different status of a process.
#[derive(Clone, Copy, Debug)]
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

/// Enum describing the different status of a thread.
#[derive(Clone, Debug)]
pub enum ThreadStatus {
    /// Thread is running normally.
    Running,
    /// Thread is stopped.
    Stopped,
    /// Thread is waiting normally.
    Waiting,
    /// Thread is in an uninterruptible wait
    Uninterruptible,
    /// Thread is halted at a clean point.
    Halted,
    /// Unknown.
    Unknown(i32),
}

impl From<i32> for ThreadStatus {
    fn from(status: i32) -> ThreadStatus {
        match status {
            1 => ThreadStatus::Running,
            2 => ThreadStatus::Stopped,
            3 => ThreadStatus::Waiting,
            4 => ThreadStatus::Uninterruptible,
            5 => ThreadStatus::Halted,
            x => ThreadStatus::Unknown(x),
        }
    }
}

impl ThreadStatus {
    /// Used to display `ThreadStatus`.
    pub fn to_string(&self) -> &str {
        match *self {
            ThreadStatus::Running         => "Running",
            ThreadStatus::Stopped         => "Stopped",
            ThreadStatus::Waiting         => "Waiting",
            ThreadStatus::Uninterruptible => "Uninterruptible",
            ThreadStatus::Halted          => "Halted",
            ThreadStatus::Unknown(_)      => "Unknown",
        }
    }
}

impl fmt::Display for ThreadStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Struct containing a process' information.
#[derive(Clone)]
pub struct Process {
    pub(crate) name: String,
    pub(crate) cmd: Vec<String>,
    pub(crate) exe: PathBuf,
    pid: Pid,
    parent: Option<Pid>,
    pub(crate) environ: Vec<String>,
    cwd: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) memory: u64,
    utime: u64,
    stime: u64,
    old_utime: u64,
    old_stime: u64,
    start_time: u64,
    updated: bool,
    cpu_usage: f32,
    /// User id of the process owner.
    pub uid: uid_t,
    /// Group id of the process owner.
    pub gid: gid_t,
    pub(crate) process_status: ProcessStatus,
    /// Status of process (running, stopped, waiting, etc). `None` means `sysinfo` doesn't have
    /// enough rights to get this information.
    ///
    /// This is very likely this one that you want instead of `process_status`.
    pub status: Option<ThreadStatus>,
}

impl ProcessExt for Process {
    fn new(pid: Pid, parent: Option<Pid>, start_time: u64) -> Process {
        Process {
            name: String::new(),
            pid,
            parent,
            cmd: Vec::new(),
            environ: Vec::new(),
            exe: PathBuf::new(),
            cwd: PathBuf::new(),
            root: PathBuf::new(),
            memory: 0,
            cpu_usage: 0.,
            utime: 0,
            stime: 0,
            old_utime: 0,
            old_stime: 0,
            updated: true,
            start_time,
            uid: 0,
            gid: 0,
            process_status: ProcessStatus::Unknown(0),
            status: None,
        }
    }

    fn kill(&self, signal: ::Signal) -> bool {
        unsafe { kill(self.pid, signal as c_int) == 0 }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn cmd(&self) -> &[String] {
        &self.cmd
    }

    fn exe(&self) -> &Path {
        self.exe.as_path()
    }

    fn pid(&self) -> Pid {
        self.pid
    }

    fn environ(&self) -> &[String] {
        &self.environ
    }

    fn cwd(&self) -> &Path {
        self.cwd.as_path()
    }

    fn root(&self) -> &Path {
        self.root.as_path()
    }

    fn memory(&self) -> u64 {
        self.memory
    }

    fn parent(&self) -> Option<Pid> {
        self.parent
    }

    fn status(&self) -> ProcessStatus {
        self.process_status
    }

    fn start_time(&self) -> u64 {
        self.start_time
    }

    fn cpu_usage(&self) -> f32 {
        self.cpu_usage
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
        writeln!(f, "executable path: {:?}", self.exe);
        writeln!(f, "current working directory: {:?}", self.cwd);
        writeln!(f, "owner/group: {}:{}", self.uid, self.gid);
        writeln!(f, "memory usage: {} kB", self.memory);
        writeln!(f, "cpu usage: {}%", self.cpu_usage);
        writeln!(f, "status: {}", match self.status {
            Some(ref v) => v.to_string(),
            None        => "Unknown",
        });
        write!(f, "root path: {:?}", self.root)
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

pub fn force_update(p: &mut Process) {
    p.updated = true;
}
