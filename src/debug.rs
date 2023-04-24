// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    Component, ComponentExt, Cpu, CpuExt, Disk, DiskExt, NetworkData, NetworkExt, Networks,
    NetworksExt, Process, ProcessExt, System, SystemExt,
};

use std::fmt;

impl fmt::Debug for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cpu")
            .field("name", &self.name())
            .field("CPU usage", &self.cpu_usage())
            .field("frequency", &self.frequency())
            .field("vendor ID", &self.vendor_id())
            .field("brand", &self.brand())
            .finish()
    }
}

impl fmt::Debug for System {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("System")
            .field("global CPU usage", &self.global_cpu_info().cpu_usage())
            .field("load average", &self.load_average())
            .field("total memory", &self.total_memory())
            .field("free memory", &self.free_memory())
            .field("total swap", &self.total_swap())
            .field("free swap", &self.free_swap())
            .field("nb CPUs", &self.cpus().len())
            .field("nb network interfaces", &self.networks().iter().count())
            .field("nb processes", &self.processes().len())
            .field("nb disks", &self.disks().len())
            .field("nb components", &self.components().len())
            .finish()
    }
}

impl fmt::Debug for Disk {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "Disk({:?})[FS: {:?}][Type: {:?}][removable: {}] mounted on {:?}: {}/{} B",
            self.name(),
            self.file_system()
                .iter()
                .map(|c| *c as char)
                .collect::<Vec<_>>(),
            self.kind(),
            if self.is_removable() { "yes" } else { "no" },
            self.mount_point(),
            self.available_space(),
            self.total_space(),
        )
    }
}

impl fmt::Debug for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Process")
            .field("pid", &self.pid())
            .field("parent", &self.parent())
            .field("name", &self.name())
            .field("environ", &self.environ())
            .field("command", &self.cmd())
            .field("executable path", &self.exe())
            .field("current working directory", &self.cwd())
            .field("memory usage", &self.memory())
            .field("virtual memory usage", &self.virtual_memory())
            .field("CPU usage", &self.cpu_usage())
            .field("status", &self.status())
            .field("root", &self.root())
            .field("disk_usage", &self.disk_usage())
            .finish()
    }
}

impl fmt::Debug for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(critical) = self.critical() {
            write!(
                f,
                "{}: {}°C (max: {}°C / critical: {}°C)",
                self.label(),
                self.temperature(),
                self.max(),
                critical
            )
        } else {
            write!(
                f,
                "{}: {}°C (max: {}°C)",
                self.label(),
                self.temperature(),
                self.max()
            )
        }
    }
}

impl fmt::Debug for Networks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Networks {{ {} }}",
            self.iter()
                .map(|x| format!("{x:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl fmt::Debug for NetworkData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NetworkData")
            .field("income", &self.received())
            .field("total income", &self.total_received())
            .field("outcome", &self.transmitted())
            .field("total outcome", &self.total_transmitted())
            .field("packets income", &self.packets_received())
            .field("total packets income", &self.total_packets_received())
            .field("packets outcome", &self.packets_transmitted())
            .field("total packets outcome", &self.total_packets_transmitted())
            .field("errors income", &self.errors_on_received())
            .field("total errors income", &self.total_errors_on_received())
            .field("errors outcome", &self.errors_on_transmitted())
            .field("total errors outcome", &self.total_errors_on_transmitted())
            .finish()
    }
}
