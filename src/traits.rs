//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use sys::{Component, Disk, DiskType, NetworkData, Process, Processor};
use Pid;
use ProcessStatus;
use RefreshKind;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;

/// Contains all the methods of the `Disk` struct.
pub trait DiskExt {
    /// Returns the disk type.
    fn get_type(&self) -> DiskType;

    /// Returns the disk name.
    fn get_name(&self) -> &OsStr;

    /// Returns the file system used on this disk (so for example: `EXT4`, `NTFS`, etc...).
    fn get_file_system(&self) -> &[u8];

    /// Returns the mount point of the disk (`/` for example).
    fn get_mount_point(&self) -> &Path;

    /// Returns the total disk size, in bytes.
    fn get_total_space(&self) -> u64;

    /// Returns the available disk size, in bytes.
    fn get_available_space(&self) -> u64;

    /// Update the disk' information.
    fn update(&mut self) -> bool;
}

/// Contains all the methods of the `Process` struct.
pub trait ProcessExt {
    /// Create a new process only containing the given information.
    ///
    /// On windows, the `start_time` argument is ignored.
    fn new(pid: Pid, parent: Option<Pid>, start_time: u64) -> Self;

    /// Sends the given `signal` to the process.
    fn kill(&self, signal: ::Signal) -> bool;

    /// Returns the name of the process.
    fn name(&self) -> &str;

    /// Returns the command line.
    // ///
    // /// On Windows, this is always a one element vector.
    fn cmd(&self) -> &[String];

    /// Returns the path to the process.
    fn exe(&self) -> &Path;

    /// Returns the pid of the process.
    fn pid(&self) -> Pid;

    /// Returns the environment of the process.
    ///
    /// Always empty on Windows except for current process.
    fn environ(&self) -> &[String];

    /// Returns the current working directory.
    ///
    /// Always empty on Windows.
    fn cwd(&self) -> &Path;

    /// Returns the path of the root directory.
    ///
    /// Always empty on Windows.
    fn root(&self) -> &Path;

    /// Returns the memory usage (in kB).
    fn memory(&self) -> u64;

    /// Returns the virtual memory usage (in kB).
    fn virtual_memory(&self) -> u64;

    /// Returns the parent pid.
    fn parent(&self) -> Option<Pid>;

    /// Returns the status of the processus.
    fn status(&self) -> ProcessStatus;

    /// Returns the time of process launch (in seconds).
    fn start_time(&self) -> u64;

    /// Returns the total CPU usage.
    fn cpu_usage(&self) -> f32;
}

/// Contains all the methods of the `Processor` struct.
pub trait ProcessorExt {
    /// Returns this processor's usage.
    ///
    /// Note: You'll need to refresh it at least twice at first if you want to have a
    /// non-zero value.
    fn get_cpu_usage(&self) -> f32;

    /// Returns this processor's name.
    fn get_name(&self) -> &str;
}

/// Contains all the methods of the [`System`] type.
pub trait SystemExt: Sized {
    /// Creates a new [`System`] instance. It only contains the disks' list  and the processes list
    /// at this stage. Use the [`refresh_all`] method to update its internal information (or any of
    /// the `refresh_` method).
    ///
    /// [`refresh_all`]: #method.refresh_all
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
    /// assert!(system.get_disks().len() > 0);
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
    /// // Let's just update network data and processes:
    /// s.refresh_specifics(RefreshKind::new().with_network().with_processes());
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
        if refreshes.network() {
            self.refresh_network();
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
    fn refresh_system(&mut self) {
        self.refresh_memory();
        self.refresh_cpu();
        self.refresh_temperatures();
    }

    /// Refresh RAM and SWAP usage.
    fn refresh_memory(&mut self);

    /// Refresh CPU usage.
    fn refresh_cpu(&mut self);

    /// Refresh components' temperature.
    fn refresh_temperatures(&mut self);

    /// Get all processes and update their information.
    fn refresh_processes(&mut self);

    /// Refresh *only* the process corresponding to `pid`. Returns `false` if the process doesn't
    /// exist.
    fn refresh_process(&mut self, pid: Pid) -> bool;

    /// Refreshes the listed disks' information.
    fn refresh_disks(&mut self);

    /// The disk list will be emptied then completely recomputed.
    fn refresh_disk_list(&mut self);

    /// Refresh data network.
    fn refresh_network(&mut self);

    /// Refreshes all system, processes and disks information.
    fn refresh_all(&mut self) {
        self.refresh_system();
        self.refresh_processes();
        self.refresh_disks();
        self.refresh_network();
    }

    /// Returns the process list.
    fn get_process_list(&self) -> &HashMap<Pid, Process>;

    /// Returns the process corresponding to the given pid or `None` if no such process exists.
    fn get_process(&self, pid: Pid) -> Option<&Process>;

    /// Returns a list of process containing the given `name`.
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
    fn get_processor_list(&self) -> &[Processor];

    /// Returns total RAM size in KiB.
    fn get_total_memory(&self) -> u64;

    /// Returns free RAM size in KiB.
    fn get_free_memory(&self) -> u64;

    /// Returns used RAM size in KiB.
    fn get_used_memory(&self) -> u64;

    /// Returns SWAP size in KiB.
    fn get_total_swap(&self) -> u64;

    /// Returns free SWAP size in KiB.
    fn get_free_swap(&self) -> u64;

    /// Returns used SWAP size in KiB.
    fn get_used_swap(&self) -> u64;

    /// Returns components list.
    fn get_components_list(&self) -> &[Component];

    /// Returns disks' list.
    fn get_disks(&self) -> &[Disk];

    /// Returns network data.
    fn get_network(&self) -> &NetworkData;

    /// Returns system uptime.
    fn get_uptime(&self) -> u64;
}

/// Getting volume of incoming and outgoing data.
pub trait NetworkExt {
    /// Returns the number of incoming bytes.
    fn get_income(&self) -> u64;

    /// Returns the number of outgoing bytes.
    fn get_outcome(&self) -> u64;
}

/// Getting a component temperature information.
pub trait ComponentExt {
    /// Returns the component's temperature (in celsius degree).
    fn get_temperature(&self) -> f32;
    /// Returns the maximum temperature of this component.
    fn get_max(&self) -> f32;
    /// Returns the highest temperature before the computer halts.
    fn get_critical(&self) -> Option<f32>;
    /// Returns component's label.
    fn get_label(&self) -> &str;
}
