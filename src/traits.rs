//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use sys::{Component, Disk, DiskType, Networks, Process, Processor};
use LoadAvg;
use NetworksIter;
use Pid;
use ProcessStatus;
use RefreshKind;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;

/// Contains all the methods of the `Disk` struct.
///
/// ```no_run
/// use sysinfo::{DiskExt, System, SystemExt};
///
/// let s = System::new();
/// for disk in s.get_disks() {
///     println!("{:?}: {:?}", disk.get_name(), disk.get_type());
/// }
/// ```
pub trait DiskExt {
    /// Returns the disk type.
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for disk in s.get_disks() {
    ///     println!("{:?}", disk.get_type());
    /// }
    /// ```
    fn get_type(&self) -> DiskType;

    /// Returns the disk name.
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for disk in s.get_disks() {
    ///     println!("{:?}", disk.get_name());
    /// }
    /// ```
    fn get_name(&self) -> &OsStr;

    /// Returns the file system used on this disk (so for example: `EXT4`, `NTFS`, etc...).
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for disk in s.get_disks() {
    ///     println!("{:?}", disk.get_file_system());
    /// }
    /// ```
    fn get_file_system(&self) -> &[u8];

    /// Returns the mount point of the disk (`/` for example).
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for disk in s.get_disks() {
    ///     println!("{:?}", disk.get_mount_point());
    /// }
    /// ```
    fn get_mount_point(&self) -> &Path;

    /// Returns the total disk size, in bytes.
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for disk in s.get_disks() {
    ///     println!("{}", disk.get_total_space());
    /// }
    /// ```
    fn get_total_space(&self) -> u64;

    /// Returns the available disk size, in bytes.
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for disk in s.get_disks() {
    ///     println!("{}", disk.get_available_space());
    /// }
    /// ```
    fn get_available_space(&self) -> u64;

    /// Update the disk' information.
    #[doc(hidden)]
    fn update(&mut self) -> bool;
}

/// Contains all the methods of the `Process` struct.
pub trait ProcessExt {
    /// Create a new process only containing the given information.
    ///
    /// On windows, the `start_time` argument is ignored.
    #[doc(hidden)]
    fn new(pid: Pid, parent: Option<Pid>, start_time: u64) -> Self;

    /// Sends the given `signal` to the process.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, Signal, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     process.kill(Signal::Kill);
    /// }
    /// ```
    fn kill(&self, signal: ::Signal) -> bool;

    /// Returns the name of the process.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     println!("{}", process.name());
    /// }
    /// ```
    fn name(&self) -> &str;

    /// Returns the command line.
    // ///
    // /// On Windows, this is always a one element vector.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     println!("{:?}", process.cmd());
    /// }
    /// ```
    fn cmd(&self) -> &[String];

    /// Returns the path to the process.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     println!("{}", process.exe().display());
    /// }
    /// ```
    fn exe(&self) -> &Path;

    /// Returns the pid of the process.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     println!("{}", process.pid());
    /// }
    /// ```
    fn pid(&self) -> Pid;

    /// Returns the environment of the process.
    ///
    /// Always empty on Windows, except for current process.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     println!("{:?}", process.environ());
    /// }
    /// ```
    fn environ(&self) -> &[String];

    /// Returns the current working directory.
    ///
    /// Always empty on Windows.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     println!("{}", process.cwd().display());
    /// }
    /// ```
    fn cwd(&self) -> &Path;

    /// Returns the path of the root directory.
    ///
    /// Always empty on Windows.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     println!("{}", process.root().display());
    /// }
    /// ```
    fn root(&self) -> &Path;

    /// Returns the memory usage (in KiB).
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     println!("{} KiB", process.memory());
    /// }
    /// ```
    fn memory(&self) -> u64;

    /// Returns the virtual memory usage (in KiB).
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     println!("{} KiB", process.virtual_memory());
    /// }
    /// ```
    fn virtual_memory(&self) -> u64;

    /// Returns the parent pid.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     println!("{:?}", process.parent());
    /// }
    /// ```
    fn parent(&self) -> Option<Pid>;

    /// Returns the status of the processus.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     println!("{:?}", process.status());
    /// }
    /// ```
    fn status(&self) -> ProcessStatus;

    /// Returns the time of process launch (in seconds).
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     println!("{}", process.start_time());
    /// }
    /// ```
    fn start_time(&self) -> u64;

    /// Returns the total CPU usage (in %).
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     println!("{}%", process.cpu_usage());
    /// }
    /// ```
    fn cpu_usage(&self) -> f32;
}

/// Contains all the methods of the `Processor` struct.
pub trait ProcessorExt {
    /// Returns this processor's usage.
    ///
    /// Note: You'll need to refresh it at least twice at first if you want to have a
    /// non-zero value.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessorExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for processor in s.get_processor_list() {
    ///     println!("{}%", processor.get_cpu_usage());
    /// }
    /// ```
    fn get_cpu_usage(&self) -> f32;

    /// Returns this processor's name.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessorExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for processor in s.get_processor_list() {
    ///     println!("{}", processor.get_name());
    /// }
    /// ```
    fn get_name(&self) -> &str;

    /// Returns the processor's vendor id.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessorExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for processor in s.get_processor_list() {
    ///     println!("{}", processor.get_vendor_id());
    /// }
    /// ```
    fn get_vendor_id(&self) -> &str;

    /// Returns the processor's brand.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessorExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for processor in s.get_processor_list() {
    ///     println!("{}", processor.get_brand());
    /// }
    /// ```
    fn get_brand(&self) -> &str;

    /// Returns the processor's frequency.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessorExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for processor in s.get_processor_list() {
    ///     println!("{}", processor.get_frequency());
    /// }
    /// ```
    fn get_frequency(&self) -> u64;
}

/// Contains all the methods of the [`System`] type.
pub trait SystemExt: Sized {
    /// Creates a new [`System`] instance. It only contains the disks' list  and the processes list
    /// at this stage. Use the [`refresh_all`] method to update its internal information (or any of
    /// the `refresh_` method).
    ///
    /// [`refresh_all`]: #method.refresh_all
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// ```
    fn new() -> Self {
        let mut s = Self::new_with_specifics(RefreshKind::new());
        s.refresh_disk_list();
        s.refresh_all();
        s
    }

    /// Creates a new [`System`] instance and refresh the data corresponding to the
    /// given [`RefreshKind`].
    ///
    /// # Example
    ///
    /// ```
    /// use sysinfo::{RefreshKind, System, SystemExt};
    ///
    /// // We want everything except disks.
    /// let mut system = System::new_with_specifics(RefreshKind::everything().without_disk_list());
    ///
    /// assert_eq!(system.get_disks().len(), 0);
    /// assert!(system.get_process_list().len() > 0);
    ///
    /// // If you want the disks list afterwards, just call the corresponding
    /// // "refresh_disk_list":
    /// system.refresh_disk_list();
    /// let disks = system.get_disks();
    /// ```
    fn new_with_specifics(refreshes: RefreshKind) -> Self;

    /// Refreshes according to the given [`RefreshKind`]. It calls the corresponding
    /// "refresh_" methods.
    ///
    /// # Examples
    ///
    /// ```
    /// use sysinfo::{RefreshKind, System, SystemExt};
    ///
    /// let mut s = System::new();
    ///
    /// // Let's just update networks and processes:
    /// s.refresh_specifics(RefreshKind::new().with_networks().with_processes());
    /// ```
    fn refresh_specifics(&mut self, refreshes: RefreshKind) {
        if refreshes.memory() {
            self.refresh_memory();
        }
        if refreshes.cpu() {
            self.refresh_cpu();
        }
        if refreshes.temperatures() {
            self.refresh_temperatures();
        }
        if refreshes.networks() {
            self.refresh_networks();
        }
        if refreshes.processes() {
            self.refresh_processes();
        }
        if refreshes.disk_list() {
            self.refresh_disk_list();
        }
        if refreshes.disks() {
            self.refresh_disks();
        }
    }

    /// Refresh system information (such as memory, swap, CPU usage and components' temperature).
    ///
    /// If you want some more specific refresh, you might be interested into looking at
    /// [`refresh_memory`], [`refresh_cpu`] and [`refresh_temperatures`].
    ///
    /// [`refresh_memory`]: SystemExt::refresh_memory
    /// [`refresh_cpu`]: SystemExt::refresh_memory
    /// [`refresh_temperatures`]: SystemExt::refresh_temperatures
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new();
    /// s.refresh_system();
    /// ```
    fn refresh_system(&mut self) {
        self.refresh_memory();
        self.refresh_cpu();
        self.refresh_temperatures();
    }

    /// Refresh RAM and SWAP usage.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new();
    /// s.refresh_memory();
    /// ```
    fn refresh_memory(&mut self);

    /// Refresh CPU usage.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new();
    /// s.refresh_cpu();
    /// ```
    fn refresh_cpu(&mut self);

    /// Refresh components' temperature.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new();
    /// s.refresh_temperatures();
    /// ```
    fn refresh_temperatures(&mut self);

    /// Get all processes and update their information.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new();
    /// s.refresh_processes();
    /// ```
    fn refresh_processes(&mut self);

    /// Refresh *only* the process corresponding to `pid`. Returns `false` if the process doesn't
    /// exist.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new();
    /// s.refresh_process(1337);
    /// ```
    fn refresh_process(&mut self, pid: Pid) -> bool;

    /// Refreshes the listed disks' information.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new();
    /// s.refresh_disks();
    /// ```
    fn refresh_disks(&mut self);

    /// The disk list will be emptied then completely recomputed.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new();
    /// s.refresh_disk_list();
    /// ```
    fn refresh_disk_list(&mut self);

    /// Refresh networks data.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new();
    /// s.refresh_networks();
    /// ```
    fn refresh_networks(&mut self);

    /// Refreshes all system, processes and disks information.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new();
    /// s.refresh_all();
    /// ```
    fn refresh_all(&mut self) {
        self.refresh_system();
        self.refresh_processes();
        self.refresh_disks();
        self.refresh_networks();
    }

    /// Returns the process list.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for (pid, process) in s.get_process_list() {
    ///     println!("{} {}", pid, process.name());
    /// }
    /// ```
    fn get_process_list(&self) -> &HashMap<Pid, Process>;

    /// Returns the process corresponding to the given pid or `None` if no such process exists.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.get_process(1337) {
    ///     println!("{}", process.name());
    /// }
    /// ```
    fn get_process(&self, pid: Pid) -> Option<&Process>;

    /// Returns a list of process containing the given `name`.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for process in s.get_process_by_name("htop") {
    ///     println!("{} {}", process.pid(), process.name());
    /// }
    /// ```
    fn get_process_by_name(&self, name: &str) -> Vec<&Process> {
        let mut ret = vec![];
        for val in self.get_process_list().values() {
            if val.name().contains(name) {
                ret.push(val);
            }
        }
        ret
    }

    /// The first processor in the array is the "main" one (aka the addition of all the others).
    ///
    /// ```no_run
    /// use sysinfo::{ProcessorExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for processor in s.get_processor_list() {
    ///     println!("{}%", processor.get_cpu_usage());
    /// }
    /// ```
    fn get_processor_list(&self) -> &[Processor];

    /// Returns total RAM size in KiB.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("{} KiB", s.get_total_memory());
    /// ```
    fn get_total_memory(&self) -> u64;

    /// Returns free RAM size in KiB.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("{} KiB", s.get_free_memory());
    /// ```
    fn get_free_memory(&self) -> u64;

    /// Returns used RAM size in KiB.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("{} KiB", s.get_used_memory());
    /// ```
    fn get_used_memory(&self) -> u64;

    /// Returns SWAP size in KiB.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("{} KiB", s.get_total_swap());
    /// ```
    fn get_total_swap(&self) -> u64;

    /// Returns free SWAP size in KiB.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("{} KiB", s.get_free_swap());
    /// ```
    fn get_free_swap(&self) -> u64;

    /// Returns used SWAP size in KiB.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("{} KiB", s.get_used_swap());
    /// ```
    fn get_used_swap(&self) -> u64;

    /// Returns components list.
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for component in s.get_components_list() {
    ///     println!("{}: {}°C", component.get_label(), component.get_temperature());
    /// }
    /// ```
    fn get_components_list(&self) -> &[Component];

    /// Returns disks' list.
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for disk in s.get_disks() {
    ///     println!("{:?}", disk.get_name());
    /// }
    /// ```
    fn get_disks(&self) -> &[Disk];

    /// Returns network interfaces.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// let networks = s.get_networks();
    /// for (interface_name, data) in networks.iter() {
    ///     println!("[{}] in: {}, out: {}", interface_name, data.get_income(), data.get_outcome());
    /// }
    /// ```
    fn get_networks(&self) -> &Networks;

    /// Returns system uptime.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("{}", s.get_uptime());
    /// ```
    fn get_uptime(&self) -> u64;

    /// Returns the system load average value.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// let load_avg = s.get_load_average();
    /// println!(
    ///     "one minute: {}%, five minutes: {}%, fifteen minutes: {}%",
    ///     load_avg.one,
    ///     load_avg.five,
    ///     load_avg.fifteen,
    /// );
    /// ```
    fn get_load_average(&self) -> LoadAvg;
}

/// Getting volume of incoming and outgoing data.
pub trait NetworkExt {
    /// Returns the number of incoming bytes since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// let networks = s.get_networks();
    /// for (interface_name, network) in networks.iter() {
    ///     println!("in: {} B", network.get_income());
    /// }
    /// ```
    fn get_income(&self) -> u64;

    /// Returns the number of outgoing bytes since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// let networks = s.get_networks();
    /// for (interface_name, network) in networks.iter() {
    ///     println!("in: {} B", network.get_outcome());
    /// }
    /// ```
    fn get_outcome(&self) -> u64;

    /// Returns the total number of incoming bytes.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// let networks = s.get_networks();
    /// for (interface_name, network) in networks.iter() {
    ///     println!("in: {} B", network.get_total_income());
    /// }
    /// ```
    fn get_total_income(&self) -> u64;

    /// Returns the total number of outgoing bytes.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// let networks = s.get_networks();
    /// for (interface_name, network) in networks.iter() {
    ///     println!("in: {} B", network.get_total_outcome());
    /// }
    /// ```
    fn get_total_outcome(&self) -> u64;
}

/// Interacting with network interfaces.
pub trait NetworksExt {
    /// Returns an iterator over the network interfaces.
    fn iter(&self) -> NetworksIter;
}

/// Getting a component temperature information.
pub trait ComponentExt {
    /// Returns the component's temperature (in celsius degree).
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for component in s.get_components_list() {
    ///     println!("{}°C", component.get_temperature());
    /// }
    /// ```
    fn get_temperature(&self) -> f32;

    /// Returns the maximum temperature of this component.
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for component in s.get_components_list() {
    ///     println!("{}°C", component.get_max());
    /// }
    /// ```
    fn get_max(&self) -> f32;

    /// Returns the highest temperature before the computer halts.
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for component in s.get_components_list() {
    ///     println!("{:?}°C", component.get_critical());
    /// }
    /// ```
    fn get_critical(&self) -> Option<f32>;

    /// Returns component's label.
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for component in s.get_components_list() {
    ///     println!("{}", component.get_label());
    /// }
    /// ```
    fn get_label(&self) -> &str;
}
