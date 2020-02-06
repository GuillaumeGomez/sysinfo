//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

use std::ffi::{OsStr, OsString};
use std::fmt::{Debug, Error, Formatter};
use std::path::Path;
use std::str;

use DiskExt;
use DiskType;

use winapi::um::fileapi::GetDiskFreeSpaceExW;
use winapi::um::winnt::ULARGE_INTEGER;

pub fn new_disk(
    name: &OsStr,
    mount_point: &[u16],
    file_system: &[u8],
    type_: DiskType,
    total_space: u64,
) -> Disk {
    let mut d = Disk {
        type_: type_,
        name: name.to_owned(),
        file_system: file_system.to_vec(),
        mount_point: mount_point.to_vec(),
        s_mount_point: String::from_utf16_lossy(&mount_point[..mount_point.len() - 1]),
        total_space: total_space,
        available_space: 0,
    };
    d.update();
    d
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

impl Debug for Disk {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        write!(
            fmt,
            "Disk({:?})[FS: {:?}][Type: {:?}] mounted on {:?}: {}/{} B",
            self.get_name(),
            str::from_utf8(self.get_file_system()).unwrap(),
            self.get_type(),
            self.get_mount_point(),
            self.get_available_space(),
            self.get_total_space()
        )
    }
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
        &Path::new(&self.s_mount_point)
    }

    fn get_total_space(&self) -> u64 {
        self.total_space
    }

    fn get_available_space(&self) -> u64 {
        self.available_space
    }

    fn update(&mut self) -> bool {
        if self.total_space != 0 {
            unsafe {
                let mut tmp: ULARGE_INTEGER = ::std::mem::zeroed();
                if GetDiskFreeSpaceExW(
                    self.mount_point.as_ptr(),
                    ::std::ptr::null_mut(),
                    ::std::ptr::null_mut(),
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
