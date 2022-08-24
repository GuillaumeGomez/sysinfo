// Take a look at the license at the top of the repository in the LICENSE file.

use crate::utils::to_cpath;
use crate::{DiskExt, DiskType};

#[cfg(target_os = "macos")]
pub(crate) use crate::sys::inner::disk::*;

use libc::statfs;
use std::ffi::{OsStr, OsString};
use std::mem;
use std::path::{Path, PathBuf};

#[doc = include_str!("../../md_doc/disk.md")]
pub struct Disk {
    pub(crate) type_: DiskType,
    pub(crate) name: OsString,
    pub(crate) file_system: Vec<u8>,
    pub(crate) mount_point: PathBuf,
    pub(crate) total_space: u64,
    pub(crate) available_space: u64,
    pub(crate) is_removable: bool,
}

impl DiskExt for Disk {
    fn type_(&self) -> DiskType {
        self.type_
    }

    fn name(&self) -> &OsStr {
        &self.name
    }

    fn file_system(&self) -> &[u8] {
        &self.file_system
    }

    fn mount_point(&self) -> &Path {
        &self.mount_point
    }

    fn total_space(&self) -> u64 {
        self.total_space
    }

    fn available_space(&self) -> u64 {
        self.available_space
    }

    fn is_removable(&self) -> bool {
        self.is_removable
    }

    fn refresh(&mut self) -> bool {
        unsafe {
            let mut stat: statfs = mem::zeroed();
            let mount_point_cpath = to_cpath(&self.mount_point);
            if statfs(mount_point_cpath.as_ptr() as *const i8, &mut stat) == 0 {
                self.available_space = u64::from(stat.f_bsize).saturating_mul(stat.f_bavail);
                true
            } else {
                false
            }
        }
    }
}
