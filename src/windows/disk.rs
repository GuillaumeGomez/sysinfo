// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

use std::fmt::{Debug, Error, Formatter};
use std::str;

use winapi::um::fileapi::GetDiskFreeSpaceExA;
use winapi::um::winnt::ULARGE_INTEGER;

/// Enum containing the different handled disks types.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DiskType {
    /// HDD type.
    HDD,
    /// SSD type.
    SSD,
    /// Unknown type.
    Unknown(isize),
}

impl From<isize> for DiskType {
    fn from(t: isize) -> DiskType {
        match t {
            0 => DiskType::HDD,
            1 => DiskType::SSD,
            id => DiskType::Unknown(id),
        }
    }
}

pub fn new_disk(name: &str, mount_point: &[u8], file_system: &str, type_: DiskType,
                total_space: u64) -> Disk {
    let mut d = Disk {
        type_: type_,
        name: name.to_owned(),
        file_system: file_system.to_owned(),
        mount_point:mount_point.to_vec(),
        total_space: total_space,
        available_space: 0,
    };
    d.update();
    d
}

/// Struct containing a disk information.
pub struct Disk {
    type_: DiskType,
    name: String,
    file_system: String,
    mount_point: Vec<u8>,
    total_space: u64,
    available_space: u64,
}

impl Debug for Disk {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        write!(fmt,
               "Disk({:?})[FS: {:?}][Type: {:?}] mounted on {:?}: {}/{} B",
               self.get_name(), self.get_file_system(), self.get_type(), self.get_mount_point(),
               self.get_available_space(), self.get_total_space())
    }
}

impl Disk {
    /// Returns the disk type.
    pub fn get_type(&self) -> DiskType {
        self.type_
    }

    /// Returns the disk name.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Returns the file system used on this disk (so for example: `EXT4`, `NTFS`, etc...).
    pub fn get_file_system(&self) -> &str {
        &self.file_system
    }

    /// Returns the mount point of the disk (`/` for example).
    pub fn get_mount_point(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.mount_point[..self.mount_point.len() - 1]) }
    }

    /// Returns the total disk size, in bytes.
    pub fn get_total_space(&self) -> u64 {
        self.total_space
    }

    /// Returns the available disk size, in bytes.
    pub fn get_available_space(&self) -> u64 {
        self.available_space
    }

    /// Update the disk' information.
    pub fn update(&mut self) -> bool {
        if self.total_space != 0 {
            unsafe {
                let mut tmp: ULARGE_INTEGER = ::std::mem::zeroed();
                if GetDiskFreeSpaceExA(self.mount_point.as_ptr() as *const i8,
                                       ::std::ptr::null_mut(),
                                       ::std::ptr::null_mut(),
                                       &mut tmp) != 0 {
                    self.available_space = *tmp.QuadPart();
                    return true;
                }
            }
        }
        false
    }
}
