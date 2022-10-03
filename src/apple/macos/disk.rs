// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::macos::utils::{CFReleaser, IOReleaser};
use crate::sys::{ffi, utils};
use crate::{Disk, DiskType};

use core_foundation_sys::array::CFArrayCreate;
use core_foundation_sys::base::{kCFAllocatorDefault, kCFAllocatorNull};
use core_foundation_sys::dictionary::{CFDictionaryGetValueIfPresent, CFDictionaryRef};
use core_foundation_sys::number::{kCFBooleanTrue, CFBooleanRef, CFNumberGetValue};
use core_foundation_sys::string::{self as cfs, CFStringRef};

use libc::{c_void, statfs};

use std::ffi::{CStr, OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::ptr;

pub(crate) fn get_disks() -> Vec<Disk> {
    let raw_disks = unsafe {
        let count = libc::getfsstat(ptr::null_mut(), 0, libc::MNT_NOWAIT);
        if count < 1 {
            return Vec::new();
        }
        let bufsize = count * std::mem::size_of::<libc::statfs>() as libc::c_int;
        let mut disks = Vec::with_capacity(count as _);
        let count = libc::getfsstat(disks.as_mut_ptr(), bufsize, libc::MNT_NOWAIT);

        if count < 1 {
            return Vec::new();
        }

        disks.set_len(count as usize);

        disks
    };

    // Create a list of properties about the disk that we want to fetch.
    let requested_properties = unsafe {
        [
            ffi::kCFURLVolumeIsEjectableKey,
            ffi::kCFURLVolumeIsRemovableKey,
            ffi::kCFURLVolumeTotalCapacityKey,
            ffi::kCFURLVolumeAvailableCapacityForImportantUsageKey,
            ffi::kCFURLVolumeAvailableCapacityKey,
            ffi::kCFURLVolumeNameKey,
            ffi::kCFURLVolumeIsBrowsableKey,
            ffi::kCFURLVolumeIsLocalKey,
        ]
    };
    let requested_properties = CFReleaser::new(unsafe {
        CFArrayCreate(
            ptr::null_mut(),
            requested_properties.as_ptr() as *const *const c_void,
            requested_properties.len() as isize,
            &core_foundation_sys::array::kCFTypeArrayCallBacks,
        )
    })
    .unwrap();

    let mut disks = Vec::with_capacity(raw_disks.len());
    for c_disk in raw_disks {
        let volume_url = CFReleaser::new(unsafe {
            core_foundation_sys::url::CFURLCreateFromFileSystemRepresentation(
                kCFAllocatorDefault,
                c_disk.f_mntonname.as_ptr() as *const u8,
                c_disk.f_mntonname.len() as isize,
                false as u8,
            )
        })
        .unwrap();

        let prop_dict = match CFReleaser::new(unsafe {
            ffi::CFURLCopyResourcePropertiesForKeys(
                volume_url.inner(),
                requested_properties.inner(),
                ptr::null_mut(),
            )
        }) {
            Some(props) => props,
            None => continue,
        };

        let browsable = unsafe {
            get_bool_value(
                prop_dict.inner(),
                DictKey::Extern(ffi::kCFURLVolumeIsBrowsableKey),
            )
            .unwrap_or_default()
        };

        // Do not return invisible "disks." Most of the time, these are APFS snapshots, hidden
        // system volumes, etc. Browsable is defined to be visible in the system's UI like Finder,
        // disk utility, system information, etc.
        //
        // To avoid seemingly duplicating many disks and creating an inaccurate view of the system's resources,
        // these are skipped entirely.
        if !browsable {
            continue;
        }

        let local_only = unsafe {
            get_bool_value(
                prop_dict.inner(),
                DictKey::Extern(ffi::kCFURLVolumeIsLocalKey),
            )
            .unwrap_or(true)
        };

        // Skip any drive that is not locally attached to the system.
        //
        // This includes items like SMB mounts, and matches the other platform's behavior.
        if !local_only {
            continue;
        }

        let mount_point = PathBuf::from(OsStr::from_bytes(
            unsafe { CStr::from_ptr(c_disk.f_mntonname.as_ptr()) }.to_bytes(),
        ));

        disks.extend(new_disk(mount_point, c_disk, &prop_dict))
    }

    disks
}

const UNKNOWN_DISK_TYPE: DiskType = DiskType::Unknown(-1);

fn get_disk_type(disk: &statfs) -> DiskType {
    let characteristics_string = CFReleaser::new(unsafe {
        cfs::CFStringCreateWithBytesNoCopy(
            kCFAllocatorDefault,
            ffi::kIOPropertyDeviceCharacteristicsKey.as_ptr(),
            ffi::kIOPropertyDeviceCharacteristicsKey.len() as _,
            cfs::kCFStringEncodingUTF8,
            false as _,
            kCFAllocatorNull,
        )
    })
    .unwrap();

    // Removes `/dev/` from the value.
    let bsd_name = unsafe {
        CStr::from_ptr(disk.f_mntfromname.as_ptr())
            .to_bytes()
            .strip_prefix(b"/dev/")
            .expect("device mount point in unknown format")
    };

    let matching =
        unsafe { ffi::IOBSDNameMatching(ffi::kIOMasterPortDefault, 0, bsd_name.as_ptr().cast()) };

    if matching.is_null() {
        return UNKNOWN_DISK_TYPE;
    }

    let mut service_iterator: ffi::io_iterator_t = 0;

    if unsafe {
        ffi::IOServiceGetMatchingServices(
            ffi::kIOMasterPortDefault,
            matching.cast(),
            &mut service_iterator,
        )
    } != libc::KERN_SUCCESS
    {
        return UNKNOWN_DISK_TYPE;
    }

    let service_iterator = IOReleaser::new(service_iterator).unwrap();

    let mut parent_entry: ffi::io_registry_entry_t = 0;

    while let Some(mut current_service_entry) =
        IOReleaser::new(unsafe { ffi::IOIteratorNext(service_iterator.inner()) })
    {
        // Note: This loop is required in a non-obvious way. Due to device properties existing as a tree
        // in IOKit, we may need an arbitrary number of calls to `IORegistryEntryCreateCFProperty` in order to find
        // the values we are looking for. The function may return nothing if we aren't deep enough into the registry
        // tree, so we need to continue going from child->parent node until its found.
        loop {
            let result = unsafe {
                ffi::IORegistryEntryGetParentEntry(
                    current_service_entry.inner(),
                    ffi::kIOServicePlane.as_ptr().cast(),
                    &mut parent_entry,
                )
            };
            if result != libc::KERN_SUCCESS {
                break;
            }

            current_service_entry = IOReleaser::new(parent_entry).unwrap();

            // There were no more parents left.
            if parent_entry == 0 {
                break;
            }

            let properties_result = unsafe {
                CFReleaser::new(ffi::IORegistryEntryCreateCFProperty(
                    current_service_entry.inner(),
                    characteristics_string.inner(),
                    kCFAllocatorDefault,
                    0,
                ))
            };

            if let Some(device_properties) = properties_result {
                let disk_type = unsafe {
                    get_str_value(
                        device_properties.inner(),
                        DictKey::Defined(ffi::kIOPropertyMediumTypeKey),
                    )
                };

                if let Some(disk_type) = disk_type.and_then(|medium| match medium.as_str() {
                    _ if medium == ffi::kIOPropertyMediumTypeSolidStateKey => Some(DiskType::SSD),
                    _ if medium == ffi::kIOPropertyMediumTypeRotationalKey => Some(DiskType::HDD),
                    _ => None,
                }) {
                    return disk_type;
                } else {
                    // Many external drive vendors do not advertise their device's storage medium.
                    //
                    // In these cases, assuming that there were _any_ properties about them registered, we fallback
                    // to `HDD` when no storage medium is provided by the device instead of `Unknown`.
                    return DiskType::HDD;
                }
            }
        }
    }

    UNKNOWN_DISK_TYPE
}

enum DictKey {
    Extern(CFStringRef),
    Defined(&'static str),
}

unsafe fn get_dict_value<T, F: FnOnce(*const c_void) -> Option<T>>(
    dict: CFDictionaryRef,
    key: DictKey,
    callback: F,
) -> Option<T> {
    let _defined;
    let key = match key {
        DictKey::Extern(val) => val,
        DictKey::Defined(val) => {
            _defined = CFReleaser::new(cfs::CFStringCreateWithBytesNoCopy(
                kCFAllocatorDefault,
                val.as_ptr(),
                val.len() as isize,
                cfs::kCFStringEncodingUTF8,
                false as _,
                kCFAllocatorNull,
            ))
            .unwrap();

            _defined.inner()
        }
    };

    let mut value = std::ptr::null();
    if CFDictionaryGetValueIfPresent(dict, key.cast(), &mut value) != 0 {
        callback(value)
    } else {
        None
    }
}

unsafe fn get_str_value(dict: CFDictionaryRef, key: DictKey) -> Option<String> {
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

unsafe fn get_bool_value(dict: CFDictionaryRef, key: DictKey) -> Option<bool> {
    get_dict_value(dict, key, |v| Some(v as CFBooleanRef == kCFBooleanTrue))
}

unsafe fn get_int_value(dict: CFDictionaryRef, key: DictKey) -> Option<i64> {
    get_dict_value(dict, key, |v| {
        let mut val: i64 = 0;
        if CFNumberGetValue(
            v.cast(),
            core_foundation_sys::number::kCFNumberSInt64Type,
            &mut val as *mut i64 as *mut c_void,
        ) {
            Some(val)
        } else {
            None
        }
    })
}

fn new_disk(
    mount_point: PathBuf,
    c_disk: statfs,
    disk_props: &CFReleaser<core_foundation_sys::dictionary::__CFDictionary>,
) -> Option<Disk> {
    let type_ = get_disk_type(&c_disk);

    let name = unsafe {
        get_str_value(
            disk_props.inner(),
            DictKey::Extern(ffi::kCFURLVolumeNameKey),
        )
    }
    .map(OsString::from)
    .unwrap();

    let is_removable = unsafe {
        let ejectable = get_bool_value(
            disk_props.inner(),
            DictKey::Extern(ffi::kCFURLVolumeIsEjectableKey),
        )
        .unwrap();

        let removable = get_bool_value(
            disk_props.inner(),
            DictKey::Extern(ffi::kCFURLVolumeIsRemovableKey),
        )
        .unwrap();

        ejectable || removable
    };

    let total_space = unsafe {
        get_int_value(
            disk_props.inner(),
            DictKey::Extern(ffi::kCFURLVolumeTotalCapacityKey),
        )
    }
    .unwrap() as u64;

    // We prefer `AvailableCapacityForImportantUsage` over `AvailableCapacity` because
    // it takes more of the system's properties into account, like the trash, system-managed caches,
    // etc. It generally also returns higher values too, because of the above, so its a more accurate
    // representation of what the system _could_ still use.
    let available_space = unsafe {
        get_int_value(
            disk_props.inner(),
            DictKey::Extern(ffi::kCFURLVolumeAvailableCapacityForImportantUsageKey),
        )
        .filter(|bytes| *bytes != 0)
        .or_else(|| {
            get_int_value(
                disk_props.inner(),
                DictKey::Extern(ffi::kCFURLVolumeAvailableCapacityKey),
            )
        })
    }
    .unwrap() as u64;

    let file_system = IntoIterator::into_iter(c_disk.f_fstypename)
        .filter_map(|b| if b != 0 { Some(b as u8) } else { None })
        .collect();

    Some(Disk {
        type_,
        name,
        file_system,
        mount_point,
        total_space,
        available_space,
        is_removable,
    })
}
