// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::{ffi, utils};
use crate::utils::to_cpath;
use crate::{Disk, DiskType};

use core_foundation_sys::base::{kCFAllocatorDefault, kCFAllocatorNull, CFRelease};
use core_foundation_sys::dictionary::{CFDictionaryGetValueIfPresent, CFDictionaryRef};
use core_foundation_sys::number::{kCFBooleanTrue, CFBooleanRef};
use core_foundation_sys::string as cfs;

use libc::{c_char, c_int, c_void, statfs};

use std::ffi::{OsStr, OsString};
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::ptr;

fn to_path(mount_path: &[c_char]) -> Option<PathBuf> {
    let mut tmp = Vec::with_capacity(mount_path.len());
    for &c in mount_path {
        if c == 0 {
            break;
        }
        tmp.push(c as u8);
    }
    if tmp.is_empty() {
        None
    } else {
        let path = OsStr::from_bytes(&tmp);
        Some(PathBuf::from(path))
    }
}

#[repr(transparent)]
struct CFReleaser<T>(*const T);

impl<T> CFReleaser<T> {
    fn new(ptr: *const T) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(Self(ptr))
        }
    }

    fn inner(&self) -> *const T {
        self.0
    }
}

impl<T> Drop for CFReleaser<T> {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0 as _) }
        }
    }
}

pub(crate) fn get_disks(session: ffi::DASessionRef) -> Vec<Disk> {
    if session.is_null() {
        return Vec::new();
    }
    unsafe {
        let count = libc::getfsstat(ptr::null_mut(), 0, libc::MNT_NOWAIT);
        if count < 1 {
            return Vec::new();
        }
        let bufsize = count * mem::size_of::<libc::statfs>() as c_int;
        let mut disks = Vec::with_capacity(count as _);
        let count = libc::getfsstat(disks.as_mut_ptr(), bufsize, libc::MNT_NOWAIT);
        if count < 1 {
            return Vec::new();
        }
        disks.set_len(count as _);
        disks
            .into_iter()
            .filter_map(|c_disk| {
                let mount_point = to_path(&c_disk.f_mntonname)?;
                let disk = CFReleaser::new(ffi::DADiskCreateFromBSDName(
                    kCFAllocatorDefault as _,
                    session,
                    c_disk.f_mntfromname.as_ptr(),
                ))?;
                let dict = CFReleaser::new(ffi::DADiskCopyDescription(disk.inner()))?;
                // Keeping this around in case one might want the list of the available
                // keys in "dict".
                // core_foundation_sys::base::CFShow(dict as _);
                let name = match get_str_value(dict.inner(), b"DAMediaName\0").map(OsString::from) {
                    Some(n) => n,
                    None => return None,
                };
                let removable =
                    get_bool_value(dict.inner(), b"DAMediaRemovable\0").unwrap_or(false);
                let ejectable =
                    get_bool_value(dict.inner(), b"DAMediaEjectable\0").unwrap_or(false);
                // This is very hackish but still better than nothing...
                let type_ = if let Some(model) = get_str_value(dict.inner(), b"DADeviceModel\0") {
                    if model.contains("SSD") {
                        DiskType::SSD
                    } else {
                        // We just assume by default that this is a HDD
                        DiskType::HDD
                    }
                } else {
                    DiskType::Unknown(-1)
                };

                new_disk(name, mount_point, type_, removable || ejectable)
            })
            .collect::<Vec<_>>()
    }
}

unsafe fn get_dict_value<T, F: FnOnce(*const c_void) -> Option<T>>(
    dict: CFDictionaryRef,
    key: &[u8],
    callback: F,
) -> Option<T> {
    let key = ffi::CFStringCreateWithCStringNoCopy(
        ptr::null_mut(),
        key.as_ptr() as *const c_char,
        cfs::kCFStringEncodingUTF8,
        kCFAllocatorNull as _,
    );
    let mut value = std::ptr::null();
    let ret = if CFDictionaryGetValueIfPresent(dict, key as _, &mut value) != 0 {
        callback(value)
    } else {
        None
    };
    CFRelease(key as _);
    ret
}

unsafe fn get_str_value(dict: CFDictionaryRef, key: &[u8]) -> Option<String> {
    get_dict_value(dict, key, |v| {
        let v = v as cfs::CFStringRef;

        let len_utf16 = cfs::CFStringGetLength(v);
        let len_bytes = len_utf16 as usize * 2; // Two bytes per UTF-16 codepoint.

        let v_ptr = cfs::CFStringGetCStringPtr(v, cfs::kCFStringEncodingUTF8);
        if v_ptr.is_null() {
            // Fallback on CFStringGetString to read the underlying bytes from the CFString.
            let mut buf = vec![0; len_bytes];
            let success = cfs::CFStringGetCString(
                v,
                buf.as_mut_ptr(),
                len_bytes as _,
                cfs::kCFStringEncodingUTF8,
            );

            if success != 0 {
                utils::vec_to_rust(buf)
            } else {
                None
            }
        } else {
            utils::cstr_to_rust_with_size(v_ptr, Some(len_bytes))
        }
    })
}

unsafe fn get_bool_value(dict: CFDictionaryRef, key: &[u8]) -> Option<bool> {
    get_dict_value(dict, key, |v| Some(v as CFBooleanRef == kCFBooleanTrue))
}

fn new_disk(
    name: OsString,
    mount_point: PathBuf,
    type_: DiskType,
    is_removable: bool,
) -> Option<Disk> {
    let mount_point_cpath = to_cpath(&mount_point);
    let mut total_space = 0;
    let mut available_space = 0;
    let mut file_system = None;
    unsafe {
        let mut stat: statfs = mem::zeroed();
        if statfs(mount_point_cpath.as_ptr() as *const i8, &mut stat) == 0 {
            // APFS is "special" because its a snapshot-based filesystem, and modern
            // macOS devices take full advantage of this.
            //
            // By default, listing volumes with `statfs` can return both the root-level
            // "data" partition and any snapshots that exist. However, other than some flags and
            // reserved(undocumented) bytes, there is no difference between the OS boot snapshot
            // and the "data" partition.
            //
            // To avoid duplicating the number of disks (and therefore available space, etc), only return
            // a disk (which is really a partition with APFS) if it is the root of the filesystem.
            let is_root = stat.f_flags & libc::MNT_ROOTFS as u32 == 0;
            if !is_root {
                return None;
            }

            total_space = u64::from(stat.f_bsize).saturating_mul(stat.f_blocks);
            available_space = u64::from(stat.f_bsize).saturating_mul(stat.f_bavail);
            let mut vec = Vec::with_capacity(stat.f_fstypename.len());
            for x in &stat.f_fstypename {
                if *x == 0 {
                    break;
                }
                vec.push(*x as u8);
            }
            file_system = Some(vec);
        }
        if total_space == 0 {
            return None;
        }
        Some(Disk {
            type_,
            name,
            file_system: file_system.unwrap_or_else(|| b"<Unknown>".to_vec()),
            mount_point,
            total_space,
            available_space,
            is_removable,
        })
    }
}
