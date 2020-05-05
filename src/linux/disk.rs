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

macro_rules! cast {
    ($x:expr) => {
        u64::from($x)
    };
}

/// Struct containing a disk information.
#[derive(PartialEq)]
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

fn new_disk(name: &OsStr, mount_point: &Path, file_system: &[u8]) -> Option<Disk> {
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
    if total == 0 {
        return None;
    }
    Some(Disk {
        type_,
        name: name.to_owned(),
        file_system: file_system.to_owned(),
        mount_point: mount_point.to_owned(),
        total_space: cast!(total),
        available_space: cast!(available),
    })
}

fn find_type_for_name(name: &OsStr) -> DiskType {
    // The format of devices are as follows:
    //  - name_path is symbolic link in the case of /dev/mapper/
    //     and /dev/root, and the target is corresponding device under
    //     /sys/block/
    //  - In the case of /dev/sd, the format is /dev/sd[a-z][1-9],
    //     corresponding to /sys/block/sd[a-z]
    //  - In the case of /dev/nvme, the format is /dev/nvme[0-9]n[0-9]p[0-9],
    //     corresponding to /sys/block/nvme[0-9]n[0-9]
    //  - In the case of /dev/mmcblk, the format is /dev/mmcblk[0-9]p[0-9],
    //     corresponding to /sys/block/mmcblk[0-9]
    let name_path = name.to_str().unwrap_or_default();
    let real_path = fs::canonicalize(name_path).unwrap_or_else(|_| PathBuf::from(name_path));
    let mut real_path = real_path.to_str().unwrap_or_default();
    if name_path.starts_with("/dev/mapper/") {
        // Recursively solve, for example /dev/dm-0
        return find_type_for_name(OsStr::new(&real_path));
    } else if name_path.starts_with("/dev/sd") {
        // Turn "sda1" into "sda"
        real_path = real_path.trim_start_matches("/dev/");
        real_path = real_path.trim_end_matches(|c| c >= '0' && c <= '9');
    } else if name_path.starts_with("/dev/nvme") {
        // Turn "nvme0n1p1" into "nvme0n1"
        real_path = real_path.trim_start_matches("/dev/");
        real_path = real_path.trim_end_matches(|c| c >= '0' && c <= '9');
        real_path = real_path.trim_end_matches(|c| c == 'p');
    } else if name_path.starts_with("/dev/root") {
        // Recursively solve, for example /dev/mmcblk0p1
        return find_type_for_name(OsStr::new(&real_path));
    } else if name_path.starts_with("/dev/mmcblk") {
        // Turn "mmcblk0p1" into "mmcblk0"
        real_path = real_path.trim_start_matches("/dev/");
        real_path = real_path.trim_end_matches(|c| c >= '0' && c <= '9');
        real_path = real_path.trim_end_matches(|c| c == 'p');
    } else {
        // Default case: remove /dev/ and expects the name presents under /sys/block/
        // For example, /dev/dm-0 to dm-0
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

fn get_all_disks_inner(content: &str) -> Vec<Disk> {
    content
        .lines()
        .map(|line| {
            let line = line.trim_start();
            // mounts format
            // http://man7.org/linux/man-pages/man5/fstab.5.html
            // fs_spec<tab>fs_file<tab>fs_vfstype<tab>other fields
            let mut fields = line.split_whitespace();
            let fs_spec = fields.next().unwrap_or("");
            let fs_file = fields.next().unwrap_or("");
            let fs_vfstype = fields.next().unwrap_or("");
            (fs_spec, fs_file, fs_vfstype)
        })
        .filter(|(fs_spec, fs_file, fs_vfstype)| {
            // Check if fs_vfstype is one of our 'ignored' file systems.
            let filtered = match *fs_vfstype {
                "sysfs" | // pseudo file system for kernel objects
                "proc" |  // another pseudo file system
                "tmpfs" |
                "cgroup" |
                "cgroup2" |
                "pstore" | // https://www.kernel.org/doc/Documentation/ABI/testing/pstore
                "squashfs" | // squashfs is a compressed read-only file system (for snaps)
                "rpc_pipefs" | // The pipefs pseudo file system service
                "iso9660" => true, // optical media
                _ => false,
            };

            !(filtered ||
               fs_file.starts_with("/sys") || // check if fs_file is an 'ignored' mount point
               fs_file.starts_with("/proc") ||
               fs_file.starts_with("/run") ||
               fs_spec.starts_with("sunrpc"))
        })
        .filter_map(|(fs_spec, fs_file, fs_vfstype)| {
            new_disk(fs_spec.as_ref(), Path::new(fs_file), fs_vfstype.as_bytes())
        })
        .collect()
}

pub fn get_all_disks() -> Vec<Disk> {
    get_all_disks_inner(&get_all_data("/proc/mounts", 16_385).unwrap_or_default())
}

// #[test]
// fn check_all_disks() {
//     let disks = get_all_disks_inner(
//         r#"tmpfs /proc tmpfs rw,seclabel,relatime 0 0
// proc /proc proc rw,nosuid,nodev,noexec,relatime 0 0
// systemd-1 /proc/sys/fs/binfmt_misc autofs rw,relatime,fd=29,pgrp=1,timeout=0,minproto=5,maxproto=5,direct,pipe_ino=17771 0 0
// tmpfs /sys tmpfs rw,seclabel,relatime 0 0
// sysfs /sys sysfs rw,seclabel,nosuid,nodev,noexec,relatime 0 0
// securityfs /sys/kernel/security securityfs rw,nosuid,nodev,noexec,relatime 0 0
// cgroup2 /sys/fs/cgroup cgroup2 rw,seclabel,nosuid,nodev,noexec,relatime,nsdelegate 0 0
// pstore /sys/fs/pstore pstore rw,seclabel,nosuid,nodev,noexec,relatime 0 0
// none /sys/fs/bpf bpf rw,nosuid,nodev,noexec,relatime,mode=700 0 0
// configfs /sys/kernel/config configfs rw,nosuid,nodev,noexec,relatime 0 0
// selinuxfs /sys/fs/selinux selinuxfs rw,relatime 0 0
// debugfs /sys/kernel/debug debugfs rw,seclabel,nosuid,nodev,noexec,relatime 0 0
// tmpfs /dev/shm tmpfs rw,seclabel,relatime 0 0
// devpts /dev/pts devpts rw,seclabel,relatime,gid=5,mode=620,ptmxmode=666 0 0
// tmpfs /sys/fs/selinux tmpfs rw,seclabel,relatime 0 0
// /dev/vda2 /proc/filesystems xfs rw,seclabel,relatime,attr2,inode64,logbufs=8,logbsize=32k,noquota 0 0
// "#,
//     );
//     assert_eq!(disks.len(), 1);
//     assert_eq!(
//         disks[0],
//         Disk {
//             type_: DiskType::Unknown(-1),
//             name: OsString::from("devpts"),
//             file_system: vec![100, 101, 118, 112, 116, 115],
//             mount_point: PathBuf::from("/dev/pts"),
//             total_space: 0,
//             available_space: 0,
//         }
//     );
// }
