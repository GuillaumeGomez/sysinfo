//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use super::system::get_all_data;
use utils;
use DiskExt;
use DiskType;

use libc::statvfs;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

fn find_type_for_name(name: &OsStr) -> DiskType {
    /*
        The format of devices are as follows:
         - name_path is symbolic link in the case of /dev/mapper/
            and /dev/root, and the target is corresponding device under
            /sys/block/
         - In the case of /dev/sd, the format is /dev/sd[a-z][1-9],
            corresponding to /sys/block/sd[a-z]
         - In the case of /dev/nvme, the format is /dev/nvme[0-9]n[0-9]p[0-9],
            corresponding to /sys/block/nvme[0-9]n[0-9]
         - In the case of /dev/mmcblk, the format is /dev/mmcblk[0-9]p[0-9],
            corresponding to /sys/block/mmcblk[0-9]
    */
    let name_path = name.to_str().unwrap_or_default();
    let real_path = fs::canonicalize(name_path).unwrap_or(PathBuf::from(name_path));
    let mut real_path = real_path.to_str().unwrap_or_default();
    if name_path.starts_with("/dev/mapper/")  {
        /* Recursively solve, for example /dev/dm-0 */
        return find_type_for_name(OsStr::new(&real_path));
    } else if name_path.starts_with("/dev/sd") {
        /* Turn "sda1" into "sda" */
        real_path = real_path.trim_start_matches("/dev/");
        real_path = real_path.trim_end_matches(|c| c >= '0' && c <= '9');
    } else if name_path.starts_with("/dev/nvme") {
        /* Turn "nvme0n1p1" into "nvme0n1" */
        real_path = real_path.trim_start_matches("/dev/");
        real_path = real_path.trim_end_matches(|c| c >= '0' && c <= '9');
        real_path = real_path.trim_end_matches(|c| c == 'p');
    } else if name_path.starts_with("/dev/root") {
        /* Recursively solve, for example /dev/mmcblk0p1 */
        return find_type_for_name(OsStr::new(&real_path));
    } else if name_path.starts_with("/dev/mmcblk") {
        /* Turn "mmcblk0p1" into "mmcblk0" */
        real_path = real_path.trim_start_matches("/dev/");
        real_path = real_path.trim_end_matches(|c| c >= '0' && c <= '9');
        real_path = real_path.trim_end_matches(|c| c == 'p');
    } else {
        /*
            Default case: remove /dev/ and expects the name presents under /sys/block/
            For example, /dev/dm-0 to dm-0
        */
        real_path = real_path.trim_start_matches("/dev/");
    }

    let trimmed: &OsStr = OsStrExt::from_bytes(real_path.as_bytes());

    let path = Path::new("/sys/block/")
        .to_owned()
        .join(trimmed)
        .join("queue/rotational");
    // Normally, this file only contains '0' or '1' but just in case, we get 8 bytes...
    let rotational_int = get_all_data(path, 8).unwrap_or_default().trim().parse();
    DiskType::from(rotational_int.unwrap_or(-1))
}

macro_rules! cast {
    ($x:expr) => {
        u64::from($x)
    };
}

pub fn new(name: &OsStr, mount_point: &Path, file_system: &[u8]) -> Disk {
    let mount_point_cpath = utils::to_cpath(mount_point);
    let type_ = find_type_for_name(name);
    let mut total = 0;
    let mut available = 0;
    unsafe {
        let mut stat: statvfs = mem::zeroed();
        if statvfs(mount_point_cpath.as_ptr() as *const _, &mut stat) == 0 {
            total = cast!(stat.f_bsize) * cast!(stat.f_blocks);
            available = cast!(stat.f_bsize) * cast!(stat.f_bavail);
        }
    }
    Disk {
        type_,
        name: name.to_owned(),
        file_system: file_system.to_owned(),
        mount_point: mount_point.to_owned(),
        total_space: cast!(total),
        available_space: cast!(available),
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
            let mut stat: statvfs = mem::zeroed();
            let mount_point_cpath = utils::to_cpath(&self.mount_point);
            if statvfs(mount_point_cpath.as_ptr() as *const _, &mut stat) == 0 {
                let tmp = cast!(stat.f_bsize) * cast!(stat.f_bavail);
                self.available_space = cast!(tmp);
                true
            } else {
                false
            }
        }
    }
}
