//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use std::path::Path;
use DiskUsage;
use Pid;
use ProcessExt;

/// Enum describing the different status of a process.
#[derive(Clone, Copy, Debug)]
pub struct ProcessStatus;

/// Struct containing a process' information.
#[derive(Clone)]
pub struct Process {
    pid: Pid,
    parent: Option<Pid>,
}

impl ProcessExt for Process {
    fn new(pid: Pid, parent: Option<Pid>, _start_time: u64) -> Process {
        Process { pid, parent }
    }

    fn kill(&self, _signal: ::Signal) -> bool {
        false
    }

    fn name(&self) -> &str {
        ""
    }

    fn cmd(&self) -> &[String] {
        &[]
    }

    fn exe(&self) -> &Path {
        &Path::new("")
    }

    fn pid(&self) -> Pid {
        self.pid
    }

    fn environ(&self) -> &[String] {
        &[]
    }

    fn cwd(&self) -> &Path {
        &Path::new("")
    }

    fn root(&self) -> &Path {
        &Path::new("")
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
        ProcessStatus
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
