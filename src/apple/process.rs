// Take a look at the license at the top of the repository in the LICENSE file.

use std::fmt;

pub use crate::sys::inner::process::*;
use crate::ProcessStatus;

// FIXME: To be removed once <https://github.com/rust-lang/libc/pull/3233> is merged and released.
const SIDL: u32 = 1;
const SRUN: u32 = 2;
const SSLEEP: u32 = 3;
const SSTOP: u32 = 4;
const SZOMB: u32 = 5;

#[doc(hidden)]
impl From<u32> for ProcessStatus {
    fn from(status: u32) -> ProcessStatus {
        match status {
            SIDL => ProcessStatus::Idle,
            SRUN => ProcessStatus::Run,
            SSLEEP => ProcessStatus::Sleep,
            SSTOP => ProcessStatus::Stop,
            SZOMB => ProcessStatus::Zombie,
            x => ProcessStatus::Unknown(x),
        }
    }
}

#[doc(hidden)]
impl From<ThreadStatus> for ProcessStatus {
    fn from(status: ThreadStatus) -> ProcessStatus {
        match status {
            ThreadStatus::Running => ProcessStatus::Run,
            ThreadStatus::Stopped => ProcessStatus::Stop,
            ThreadStatus::Waiting => ProcessStatus::Sleep,
            ThreadStatus::Uninterruptible => ProcessStatus::Dead,
            ThreadStatus::Halted => ProcessStatus::Parked,
            ThreadStatus::Unknown(x) => ProcessStatus::Unknown(x as _),
        }
    }
}

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            ProcessStatus::Idle => "Idle",
            ProcessStatus::Run => "Runnable",
            ProcessStatus::Sleep => "Sleeping",
            ProcessStatus::Stop => "Stopped",
            ProcessStatus::Zombie => "Zombie",
            _ => "Unknown",
        })
    }
}

/// Enum describing the different status of a thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ThreadStatus {
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
            libc::TH_STATE_RUNNING => ThreadStatus::Running,
            libc::TH_STATE_STOPPED => ThreadStatus::Stopped,
            libc::TH_STATE_WAITING => ThreadStatus::Waiting,
            libc::TH_STATE_UNINTERRUPTIBLE => ThreadStatus::Uninterruptible,
            libc::TH_STATE_HALTED => ThreadStatus::Halted,
            x => ThreadStatus::Unknown(x),
        }
    }
}
