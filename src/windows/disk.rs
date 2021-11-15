// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{DiskExt, DiskType};

use std::ffi::{OsStr, OsString};
use std::path::Path;

use winapi::um::fileapi::GetDiskFreeSpaceExW;
use winapi::um::winnt::ULARGE_INTEGER;

pub fn new_disk(
    name: &OsStr,
    mount_point: &[u16],
    file_system: &[u8],
    type_: DiskType,
    total_space: u64,
    is_removable: bool,
) -> Option<Disk> {
    if total_space == 0 {
        return None;
    }
    let mut d = Disk {
        type_,
        name: name.to_owned(),
        file_system: file_system.to_vec(),
        mount_point: mount_point.to_vec(),
        s_mount_point: String::from_utf16_lossy(&mount_point[..mount_point.len() - 1]),
        total_space,
        available_space: 0,
        is_removable,
    };
    d.refresh();
    Some(d)
}

#[doc = include_str!("../../md_doc/disk.md")]
pub struct Disk {
    type_: DiskType,
    name: OsString,
    file_system: Vec<u8>,
    mount_point: Vec<u16>,
    s_mount_point: String,
    total_space: u64,
    available_space: u64,
    is_removable: bool,
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
        Path::new(&self.s_mount_point)
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
        if self.total_space != 0 {
            unsafe {
                let mut tmp: ULARGE_INTEGER = std::mem::zeroed();
                if GetDiskFreeSpaceExW(
                    self.mount_point.as_ptr(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut tmp,
                ) != 0
                {
                    self.available_space = *tmp.QuadPart();
                    return true;
                }
            }
        }
        false
    }
}
