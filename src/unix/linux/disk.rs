// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::utils::{get_all_utf8_data, to_cpath};
use crate::{Disk, DiskKind, DiskUsage};

use libc::statvfs;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

/// Copied from [`psutil`]:
///
/// "man iostat" states that sectors are equivalent with blocks and have
/// a size of 512 bytes. Despite this value can be queried at runtime
/// via /sys/block/{DISK}/queue/hw_sector_size and results may vary
/// between 1k, 2k, or 4k... 512 appears to be a magic constant used
/// throughout Linux source code:
/// * <https://stackoverflow.com/a/38136179/376587>
/// * <https://lists.gt.net/linux/kernel/2241060>
/// * <https://github.com/giampaolo/psutil/issues/1305>
/// * <https://github.com/torvalds/linux/blob/4f671fe2f9523a1ea206f63fe60a7c7b3a56d5c7/include/linux/bio.h#L99>
/// * <https://lkml.org/lkml/2015/8/17/234>
///
/// [`psutil]: <https://github.com/giampaolo/psutil/blob/master/psutil/_pslinux.py#L103>
const SECTOR_SIZE: u64 = 512;

macro_rules! cast {
    ($x:expr) => {
        u64::from($x)
    };
}

pub(crate) struct DiskInner {
    type_: DiskKind,
    device_name: OsString,
    actual_device_name: String,
    file_system: OsString,
    mount_point: PathBuf,
    total_space: u64,
    available_space: u64,
    is_removable: bool,
    is_read_only: bool,
    old_written_bytes: u64,
    old_read_bytes: u64,
    written_bytes: u64,
    read_bytes: u64,
}

impl DiskInner {
    pub(crate) fn kind(&self) -> DiskKind {
        self.type_
    }

    pub(crate) fn name(&self) -> &OsStr {
        &self.device_name
    }

    pub(crate) fn file_system(&self) -> &OsStr {
        &self.file_system
    }

    pub(crate) fn mount_point(&self) -> &Path {
        &self.mount_point
    }

    pub(crate) fn total_space(&self) -> u64 {
        self.total_space
    }

    pub(crate) fn available_space(&self) -> u64 {
        self.available_space
    }

    pub(crate) fn is_removable(&self) -> bool {
        self.is_removable
    }

    pub(crate) fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    pub(crate) fn refresh(&mut self) -> bool {
        self.efficient_refresh(None)
    }

    fn efficient_refresh(&mut self, procfs_disk_stats: Option<&[procfs::DiskStat]>) -> bool {
        let Some((read_bytes, written_bytes)) = procfs_disk_stats
            .or(procfs::diskstats().ok().as_deref())
            .unwrap_or_default()
            .iter()
            .find_map(|stat| {
                if stat.name != self.actual_device_name {
                    return None;
                }

                Some((
                    stat.sectors_read * SECTOR_SIZE,
                    stat.sectors_written * SECTOR_SIZE,
                ))
            })
        else {
            sysinfo_debug!("Failed to update disk i/o stats");
            return false;
        };

        self.old_read_bytes = self.read_bytes;
        self.old_written_bytes = self.written_bytes;
        self.read_bytes = read_bytes;
        self.written_bytes = written_bytes;

        unsafe {
            let mut stat: statvfs = mem::zeroed();
            let mount_point_cpath = to_cpath(&self.mount_point);
            if retry_eintr!(statvfs(mount_point_cpath.as_ptr() as *const _, &mut stat)) == 0 {
                let tmp = cast!(stat.f_bsize).saturating_mul(cast!(stat.f_bavail));
                self.available_space = cast!(tmp);
                true
            } else {
                false
            }
        }
    }

    pub(crate) fn usage(&self) -> DiskUsage {
        DiskUsage {
            read_bytes: self.read_bytes.saturating_sub(self.old_read_bytes),
            total_read_bytes: self.read_bytes,
            written_bytes: self.written_bytes.saturating_sub(self.old_written_bytes),
            total_written_bytes: self.written_bytes,
        }
    }
}

impl crate::DisksInner {
    pub(crate) fn new() -> Self {
        Self {
            disks: Vec::with_capacity(2),
        }
    }

    pub(crate) fn refresh_list(&mut self) {
        get_all_list(
            &mut self.disks,
            &get_all_utf8_data("/proc/mounts", 16_385).unwrap_or_default(),
        )
    }

    pub(crate) fn refresh(&mut self) {
        let procfs_disk_stats = procfs::diskstats().ok();
        for disk in self.list_mut() {
            disk.inner.efficient_refresh(procfs_disk_stats.as_deref());
        }
    }

