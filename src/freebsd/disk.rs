// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{DiskExt, DiskKind};

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use super::utils::c_buf_to_str;

#[doc = include_str!("../../md_doc/disk.md")]
pub struct Disk {
    name: OsString,
    c_mount_point: Vec<libc::c_char>,
    mount_point: PathBuf,
    total_space: u64,
    available_space: u64,
    file_system: Vec<u8>,
    is_removable: bool,
}

impl DiskExt for Disk {
    fn kind(&self) -> DiskKind {
        DiskKind::Unknown(-1)
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
            let mut vfs: libc::statvfs = std::mem::zeroed();
            refresh_disk(self, &mut vfs)
        }
    }
}

// FIXME: if you want to get disk I/O usage:
// statfs.[f_syncwrites, f_asyncwrites, f_syncreads, f_asyncreads]

unsafe fn refresh_disk(disk: &mut Disk, vfs: &mut libc::statvfs) -> bool {
    if libc::statvfs(disk.c_mount_point.as_ptr() as *const _, vfs) < 0 {
        return false;
    }
    let f_frsize: u64 = vfs.f_frsize as _;

    disk.total_space = vfs.f_blocks.saturating_mul(f_frsize);
    disk.available_space = vfs.f_favail.saturating_mul(f_frsize);
    true
}

pub unsafe fn get_all_disks() -> Vec<Disk> {
    let mut fs_infos: *mut libc::statfs = std::ptr::null_mut();

    let count = libc::getmntinfo(&mut fs_infos, libc::MNT_WAIT);

    if count < 1 {
        return Vec::new();
    }
    let mut vfs: libc::statvfs = std::mem::zeroed();
    let fs_infos: &[libc::statfs] = std::slice::from_raw_parts(fs_infos as _, count as _);
    let mut disks = Vec::new();

    for fs_info in fs_infos {
        if fs_info.f_mntfromname[0] == 0 || fs_info.f_mntonname[0] == 0 {
            // If we have missing information, no need to look any further...
            continue;
        }
        let fs_type: &[libc::c_char] =
            if let Some(pos) = fs_info.f_fstypename.iter().position(|x| *x == 0) {
                &fs_info.f_fstypename[..pos]
            } else {
                &fs_info.f_fstypename
            };
        let fs_type: &[u8] = std::slice::from_raw_parts(fs_type.as_ptr() as _, fs_type.len());
        match fs_type {
            b"autofs" | b"devfs" | b"linprocfs" | b"procfs" | b"fdesckfs" | b"tmpfs"
            | b"linsysfs" => {
                sysinfo_debug!(
                    "Memory filesystem `{:?}`, ignoring it.",
                    c_buf_to_str(&fs_info.f_fstypename).unwrap(),
                );
                continue;
            }
            _ => {}
        }

        if libc::statvfs(fs_info.f_mntonname.as_ptr(), &mut vfs) != 0 {
            continue;
        }

        let mount_point = match c_buf_to_str(&fs_info.f_mntonname) {
            Some(m) => m,
            None => {
                sysinfo_debug!("Cannot get disk mount point, ignoring it.");
                continue;
            }
        };

        let name = if mount_point == "/" {
            OsString::from("root")
        } else {
            OsString::from(mount_point)
        };

        // USB keys and CDs are removable.
        let is_removable =
            [b"USB", b"usb"].iter().any(|b| b == &fs_type) || fs_type.starts_with(b"/dev/cd");

        let f_frsize: u64 = vfs.f_frsize as _;

        disks.push(Disk {
            name,
            c_mount_point: fs_info.f_mntonname.to_vec(),
            mount_point: PathBuf::from(mount_point),
            total_space: vfs.f_blocks.saturating_mul(f_frsize),
            available_space: vfs.f_favail.saturating_mul(f_frsize),
            file_system: fs_type.to_vec(),
            is_removable,
        });
    }
    disks
}
