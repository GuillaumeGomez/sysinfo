// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{DiskUsage, Gid, Pid, ProcessExt, ProcessStatus, Signal, Uid};

use std::fmt;
use std::path::Path;

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Unknown")
    }
}

#[doc = include_str!("../../md_doc/process.md")]
pub struct Process {
    pid: Pid,
    parent: Option<Pid>,
}

impl ProcessExt for Process {
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

    fn user_id(&self) -> Option<&Uid> {
        None
    }

    fn effective_user_id(&self) -> Option<&Uid> {
        None
    }

    fn group_id(&self) -> Option<Gid> {
        None
    }

    fn effective_group_id(&self) -> Option<Gid> {
        None
    }

    fn wait(&self) {}

    fn session_id(&self) -> Option<Pid> {
        None
    }
}