    pub(crate) fn list(&self) -> &[Disk] {
        &self.disks
    }

    pub(crate) fn list_mut(&mut self) -> &mut [Disk] {
        &mut self.disks
    }
}

/// Resolves the actual device name for a specified `device` from `/proc/mounts`
///
/// This function is inspired by the [`bottom`] crate implementation and essentially does the following:
///     1. Canonicalizes the specified device path to its absolute form
///     2. Strips the "/dev" prefix from the canonicalized path
///
/// [`bottom`]: <https://github.com/ClementTsang/bottom/blob/main/src/data_collection/disks/unix/linux/partition.rs#L44>
fn get_actual_device_name(device: &OsStr) -> String {
    let device_path = PathBuf::from(device);

    std::fs::canonicalize(&device_path)
        .ok()
        .and_then(|path| path.strip_prefix("/dev").ok().map(Path::to_path_buf))
        .unwrap_or(device_path)
        .to_str()
        .map(str::to_owned)
        .unwrap_or_default()
}

fn new_disk(
    device_name: &OsStr,
    mount_point: &Path,
    file_system: &OsStr,
    removable_entries: &[PathBuf],
    procfs_disk_stats: &[procfs::DiskStat],
) -> Option<Disk> {
    let mount_point_cpath = to_cpath(mount_point);
    let type_ = find_type_for_device_name(device_name);
    let mut total = 0;
    let mut available = 0;
    let mut is_read_only = false;
    unsafe {
        let mut stat: statvfs = mem::zeroed();
        if retry_eintr!(statvfs(mount_point_cpath.as_ptr() as *const _, &mut stat)) == 0 {
            let bsize = cast!(stat.f_bsize);
            let blocks = cast!(stat.f_blocks);
            let bavail = cast!(stat.f_bavail);
            total = bsize.saturating_mul(blocks);
            available = bsize.saturating_mul(bavail);
            is_read_only = (stat.f_flag & libc::ST_RDONLY) != 0;
        }
        if total == 0 {
            return None;
        }
        let mount_point = mount_point.to_owned();
        let is_removable = removable_entries
            .iter()
            .any(|e| e.as_os_str() == device_name);

        let actual_device_name = get_actual_device_name(device_name);

        let (read_bytes, written_bytes) = procfs_disk_stats
            .iter()
            .find_map(|stat| {
                if stat.name != actual_device_name {
                    return None;
                }

                Some((
                    stat.sectors_read * SECTOR_SIZE,
                    stat.sectors_written * SECTOR_SIZE,
                ))
            })
            .unwrap_or_default();

        Some(Disk {
            inner: DiskInner {
                type_,
                device_name: device_name.to_owned(),
                actual_device_name,
                file_system: file_system.to_owned(),
                mount_point,
                total_space: cast!(total),
                available_space: cast!(available),
                is_removable,
                is_read_only,
                old_read_bytes: 0,
                old_written_bytes: 0,
                read_bytes,
                written_bytes,
            },
        })
    }
}

