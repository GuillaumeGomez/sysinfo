//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

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
    };
    d.refresh();
    Some(d)
}

/// Struct containing a disk information.
pub struct Disk {
    type_: DiskType,
    name: OsString,
    file_system: Vec<u8>,
    mount_point: Vec<u16>,
    s_mount_point: String,
    total_space: u64,
    available_space: u64,
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
