//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};

use libc::{c_int, gid_t, kill, uid_t};

use crate::{DiskUsage, Pid, ProcessExt, Signal};

/// Enum describing the different status of a process.
#[derive(Clone, Copy, Debug)]
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

impl From<char> for ProcessStatus {
    fn from(status: char) -> ProcessStatus {
        match status {
            'R' => ProcessStatus::Run,
            'S' => ProcessStatus::Sleep,
            'D' => ProcessStatus::Idle,
            'Z' => ProcessStatus::Zombie,
            'T' => ProcessStatus::Stop,
            't' => ProcessStatus::Tracing,
            'X' | 'x' => ProcessStatus::Dead,
            'K' => ProcessStatus::Wakekill,
            'W' => ProcessStatus::Waking,
            'P' => ProcessStatus::Parked,
            x => ProcessStatus::Unknown(x as u32),
        }
    }
}

impl ProcessStatus {
    /// Used to display `ProcessStatus`.
    pub fn as_str(&self) -> &str {
        match *self {
            ProcessStatus::Idle => "Idle",
            ProcessStatus::Run => "Runnable",
            ProcessStatus::Sleep => "Sleeping",
            ProcessStatus::Stop => "Stopped",
            ProcessStatus::Zombie => "Zombie",
            ProcessStatus::Tracing => "Tracing",
            ProcessStatus::Dead => "Dead",
            ProcessStatus::Wakekill => "Wakekill",
            ProcessStatus::Waking => "Waking",
            ProcessStatus::Parked => "Parked",
            ProcessStatus::Unknown(_) => "Unknown",
        }
    }
}

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Struct containing a process' information.
pub struct Process {
    pub(crate) name: String,
    pub(crate) cmd: Vec<String>,
    pub(crate) exe: PathBuf,
    pub(crate) pid: Pid,
    parent: Option<Pid>,
    pub(crate) environ: Vec<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) memory: u64,
    pub(crate) virtual_memory: u64,
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
    pub(crate) status: ProcessStatus,
    /// Tasks run by this process.
    pub tasks: HashMap<Pid, Process>,
    pub(crate) stat_file: Option<File>,
    old_read_bytes: u64,
    old_written_bytes: u64,
    read_bytes: u64,
    written_bytes: u64,
}

impl ProcessExt for Process {
    fn new(pid: Pid, parent: Option<Pid>, start_time: u64) -> Process {
        Process {
            name: String::with_capacity(20),
            pid,
            parent,
            cmd: Vec::with_capacity(2),
            environ: Vec::with_capacity(10),
            exe: PathBuf::new(),
            cwd: PathBuf::new(),
            root: PathBuf::new(),
            memory: 0,
            virtual_memory: 0,
            cpu_usage: 0.,
            utime: 0,
            stime: 0,
            old_utime: 0,
            old_stime: 0,
            updated: true,
            start_time,
            uid: 0,
            gid: 0,
            status: ProcessStatus::Unknown(0),
            tasks: if pid == 0 {
                HashMap::with_capacity(1000)
            } else {
                HashMap::new()
            },
            stat_file: None,
            old_read_bytes: 0,
            old_written_bytes: 0,
            read_bytes: 0,
            written_bytes: 0,
        }
    }

    fn kill(&self, signal: Signal) -> bool {
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

    fn virtual_memory(&self) -> u64 {
        self.virtual_memory
    }

    fn parent(&self) -> Option<Pid> {
        self.parent
    }

    /// Returns the status of the processus (idle, run, zombie, etc). `None` means that
    /// `sysinfo` doesn't have enough rights to get this information.
    fn status(&self) -> ProcessStatus {
        self.status
    }

    fn start_time(&self) -> u64 {
        self.start_time
    }

    fn cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    fn disk_usage(&self) -> DiskUsage {
        DiskUsage {
            written_bytes: self.written_bytes - self.old_written_bytes,
            total_written_bytes: self.written_bytes,
            read_bytes: self.read_bytes - self.old_read_bytes,
            total_read_bytes: self.read_bytes,
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if self.stat_file.is_some() {
            if let Ok(ref mut x) = unsafe { crate::sys::system::REMAINING_FILES.lock() } {
                **x += 1;
            }
        }
    }
}

pub fn compute_cpu_usage(p: &mut Process, nb_processors: u64, total_time: f32) {
    // First time updating the values without reference, wait for a second cycle to update cpu_usage
    if p.old_utime == 0 && p.old_stime == 0 {
        return;
    }

    p.cpu_usage =
        ((p.utime - p.old_utime + p.stime - p.old_stime) * nb_processors * 100) as f32 / total_time;
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

pub(crate) fn update_process_disk_activity(p: &mut Process, path: &Path) {
    let mut path = PathBuf::from(path);
    path.push("io");
    let data = match super::system::get_all_data(&path, 16_384) {
        Ok(d) => d,
        Err(_) => return,
    };
    let mut done = 0;
    for line in data.split('\n') {
        let mut parts = line.split(": ");
        match parts.next() {
            Some("read_bytes") => {
                p.old_read_bytes = p.read_bytes;
                p.read_bytes = parts
                    .next()
                    .and_then(|x| x.parse::<u64>().ok())
                    .unwrap_or(p.old_read_bytes);
            }
            Some("write_bytes") => {
                p.old_written_bytes = p.written_bytes;
                p.written_bytes = parts
                    .next()
                    .and_then(|x| x.parse::<u64>().ok())
                    .unwrap_or(p.old_written_bytes);
            }
            _ => continue,
        }
        done += 1;
        if done > 1 {
            // No need to continue the reading.
            break;
        }
    }
}
