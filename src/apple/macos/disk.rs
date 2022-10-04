// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::ffi;
use crate::sys::{
    disk::{get_str_value, DictKey},
    macos::utils::IOReleaser,
    utils::CFReleaser,
};
use crate::DiskType;

use core_foundation_sys::base::{kCFAllocatorDefault, kCFAllocatorNull};
use core_foundation_sys::string as cfs;

use std::ffi::CStr;

const UNKNOWN_DISK_TYPE: DiskType = DiskType::Unknown(-1);

pub(crate) fn get_disk_type(disk: &libc::statfs) -> DiskType {
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
                    super::disk::get_str_value(
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
