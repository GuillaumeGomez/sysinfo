//
// Sysinfo
//
// Copyright (c) 2021 Guillaume Gomez
//

use std::path::Path;

use crate::{DiskUsage, Pid, ProcessExt, ProcessStatus, Signal};

/// Dummy struct representing a process because iOS doesn't support
/// obtaining process information due to sandboxing.
#[derive(Clone)]
pub struct Process;

impl ProcessExt for Process {
    fn new(_pid: Pid, _parent: Option<Pid>, _start_time: u64) -> Process {
        Process {}
    }

    fn kill(&self, _signal: Signal) -> bool {
        false
    }

    fn name(&self) -> &str {
        ""
    }

    fn cmd(&self) -> &[String] {
        &[]
    }

    fn exe(&self) -> &Path {
        Path::new("/")
    }

    fn pid(&self) -> Pid {
        0
    }

    fn environ(&self) -> &[String] {
        &[]
    }

    fn cwd(&self) -> &Path {
        Path::new("/")
    }

    fn root(&self) -> &Path {
        Path::new("/")
    }

    fn memory(&self) -> u64 {
        0
    }

    fn virtual_memory(&self) -> u64 {
        0
    }

    fn parent(&self) -> Option<Pid> {
        None
    }

    fn status(&self) -> ProcessStatus {
        ProcessStatus::Unknown(0)
    }

    fn start_time(&self) -> u64 {
        0
    }

    fn cpu_usage(&self) -> f32 {
        0.0
    }

    fn disk_usage(&self) -> DiskUsage {
        DiskUsage::default()
    }
}
