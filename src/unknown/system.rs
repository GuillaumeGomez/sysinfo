// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    Cpu, CpuRefreshKind, Error, LoadAvg, MemoryRefreshKind, Pid, Process, ProcessRefreshKind,
    ProcessesToUpdate,
};

use std::collections::HashMap;
use std::time::Duration;

declare_signals! {
    (),
    _ => None,
}

#[doc = include_str!("../../md_doc/supported_signals.md")]
pub const SUPPORTED_SIGNALS: &[crate::Signal] = supported_signals();
#[doc = include_str!("../../md_doc/minimum_cpu_update_interval.md")]
pub const MINIMUM_CPU_UPDATE_INTERVAL: Duration = Duration::from_millis(0);

pub(crate) struct SystemInner;

impl SystemInner {
    pub(crate) fn new() -> Result<Self, Error> {
        Err(Error::Unsupported)
    }

    pub(crate) fn refresh_memory_specifics(&mut self, _refresh_kind: MemoryRefreshKind) {}

    pub(crate) fn cgroup_limits(&self) -> Option<crate::CGroupLimits> {
        unreachable!()
    }

    pub(crate) fn refresh_cpu_specifics(&mut self, _refresh_kind: CpuRefreshKind) {}

    pub(crate) fn refresh_cpu_list(&mut self, _refresh_kind: CpuRefreshKind) {}

    pub(crate) fn refresh_processes_specifics(
        &mut self,
        _processes_to_update: ProcessesToUpdate<'_>,
        _refresh_kind: ProcessRefreshKind,
    ) -> usize {
        unreachable!()
    }

    // COMMON PART
    //
    // Need to be moved into a "common" file to avoid duplication.

    pub(crate) fn processes(&self) -> &HashMap<Pid, Process> {
        unreachable!()
    }

    pub(crate) fn processes_mut(&mut self) -> &mut HashMap<Pid, Process> {
        unreachable!()
    }

    pub(crate) fn process(&self, _pid: Pid) -> Option<&Process> {
        unreachable!()
    }

    pub(crate) fn global_cpu_usage(&self) -> f32 {
        unreachable!()
    }

    pub(crate) fn cpus(&self) -> &[Cpu] {
        unreachable!()
    }

    pub(crate) fn total_memory(&self) -> u64 {
        unreachable!()
    }

    pub(crate) fn free_memory(&self) -> u64 {
        unreachable!()
    }

    pub(crate) fn available_memory(&self) -> u64 {
        unreachable!()
    }

    pub(crate) fn used_memory(&self) -> u64 {
        unreachable!()
    }

    pub(crate) fn total_swap(&self) -> u64 {
        unreachable!()
    }

    pub(crate) fn free_swap(&self) -> u64 {
        unreachable!()
    }

    pub(crate) fn used_swap(&self) -> u64 {
        unreachable!()
    }

    pub(crate) fn uptime() -> Result<u64, crate::Error> {
        Err(crate::Error::Unsupported)
    }

    pub(crate) fn boot_time() -> Result<u64, crate::Error> {
        Err(crate::Error::Unsupported)
    }

    pub(crate) fn load_average() -> Result<LoadAvg, crate::Error> {
        Err(crate::Error::Unsupported)
    }

    pub(crate) fn name() -> Option<String> {
        None
    }

    pub(crate) fn long_os_version() -> Option<String> {
        None
    }

    pub(crate) fn kernel_version() -> Option<String> {
        None
    }

    pub(crate) fn os_version() -> Option<String> {
        None
    }

    pub(crate) fn distribution_id() -> String {
        std::env::consts::OS.to_owned()
    }

    pub(crate) fn distribution_id_like() -> Vec<String> {
        Vec::new()
    }

    pub(crate) fn kernel_name() -> Option<&'static str> {
        None
    }

    pub(crate) fn host_name() -> Option<String> {
        None
    }

    pub(crate) fn cpu_arch() -> Option<String> {
        None
    }

    pub(crate) fn physical_core_count() -> Option<usize> {
        None
    }

    pub(crate) fn open_files_limit() -> Option<usize> {
        None
    }
}
