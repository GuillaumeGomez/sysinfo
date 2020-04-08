//
// Sysinfo
//
// Copyright (c) 2020 Guillaume Gomez
//

use Component;
use ComponentExt;
use Disk;
use DiskExt;
use NetworkData;
use NetworkExt;
use Networks;
use NetworksExt;
use Process;
use ProcessExt;
use Processor;
use ProcessorExt;
use System;
use SystemExt;

use std::fmt;

impl fmt::Debug for Processor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Processor")
            .field("name", &self.get_name())
            .field("CPU usage", &self.get_cpu_usage())
            .finish()
    }
}

impl fmt::Debug for System {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("System")
            .field(
                "global CPU usage",
                &self.get_global_processor_info().get_cpu_usage(),
            )
            .field("load average", &self.get_load_average())
            .field("total memory", &self.get_total_memory())
            .field("free memory", &self.get_free_memory())
            .field("total swap", &self.get_total_swap())
            .field("free swap", &self.get_free_swap())
            .field("nb CPUs", &self.get_processors().len())
            .field("nb network interfaces", &self.get_networks().iter().count())
            .field("nb processes", &self.get_processes().len())
            .field("nb disks", &self.get_disks().len())
            .field("nb components", &self.get_components().len())
            .finish()
    }
}

impl fmt::Debug for Disk {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "Disk({:?})[FS: {:?}][Type: {:?}] mounted on {:?}: {}/{} B",
            self.get_name(),
            self.get_file_system(),
            self.get_type(),
            self.get_mount_point(),
            self.get_available_space(),
            self.get_total_space()
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
        if let Some(critical) = self.get_critical() {
            write!(
                f,
                "{}: {}°C (max: {}°C / critical: {}°C)",
                self.get_label(),
                self.get_temperature(),
                self.get_max(),
                critical
            )
        } else {
            write!(
                f,
                "{}: {}°C (max: {}°C)",
                self.get_label(),
                self.get_temperature(),
                self.get_max()
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
                .map(|x| format!("{:?}", x))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl fmt::Debug for NetworkData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NetworkData")
            .field("income", &self.get_received())
            .field("total income", &self.get_total_received())
            .field("outcome", &self.get_transmitted())
            .field("total outcome", &self.get_total_transmitted())
            .field("packets income", &self.get_packets_received())
            .field("total packets income", &self.get_total_packets_received())
            .field("packets outcome", &self.get_packets_transmitted())
            .field(
                "total packets outcome",
                &self.get_total_packets_transmitted(),
            )
            .field("errors income", &self.get_errors_on_received())
            .field("total errors income", &self.get_total_errors_on_received())
            .field("errors outcome", &self.get_errors_on_transmitted())
            .field(
                "total errors outcome",
                &self.get_total_errors_on_transmitted(),
            )
            .finish()
    }
}
