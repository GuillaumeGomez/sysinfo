// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

use ::DiskExt;

use libc::statfs;
use std::{mem, str};
use std::fmt::{Debug, Error, Formatter};

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

pub fn new_disk(name: String, mount_point: &str, type_: DiskType) -> Disk {
    let mut mount_point = mount_point.as_bytes().to_vec();
    mount_point.push(0);
    let mut total_space = 0;
    let mut available_space = 0;
    let mut file_system = None;
    unsafe {
        let mut stat: statfs = mem::zeroed();
        if statfs(mount_point.as_ptr() as *const i8, &mut stat) == 0 {
            total_space = stat.f_bsize as u64 * stat.f_blocks as u64;
            available_space = stat.f_bfree as u64  * stat.f_blocks as u64;
            let mut vec = Vec::with_capacity(stat.f_fstypename.len());
            for x in &stat.f_fstypename {
                if *x == 0 {
                    break
                }
                vec.push(*x as u8);
            }
            file_system = Some(String::from_utf8_unchecked(vec));
        }
    }
    Disk {
        type_: type_,
        name: name,
        file_system: file_system.unwrap_or_else(|| "<Unknown>".to_owned()),
        mount_point: mount_point,
        total_space: total_space,
        available_space: available_space,
    }
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

impl DiskExt for Disk {
    fn get_type(&self) -> DiskType {
        self.type_
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_file_system(&self) -> &str {
        &self.file_system
    }

    fn get_mount_point(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.mount_point[..self.mount_point.len() - 1]) }
    }

    fn get_total_space(&self) -> u64 {
        self.total_space
    }

    fn get_available_space(&self) -> u64 {
        self.available_space
    }

    fn update(&mut self) -> bool {
        unsafe {
            let mut stat: statfs = mem::zeroed();
            if statfs(self.mount_point.as_ptr() as *const i8, &mut stat) == 0 {
                self.available_space = stat.f_bsize as u64 * stat.f_bavail as u64;
                true
            } else {
                false
            }
        }
    }
}
