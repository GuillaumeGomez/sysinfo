//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use utils;
use DiskExt;
use DiskType;

use libc::{c_char, c_void, statfs};
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::mem;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStringExt;
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

static DISK_TYPES: once_cell::sync::Lazy<HashMap<OsString, DiskType>> =
    once_cell::sync::Lazy::new(get_disk_types);

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

pub(crate) fn get_disks() -> Vec<Disk> {
    match fs::read_dir("/Volumes") {
        Ok(d) => d
            .flat_map(|x| {
                if let Ok(ref entry) = x {
                    let mount_point = utils::realpath(&entry.path());
                    if mount_point.as_os_str().is_empty() {
                        None
                    } else {
                        let name = entry.path().file_name()?.to_owned();
                        let type_ = DISK_TYPES
                            .get(&name)
                            .cloned()
                            .unwrap_or(DiskType::Unknown(-2));
                        new_disk(name, &mount_point, type_)
                    }
                } else {
                    None
                }
            })
            .collect(),
        _ => Vec::new(),
    }
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

fn new_disk(name: OsString, mount_point: &Path, type_: DiskType) -> Option<Disk> {
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
        mount_point: mount_point.to_owned(),
        total_space,
        available_space,
    })
}
