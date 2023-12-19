// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    Cpu, CpuInner, CpuRefreshKind, LoadAvg, MemoryRefreshKind, Pid, Process, ProcessRefreshKind,
};

use std::collections::HashMap;

pub(crate) struct SystemInner {
    processes_list: HashMap<Pid, Process>,
    global_cpu: Cpu,
}

impl SystemInner {
    pub(crate) fn new() -> Self {
        Self {
            processes_list: Default::default(),
            global_cpu: Cpu {
                inner: CpuInner::new(),
            },
        }
    }

    pub(crate) fn refresh_memory_specifics(&mut self, _refresh_kind: MemoryRefreshKind) {}

    pub(crate) fn cgroup_limits(&self) -> Option<crate::CGroupLimits> {
        None
    }

    pub(crate) fn refresh_cpu_specifics(&mut self, _refresh_kind: CpuRefreshKind) {}

    pub(crate) fn refresh_processes_specifics(
        &mut self,
        _filter: Option<&[Pid]>,
        _refresh_kind: ProcessRefreshKind,
    ) {
    }

    pub(crate) fn refresh_process_specifics(
        &mut self,
        _pid: Pid,
        _refresh_kind: ProcessRefreshKind,
    ) -> bool {
        false
    }

    // COMMON PART
    //
    // Need to be moved into a "common" file to avoid duplication.

    pub(crate) fn processes(&self) -> &HashMap<Pid, Process> {
        &self.processes_list
    }

    pub(crate) fn process(&self, _pid: Pid) -> Option<&Process> {
        None
    }

    pub(crate) fn global_cpu_info(&self) -> &Cpu {
        &self.global_cpu
    }

    pub(crate) fn cpus(&self) -> &[Cpu] {
        &[]
    }

    pub(crate) fn physical_core_count(&self) -> Option<usize> {
        None
    }

    pub(crate) fn total_memory(&self) -> u64 {
        0
    }

    pub(crate) fn free_memory(&self) -> u64 {
        0
    }

    pub(crate) fn available_memory(&self) -> u64 {
        0
    }

    pub(crate) fn used_memory(&self) -> u64 {
        0
    }

    pub(crate) fn total_swap(&self) -> u64 {
        0
    }

    pub(crate) fn free_swap(&self) -> u64 {
        0
    }

    pub(crate) fn used_swap(&self) -> u64 {
        0
    }

    pub(crate) fn uptime() -> u64 {
        0
    }

    pub(crate) fn boot_time() -> u64 {
        0
    }

    pub(crate) fn load_average() -> LoadAvg {
        LoadAvg {
            one: 0.,
            five: 0.,
            fifteen: 0.,
        }
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

    pub(crate) fn host_name() -> Option<String> {
        None
    }
    pub(crate) fn cpu_arch() -> Option<String> {
        None
    }
}
