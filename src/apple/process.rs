//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use std::fmt;

pub use crate::sys::inner::process::*;

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
    pub fn as_str(&self) -> &str {
        match *self {
            ProcessStatus::Idle => "Idle",
            ProcessStatus::Run => "Runnable",
            ProcessStatus::Sleep => "Sleeping",
            ProcessStatus::Stop => "Stopped",
            ProcessStatus::Zombie => "Zombie",
            ProcessStatus::Unknown(_) => "Unknown",
        }
    }
}

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
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
            ThreadStatus::Running => "Running",
            ThreadStatus::Stopped => "Stopped",
            ThreadStatus::Waiting => "Waiting",
            ThreadStatus::Uninterruptible => "Uninterruptible",
            ThreadStatus::Halted => "Halted",
            ThreadStatus::Unknown(_) => "Unknown",
        }
    }
}

impl fmt::Display for ThreadStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
