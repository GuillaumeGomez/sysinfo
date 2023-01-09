// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    sys::{component::Component, Cpu, Disk, Networks, Process},
    CpuRefreshKind, LoadAvg, Pid, ProcessRefreshKind, RefreshKind, SystemExt, User,
};

use std::collections::HashMap;
use std::time::Duration;

declare_signals! {
    (),
    _ => None,
}

#[doc = include_str!("../../md_doc/system.md")]
pub struct System {
    processes_list: HashMap<Pid, Process>,
    networks: Networks,
    global_cpu: Cpu,
}

impl SystemExt for System {
    const IS_SUPPORTED: bool = false;
    const SUPPORTED_SIGNALS: &'static [Signal] = supported_signals();
    const MINIMUM_CPU_UPDATE_INTERVAL: Duration = Duration::from_millis(0);

    fn new_with_specifics(_: RefreshKind) -> System {
        System {
            processes_list: Default::default(),
            networks: Networks::new(),
            global_cpu: Cpu::new(),
        }
    }

    fn refresh_memory(&mut self) {}

    fn refresh_cpu_specifics(&mut self, _refresh_kind: CpuRefreshKind) {}

    fn refresh_components_list(&mut self) {}

    fn refresh_processes_specifics(&mut self, _refresh_kind: ProcessRefreshKind) {}

    fn refresh_process_specifics(&mut self, _pid: Pid, _refresh_kind: ProcessRefreshKind) -> bool {
        false
    }

    fn refresh_disks_list(&mut self) {}

    fn refresh_users_list(&mut self) {}

    // COMMON PART
    //
    // Need to be moved into a "common" file to avoid duplication.

    fn processes(&self) -> &HashMap<Pid, Process> {
        &self.processes_list
    }

    fn process(&self, _pid: Pid) -> Option<&Process> {
        None
    }

    fn networks(&self) -> &Networks {
        &self.networks
    }

    fn networks_mut(&mut self) -> &mut Networks {
        &mut self.networks
    }

    fn global_cpu_info(&self) -> &Cpu {
        &self.global_cpu
    }

    fn cpus(&self) -> &[Cpu] {
        &[]
    }

    fn physical_core_count(&self) -> Option<usize> {
        None
    }

    fn total_memory(&self) -> u64 {
        0
    }

    fn free_memory(&self) -> u64 {
        0
    }

    fn available_memory(&self) -> u64 {
        0
    }

    fn used_memory(&self) -> u64 {
        0
    }

    fn total_swap(&self) -> u64 {
        0
    }

    fn free_swap(&self) -> u64 {
        0
    }

    fn used_swap(&self) -> u64 {
        0
    }

    fn components(&self) -> &[Component] {
        &[]
    }

    fn components_mut(&mut self) -> &mut [Component] {
        &mut []
    }

    fn disks(&self) -> &[Disk] {
        &[]
    }

    fn disks_mut(&mut self) -> &mut [Disk] {
        &mut []
    }

    fn sort_disks_by<F>(&mut self, _compare: F)
    where
        F: FnMut(&Disk, &Disk) -> std::cmp::Ordering,
    {
        // does nothing.
    }

    fn uptime(&self) -> u64 {
        0
    }

    fn boot_time(&self) -> u64 {
        0
    }

    fn load_average(&self) -> LoadAvg {
        LoadAvg {
            one: 0.,
            five: 0.,
            fifteen: 0.,
        }
    }

    fn users(&self) -> &[User] {
        &[]
    }

    fn name(&self) -> Option<String> {
        None
    }

    fn long_os_version(&self) -> Option<String> {
        None
    }

    fn kernel_version(&self) -> Option<String> {
        None
    }

    fn os_version(&self) -> Option<String> {
        None
    }

    fn distribution_id(&self) -> String {
        std::env::consts::OS.to_owned()
    }

    fn host_name(&self) -> Option<String> {
        None
    }
}

impl Default for System {
    fn default() -> System {
        System::new()
    }
}
