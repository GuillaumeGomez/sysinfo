//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use DiskExt;
use DiskType;

use libc::statfs;
use std::ffi::{OsStr, OsString};
use std::mem;
use std::path::{Path, PathBuf};
use utils::to_cpath;

cfg_if! {
    if #[cfg(target_os = "macos")] {
        mod macos;
        pub(crate) use self::macos::get_disks;
    } else {}
}

/// Struct containing a disk information.
pub struct Disk {
    type_: DiskType,
    name: OsString,
    file_system: Vec<u8>,
    mount_point: PathBuf,
    total_space: u64,
    available_space: u64,
}

impl DiskExt for Disk {
    fn get_type(&self) -> DiskType {
        self.type_
    }

    fn get_name(&self) -> &OsStr {
        &self.name
    }

    fn get_file_system(&self) -> &[u8] {
        &self.file_system
    }

    fn get_mount_point(&self) -> &Path {
        &self.mount_point
    }

    fn get_total_space(&self) -> u64 {
        self.total_space
    }

    fn get_available_space(&self) -> u64 {
        self.available_space
    }

    fn refresh(&mut self) -> bool {
        unsafe {
            let mut stat: statfs = mem::zeroed();
            let mount_point_cpath = to_cpath(&self.mount_point);
            if statfs(mount_point_cpath.as_ptr() as *const i8, &mut stat) == 0 {
                self.available_space = u64::from(stat.f_bsize) * stat.f_bavail;
                true
            } else {
                false
            }
        }
    }
}