#[allow(clippy::manual_range_contains)]
fn find_type_for_device_name(device_name: &OsStr) -> DiskKind {
    // The format of devices are as follows:
    //  - device_name is symbolic link in the case of /dev/mapper/
    //     and /dev/root, and the target is corresponding device under
    //     /sys/block/
    //  - In the case of /dev/sd, the format is /dev/sd[a-z][1-9],
    //     corresponding to /sys/block/sd[a-z]
    //  - In the case of /dev/nvme, the format is /dev/nvme[0-9]n[0-9]p[0-9],
    //     corresponding to /sys/block/nvme[0-9]n[0-9]
    //  - In the case of /dev/mmcblk, the format is /dev/mmcblk[0-9]p[0-9],
    //     corresponding to /sys/block/mmcblk[0-9]
    let device_name_path = device_name.to_str().unwrap_or_default();
    let real_path = fs::canonicalize(device_name).unwrap_or_else(|_| PathBuf::from(device_name));
    let mut real_path = real_path.to_str().unwrap_or_default();
    if device_name_path.starts_with("/dev/mapper/") {
        // Recursively solve, for example /dev/dm-0
        if real_path != device_name_path {
            return find_type_for_device_name(OsStr::new(&real_path));
        }
    } else if device_name_path.starts_with("/dev/sd") || device_name_path.starts_with("/dev/vd") {
        // Turn "sda1" into "sda" or "vda1" into "vda"
        real_path = real_path.trim_start_matches("/dev/");
        real_path = real_path.trim_end_matches(|c| c >= '0' && c <= '9');
    } else if device_name_path.starts_with("/dev/nvme") {
        // Turn "nvme0n1p1" into "nvme0n1"
        real_path = match real_path.find('p') {
            Some(idx) => &real_path["/dev/".len()..idx],
            None => &real_path["/dev/".len()..],
        };
    } else if device_name_path.starts_with("/dev/root") {
        // Recursively solve, for example /dev/mmcblk0p1
        if real_path != device_name_path {
            return find_type_for_device_name(OsStr::new(&real_path));
        }
    } else if device_name_path.starts_with("/dev/mmcblk") {
        // Turn "mmcblk0p1" into "mmcblk0"
        real_path = match real_path.find('p') {
            Some(idx) => &real_path["/dev/".len()..idx],
            None => &real_path["/dev/".len()..],
        };
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
    match get_all_utf8_data(path, 8)
        .unwrap_or_default()
        .trim()
        .parse()
        .ok()
    {
        // The disk is marked as rotational so it's a HDD.
        Some(1) => DiskKind::HDD,
        // The disk is marked as non-rotational so it's very likely a SSD.
        Some(0) => DiskKind::SSD,
        // Normally it shouldn't happen but welcome to the wonderful world of IT! :D
        Some(x) => DiskKind::Unknown(x),
        // The information isn't available...
        None => DiskKind::Unknown(-1),
    }
}

fn get_all_list(container: &mut Vec<Disk>, content: &str) {
    container.clear();
    // The goal of this array is to list all removable devices (the ones whose name starts with
    // "usb-").
    let removable_entries = match fs::read_dir("/dev/disk/by-id/") {
        Ok(r) => r
            .filter_map(|res| Some(res.ok()?.path()))
            .filter_map(|e| {
                if e.file_name()
                    .and_then(|x| Some(x.to_str()?.starts_with("usb-")))
                    .unwrap_or_default()
                {
                    e.canonicalize().ok()
                } else {
                    None
                }
            })
            .collect::<Vec<PathBuf>>(),
        _ => Vec::new(),
    };

    let procfs_disk_stats = procfs::diskstats().unwrap_or_default();

    for disk in content
        .lines()
        .map(|line| {
            let line = line.trim_start();
            // mounts format
            // http://man7.org/linux/man-pages/man5/fstab.5.html
            // fs_spec<tab>fs_file<tab>fs_vfstype<tab>other fields
            let mut fields = line.split_whitespace();
            let fs_spec = fields.next().unwrap_or("");
            let fs_file = fields
                .next()
                .unwrap_or("")
                .replace("\\134", "\\")
                .replace("\\040", " ")
                .replace("\\011", "\t")
                .replace("\\012", "\n");
            let fs_vfstype = fields.next().unwrap_or("");
            (fs_spec, fs_file, fs_vfstype)
        })
        .filter(|(fs_spec, fs_file, fs_vfstype)| {
            // Check if fs_vfstype is one of our 'ignored' file systems.
            let filtered = match *fs_vfstype {
                "rootfs" | // https://www.kernel.org/doc/Documentation/filesystems/ramfs-rootfs-initramfs.txt
                "sysfs" | // pseudo file system for kernel objects
                "proc" |  // another pseudo file system
                "devtmpfs" |
                "cgroup" |
                "cgroup2" |
                "pstore" | // https://www.kernel.org/doc/Documentation/ABI/testing/pstore
                "squashfs" | // squashfs is a compressed read-only file system (for snaps)
                "rpc_pipefs" | // The pipefs pseudo file system service
                "iso9660" // optical media
                => true,
                "tmpfs" => !cfg!(feature = "linux-tmpfs"),
                // calling statvfs on a mounted CIFS or NFS may hang, when they are mounted with option: hard
                "cifs" | "nfs" | "nfs4" => !cfg!(feature = "linux-netdevs"),
                _ => false,
            };

            !(filtered ||
               fs_file.starts_with("/sys") || // check if fs_file is an 'ignored' mount point
               fs_file.starts_with("/proc") ||
               (fs_file.starts_with("/run") && !fs_file.starts_with("/run/media")) ||
               fs_spec.starts_with("sunrpc"))
        })
        .filter_map(|(fs_spec, fs_file, fs_vfstype)| {
            new_disk(
                fs_spec.as_ref(),
                Path::new(&fs_file),
                fs_vfstype.as_ref(),
                &removable_entries,
                &procfs_disk_stats,
            )
        })
    {
        container.push(disk);
    }
}

// #[test]
// fn check_all_list() {
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
