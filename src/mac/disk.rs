// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

use ::DiskExt;
use ::utils;

use libc::statfs;
use std::mem;
use std::fmt::{Debug, Error, Formatter};
use std::path::{Path, PathBuf};
use std::ffi::{OsStr, OsString};

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

pub fn new(name: OsString, mount_point: &Path, type_: DiskType) -> Disk {
    let mount_point_cpath = utils::to_cpath(mount_point);
    let mut total_space = 0;
    let mut available_space = 0;
    let mut file_system = None;
    unsafe {
        let mut stat: statfs = mem::zeroed();
        if statfs(mount_point_cpath.as_ptr() as *const i8, &mut stat) == 0 {
            total_space = u64::from(stat.f_bsize) * stat.f_blocks;
            available_space = stat.f_bfree * stat.f_blocks;
            let mut vec = Vec::with_capacity(stat.f_fstypename.len());
            for x in &stat.f_fstypename {
                if *x == 0 {
                    break
                }
                vec.push(*x as u8);
            }
            file_system = Some(vec);
        }
    }
    Disk {
        type_,
        name,
        file_system: file_system.unwrap_or_else(|| b"<Unknown>".to_vec()),
        mount_point: mount_point.to_owned(),
        total_space,
        available_space,
    }
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

    fn update(&mut self) -> bool {
        unsafe {
            let mut stat: statfs = mem::zeroed();
            let mount_point_cpath = utils::to_cpath(&self.mount_point);
            if statfs(mount_point_cpath.as_ptr() as *const i8, &mut stat) == 0 {
                self.available_space = u64::from(stat.f_bsize) * stat.f_bavail;
                true
            } else {
                false
            }
        }
    }
}
