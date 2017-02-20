
// use sys::{Component, Disk, DiskType, Process, Processor};

// use libc::pid_t;
// use std::collections::HashMap;

/// Enum describing the different status of a process.
#[derive(Clone, Debug)]
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
            x   => ProcessStatus::Unknown(x as u32),
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
            ProcessStatus::Tracing    => "Tracing",
            ProcessStatus::Dead       => "Dead",
            ProcessStatus::Wakekill   => "Wakekill",
            ProcessStatus::Waking     => "Waking",
            ProcessStatus::Parked     => "Parked",
            ProcessStatus::Unknown(_) => "Unknown",
        }
    }
}

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}


