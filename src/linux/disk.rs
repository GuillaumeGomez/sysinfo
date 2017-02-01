// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use super::system::get_all_data;

use libc::statvfs;
use std::{mem, str};
use std::fmt::{Debug, Error, Formatter};
use std::str::FromStr;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DiskType {
    HDD,
    SSD,
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

pub fn new_disk(name: &str, mount_point: &str, file_system: &str) -> Disk {
    let type_ =  isize::from_str(get_all_data(&format!("/sys/block/{}/queue/rotational",
                                                       name)).lines()
                                                             .next()
                                                             .unwrap_or("-1")).unwrap_or(-1);
    let mut mount_point = mount_point.as_bytes().to_vec();
    mount_point.push(0);
    let mut total_space = 0;
    let mut available_space = 0;
    unsafe {
        let mut stat: statvfs = mem::zeroed();
        if statvfs(mount_point.as_ptr() as *const i8, &mut stat) == 0 {
            total_space = stat.f_bsize * stat.f_blocks;
            available_space = stat.f_bsize * stat.f_bavail;
        }
    }
    Disk {
        type_: DiskType::from(type_),
        name: name.to_owned(),
        file_system: file_system.to_owned(),
        mount_point: mount_point,
        total_space: total_space,
        available_space: available_space,
    }
}

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
    pub fn get_type(&self) -> DiskType {
        self.type_
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_file_system(&self) -> &str {
        &self.file_system
    }

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

    pub fn update(&mut self) -> bool {
        unsafe {
            let mut stat: statvfs = mem::zeroed();
            if statvfs(self.mount_point.as_ptr() as *const i8, &mut stat) == 0 {
                self.available_space = stat.f_bsize * stat.f_bavail;
                true
            } else {
                false
            }
        }
    }
}
