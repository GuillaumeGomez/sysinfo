//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use sys::utils;
use utils::to_cpath;
use DiskExt;
use DiskType;

use core_foundation::boolean as cfb;
use core_foundation::string as cfs;
use core_foundation::url::{CFURLGetFileSystemRepresentation, CFURLRef};
use libc::{c_char, c_void, statfs, strlen, PATH_MAX};
use std::ffi::{OsStr, OsString};
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;
use sys::ffi;

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
            let mut stat: statfs = mem::zeroed();
            let mount_point_cpath = to_cpath(&self.mount_point);
            if statfs(mount_point_cpath.as_ptr() as *const i8, &mut stat) == 0 {
                self.available_space = u64::from(stat.f_bsize) * stat.f_bavail;
                true
            } else {
                false
            }
        }
    }
}

unsafe fn to_path(url: CFURLRef) -> Option<PathBuf> {
    let mut buf = [0u8; PATH_MAX as usize];
    let result =
        CFURLGetFileSystemRepresentation(url, true as u8, buf.as_mut_ptr(), buf.len() as _);
    if result == false as u8 {
        return None;
    }
    let len = strlen(buf.as_ptr() as *const c_char);
    let path = OsStr::from_bytes(&buf[0..len]);
    Some(PathBuf::from(path))
}

pub(crate) fn get_disks(session: ffi::DASessionRef) -> Vec<Disk> {
    if session.is_null() {
        return Vec::new();
    }
    let arr = unsafe { ffi::macos_get_disks() };
    if arr.is_null() {
        return Vec::new();
    }
    let mut disks = Vec::new();
    for i in 0..unsafe { ffi::CFArrayGetCount(arr) } {
        let url = unsafe { ffi::CFArrayGetValueAtIndex(arr, i) } as CFURLRef;
        if url.is_null() {
            continue;
        }
        if let Some(mount_point) = unsafe { to_path(url) } {
            unsafe {
                let disk = ffi::DADiskCreateFromVolumePath(ffi::kCFAllocatorDefault, session, url);
                let dict = ffi::DADiskCopyDescription(disk);
                if !dict.is_null() {
                    // Keeping this around in case one might want the list of the available
                    // keys in "dict".
                    // ::core_foundation::base::CFShow(dict as _);
                    let name = match get_str_value(dict, b"DAMediaName\0").map(OsString::from) {
                        Some(n) => n,
                        None => continue,
                    };
                    let removable = get_bool_value(dict, b"DAMediaRemovable\0").unwrap_or(false);
                    let ejectable = get_bool_value(dict, b"DAMediaEjectable\0").unwrap_or(false);
                    // This is very hackish but still better than nothing...
                    let type_ = if let Some(model) = get_str_value(dict, b"DADeviceModel\0") {
                        if model.contains("SSD") {
                            DiskType::SSD
                        } else if removable || ejectable {
                            DiskType::Removable
                        } else {
                            // We just assume by default that this is a HDD
                            DiskType::HDD
                        }
                    } else if removable || ejectable {
                        DiskType::Removable
                    } else {
                        DiskType::Unknown(-1)
                    };

                    if let Some(disk) = new_disk(name, mount_point, type_) {
                        disks.push(disk);
                    }
                    ffi::CFRelease(dict as *const c_void);
                }
            }
        }
    }
    unsafe {
        ffi::CFRelease(arr as *const c_void);
    }
    disks
}

unsafe fn get_dict_value<T, F: FnOnce(*const c_void) -> Option<T>>(
    dict: ffi::CFDictionaryRef,
    key: &[u8],
    callback: F,
) -> Option<T> {
    let key = ffi::CFStringCreateWithCStringNoCopy(
        ptr::null_mut(),
        key.as_ptr() as *const c_char,
        ffi::kCFStringEncodingMacRoman,
        ffi::kCFAllocatorNull as *mut c_void,
    );
    let mut value = ::std::ptr::null();
    let ret = if ffi::CFDictionaryGetValueIfPresent(dict, key as _, &mut value) != 0 {
        callback(value)
    } else {
        None
    };
    ffi::CFRelease(key as *const c_void);
    ret
}

unsafe fn get_str_value(dict: ffi::CFDictionaryRef, key: &[u8]) -> Option<String> {
    get_dict_value(dict, key, |v| {
        let v = v as cfs::CFStringRef;
        let len = cfs::CFStringGetLength(v);
        utils::cstr_to_rust_with_size(
            cfs::CFStringGetCStringPtr(v, cfs::kCFStringEncodingUTF8),
            Some(len as _),
        )
    })
}

unsafe fn get_bool_value(dict: ffi::CFDictionaryRef, key: &[u8]) -> Option<bool> {
    get_dict_value(dict, key, |v| {
        Some(v as cfb::CFBooleanRef == cfb::kCFBooleanTrue)
    })
}

fn new_disk(name: OsString, mount_point: PathBuf, type_: DiskType) -> Option<Disk> {
    let mount_point_cpath = to_cpath(&mount_point);
    let mut total_space = 0;
    let mut available_space = 0;
    let mut file_system = None;
    unsafe {
        let mut stat: statfs = mem::zeroed();
        if statfs(mount_point_cpath.as_ptr() as *const i8, &mut stat) == 0 {
            total_space = u64::from(stat.f_bsize) * stat.f_blocks;
            available_space = u64::from(stat.f_bsize) * stat.f_bavail;
            let mut vec = Vec::with_capacity(stat.f_fstypename.len());
            for x in &stat.f_fstypename {
                if *x == 0 {
                    break;
                }
                vec.push(*x as u8);
            }
            file_system = Some(vec);
        }
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
    })
}
