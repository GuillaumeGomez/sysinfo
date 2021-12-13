// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{DiskUsage, Pid, ProcessExt, ProcessStatus, Signal};

use std::fmt;
use std::path::Path;

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Unknown")
    }
}

#[doc = include_str!("../../md_doc/process.md")]
#[derive(Clone)]
pub struct Process {
    pid: Pid,
    parent: Option<Pid>,
}

impl ProcessExt for Process {
    fn kill(&self) -> bool {
        false
    }

    fn kill_with(&self, _signal: Signal) -> Option<bool> {
        None
    }

    fn name(&self) -> &str {
        ""
    }

    fn cmd(&self) -> &[String] {
        &[]
    }

    fn exe(&self) -> &Path {
        Path::new("")
    }

    fn pid(&self) -> Pid {
        self.pid
    }

    fn environ(&self) -> &[String] {
        &[]
    }

    fn cwd(&self) -> &Path {
        Path::new("")
    }

    fn root(&self) -> &Path {
        Path::new("")
    }

    fn memory(&self) -> u64 {
        0
    }

    fn virtual_memory(&self) -> u64 {
        0
    }

    fn parent(&self) -> Option<Pid> {
        self.parent
    }

    fn status(&self) -> ProcessStatus {
        ProcessStatus::Unknown(0)
    }

    fn start_time(&self) -> u64 {
        0
    }

    fn run_time(&self) -> u64 {
        0
    }

    fn cpu_usage(&self) -> f32 {
        0.0
    }

    fn disk_usage(&self) -> DiskUsage {
        DiskUsage::default()
    }
}
