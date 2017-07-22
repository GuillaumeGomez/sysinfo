// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

use sys::{Component, Disk, DiskType, NetworkData, Process, Processor};

use libc::pid_t;
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
    fn new(pid: pid_t, parent: Option<pid_t>, start_time: u64) -> Self;

    /// Sends the given `signal` to the process.
    fn kill(&self, signal: ::Signal) -> bool;
}

/// Contains all the methods of the `Processor` struct.
pub trait ProcessorExt {
    /// Returns this processor's usage.
    fn get_cpu_usage(&self) -> f32;

    /// Returns this processor's name.
    fn get_name(&self) -> &str;
}

/// Contains all the methods of the `System` struct.
pub trait SystemExt {
    /// Creates a new `System` instance. It only contains the disks' list at this stage. Use the
    /// [`refresh_all`] method to update its internal information (or any of the `refresh_` method).
    ///
    /// [`refresh_all`]: #method.refresh_all
    fn new() -> Self;

    /// Refresh system information (such as memory, swap, CPU usage and components' temperature).
    fn refresh_system(&mut self);

    /// Get all processes and update their information.
    fn refresh_processes(&mut self);

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
    fn get_process_list(&self) -> &HashMap<pid_t, Process>;

    /// Returns the process corresponding to the given pid or `None` if no such process exists.
    fn get_process(&self, pid: pid_t) -> Option<&Process>;

    /// Returns a list of process starting with the given name.
    fn get_process_by_name(&self, name: &str) -> Vec<&Process>;

    /// The first processor in the array is the "main" process.
    fn get_processor_list(&self) -> &[Processor];

    /// Returns total RAM size.
    fn get_total_memory(&self) -> u64;

    /// Returns free RAM size.
    fn get_free_memory(&self) -> u64;

    /// Returns used RAM size.
    fn get_used_memory(&self) -> u64;

    /// Returns SWAP size.
    fn get_total_swap(&self) -> u64;

    /// Returns free SWAP size.
    fn get_free_swap(&self) -> u64;

    /// Returns used SWAP size.
    fn get_used_swap(&self) -> u64;

    /// Returns components list.
    fn get_components_list(&self) -> &[Component];

    /// Returns disks' list.
    fn get_disks(&self) -> &[Disk];

    /// Returns network data.
    fn get_network(&self) -> &NetworkData;
}

/// Getting volume of incoming and outcoming data.
pub trait NetworkExt {
    /// Returns the number of incoming bytes.
    fn get_income(&self) -> u64;

    /// Returns the number of outcoming bytes.
    fn get_outcome(&self) -> u64;
}
