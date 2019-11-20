//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use DiskExt;

use std::ffi::OsStr;
use std::path::Path;

/// Enum containing the different handled disks types.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DiskType {}

/// Struct containing a disk information.
pub struct Disk {}

impl DiskExt for Disk {
    fn get_type(&self) -> DiskType {
        unreachable!()
    }

    fn get_name(&self) -> &OsStr {
        unreachable!()
    }

    fn get_file_system(&self) -> &[u8] {
        &[]
    }

    fn get_mount_point(&self) -> &Path {
        Path::new("")
    }

    fn get_total_space(&self) -> u64 {
        0
    }

    fn get_available_space(&self) -> u64 {
        0
    }

    fn update(&mut self) -> bool {
        true
    }
}
