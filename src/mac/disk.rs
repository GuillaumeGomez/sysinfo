//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use sys::utils;
use utils::to_cpath;
use DiskExt;
use DiskType;

use libc::{c_char, c_void, statfs, strlen, PATH_MAX};
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
// use std::fs;
use std::mem;
use std::mem::MaybeUninit;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr;
use sys::ffi;

use core_foundation::url::{CFURLRef, CFURLGetFileSystemRepresentation};

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

static DISK_TYPES: once_cell::sync::Lazy<HashMap<OsString, DiskType>> =
    once_cell::sync::Lazy::new(get_disk_types);

struct DASessionRefWrap(ffi::DASessionRef);

unsafe impl Sync for DASessionRefWrap {}
unsafe impl Send for DASessionRefWrap {}

static SESSION: once_cell::sync::Lazy<DASessionRefWrap> = once_cell::sync::Lazy::new(|| 
    unsafe { DASessionRefWrap(ffi::DASessionCreate(ffi::kCFAllocatorDefault)) });

fn get_disk_types() -> HashMap<OsString, DiskType> {
    let mut master_port: ffi::mach_port_t = 0;
    let mut media_iterator: ffi::io_iterator_t = 0;
    let mut ret = HashMap::with_capacity(1);

    unsafe {
        ffi::IOMasterPort(ffi::MACH_PORT_NULL, &mut master_port);

        let matching_dictionary = ffi::IOServiceMatching(b"IOMedia\0".as_ptr() as *const i8);
        let result = ffi::IOServiceGetMatchingServices(
            master_port,
            matching_dictionary,
            &mut media_iterator,
        );
        if result != ffi::KERN_SUCCESS as i32 {
            sysinfo_debug!("Error: IOServiceGetMatchingServices() = {}", result);
            return ret;
        }

        loop {
            let next_media = ffi::IOIteratorNext(media_iterator);
            if next_media == 0 {
                break;
            }
            let mut props = MaybeUninit::<ffi::CFMutableDictionaryRef>::uninit();
            let result = ffi::IORegistryEntryCreateCFProperties(
                next_media,
                props.as_mut_ptr(),
                ffi::kCFAllocatorDefault,
                0,
            );
            let props = props.assume_init();
            if result == ffi::KERN_SUCCESS as i32 && check_value(props, b"Whole\0") {
                let mut name: ffi::io_name_t = mem::zeroed();
                if ffi::IORegistryEntryGetName(next_media, name.as_mut_ptr() as *mut c_char)
                    == ffi::KERN_SUCCESS as i32
                {
                    ret.insert(
                        make_name(&name),
                        if check_value(props, b"RAID\0") {
                            DiskType::Unknown(-1)
                        } else {
                            DiskType::SSD
                        },
                    );
                }
                ffi::CFRelease(props as *mut _);
            }
            ffi::IOObjectRelease(next_media);
        }
        ffi::IOObjectRelease(media_iterator);
    }
    ret
}

fn make_name(v: &[u8]) -> OsString {
    for (pos, x) in v.iter().enumerate() {
        if *x == 0 {
            return OsStringExt::from_vec(v[0..pos].to_vec());
        }
    }
    OsStringExt::from_vec(v.to_vec())
}

unsafe fn to_path(url: CFURLRef) -> Option<PathBuf> {
    let mut buf = [0u8; PATH_MAX as usize];
    let result = CFURLGetFileSystemRepresentation(url, true as u8, buf.as_mut_ptr(), buf.len() as _);
    if result == false as u8 {
        return None;
    }
    let len = strlen(buf.as_ptr() as *const c_char);
    let path = OsStr::from_bytes(&buf[0..len]);
    Some(PathBuf::from(path))
}

pub(crate) fn get_disks() -> Vec<Disk> {
    if SESSION.0.is_null() {
        return Vec::new();
    }
    let arr = unsafe {
        ffi::mountedVolumeURLsIncludingResourceValuesForKeys(std::ptr::null_mut(), 0)
    };
    if arr.is_null() {
        return Vec::new();
    }
    let mut disks = Vec::new();
    for i in 0..unsafe { ffi::NSArrayCount(arr) } {
        let url = unsafe { ffi::NSArrayObjectAtIndex(arr, i) } as CFURLRef;
        if url.is_null() {
            continue;
        }
        if let Some(mount_point) = unsafe { to_path(url) } {
            unsafe {
                let disk = ffi::DADiskCreateFromVolumePath(ffi::kCFAllocatorDefault, SESSION.0, url);
                let name = ffi::DADiskGetBSDName(disk);
                if name.is_null() {
                    continue;
                }
                let name = utils::cstr_to_rust(name);
                if name.is_none() {
                    continue;
                }
                let name = OsString::from(name.unwrap());
                let dict = ffi::DADiskCopyDescription(disk);
                if !dict.is_null() {
                    let removable = check_value(dict, b"DAMediaRemovable\0");
                    let ejectable = check_value(dict, b"DAMediaEjectable\0");

                    let type_ = if !removable && !ejectable {
                        DISK_TYPES
                                .get(&name)
                                .cloned()
                                .unwrap_or(DiskType::Unknown(-2))
                    } else {
                        DiskType::Removable
                    };
                    if let Some(disk) = new_disk(name, mount_point, type_) {
                        disks.push(disk);
                    }
                    ffi::CFRelease(dict as *const c_void);
                }
            }
        }
    }
    unsafe { ffi::CFRelease(arr as *const c_void); }
    disks
    // match fs::read_dir("/Volumes") {
    //     Ok(d) => d
    //         .flat_map(|x| {
    //             if let Ok(ref entry) = x {
    //                 let mount_point = utils::realpath(&entry.path());
    //                 if mount_point.as_os_str().is_empty() {
    //                     None
    //                 } else {
    //                     let name = entry.path().file_name()?.to_owned();
    //                     let type_ = DISK_TYPES
    //                         .get(&name)
    //                         .cloned()
    //                         .unwrap_or(DiskType::Unknown(-2));
    //                     new_disk(name, &mount_point, type_)
    //                 }
    //             } else {
    //                 None
    //             }
    //         })
    //         .collect(),
    //     _ => Vec::new(),
    // }
}

unsafe fn check_value(dict: ffi::CFMutableDictionaryRef, key: &[u8]) -> bool {
    let key = ffi::CFStringCreateWithCStringNoCopy(
        ptr::null_mut(),
        key.as_ptr() as *const c_char,
        ffi::kCFStringEncodingMacRoman,
        ffi::kCFAllocatorNull as *mut c_void,
    );
    let ret = ffi::CFDictionaryContainsKey(dict as ffi::CFDictionaryRef, key as *const c_void) != 0
        && *(ffi::CFDictionaryGetValue(dict as ffi::CFDictionaryRef, key as *const c_void)
            as *const ffi::Boolean)
            != 0;
    ffi::CFRelease(key as *const c_void);
    ret
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
