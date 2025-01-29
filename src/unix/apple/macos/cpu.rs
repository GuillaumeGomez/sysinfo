// Take a look at the license at the top of the repository in the LICENSE file.

#[cfg(feature = "apple-sandbox")]
pub(crate) unsafe fn get_cpu_frequency() -> u64 {
    0
}

#[cfg(not(feature = "apple-sandbox"))]
pub(crate) unsafe fn get_cpu_frequency() -> u64 {
    use crate::sys::ffi;
    use crate::sys::macos::utils::IOReleaser;
    use objc2_core_foundation::{
        kCFAllocatorDefault, CFData, CFDataGetBytes, CFDataGetLength, CFRange, CFRetained, CFString,
    };

    let Some(matching) = ffi::IOServiceMatching(b"AppleARMIODevice\0".as_ptr() as *const _) else {
        sysinfo_debug!("IOServiceMatching call failed, `AppleARMIODevice` not found");
        return 0;
    };

    // Starting from mac M1, the above call returns nothing for the CPU frequency
    // so we try to get it from another source. This code comes from
    // <https://github.com/giampaolo/psutil/pull/2222>.
    let mut iterator: ffi::io_iterator_t = 0;
    let result =
        ffi::IOServiceGetMatchingServices(ffi::kIOMasterPortDefault, matching, &mut iterator);
    if result != ffi::KIO_RETURN_SUCCESS {
        sysinfo_debug!("Error: IOServiceGetMatchingServices() = {}", result);
        return 0;
    }
    let iterator = match IOReleaser::new(iterator) {
        Some(i) => i,
        None => {
            sysinfo_debug!(
                "Error: IOServiceGetMatchingServices() succeeded but returned invalid descriptor"
            );
            return 0;
        }
    };

    let mut name: ffi::io_name = std::mem::zeroed();
    let entry = loop {
        let entry = match IOReleaser::new(ffi::IOIteratorNext(iterator.inner())) {
            Some(d) => d,
            None => {
                sysinfo_debug!("`pmgr` entry was not found in AppleARMIODevice service");
                return 0;
            }
        };
        let status = ffi::IORegistryEntryGetName(entry.inner(), name.as_mut_ptr());
        if status != libc::KERN_SUCCESS {
            continue;
        } else if libc::strcmp(name.as_ptr(), b"pmgr\0".as_ptr() as *const _) == 0 {
            break entry;
        }
    };

    let node_name = CFString::from_static_str("voltage-states5-sram");

    let core_ref = match ffi::IORegistryEntryCreateCFProperty(
        entry.inner(),
        &node_name,
        kCFAllocatorDefault,
        0,
    ) {
        Some(c) => c,
        None => {
            sysinfo_debug!("`voltage-states5-sram` property not found");
            return 0;
        }
    };
    let core_ref = CFRetained::from_raw(core_ref);

    let Ok(core_ref) = core_ref.downcast::<CFData>() else {
        sysinfo_debug!("`voltage-states5-sram` property was not CFData");
        return 0;
    };

    let core_length = CFDataGetLength(&core_ref);
    if core_length < 8 {
        sysinfo_debug!("expected `voltage-states5-sram` buffer to have at least size 8");
        return 0;
    }
    let mut max: u64 = 0;
    CFDataGetBytes(
        &core_ref,
        CFRange {
            location: core_length - 8,
            length: 4,
        },
        &mut max as *mut _ as *mut _,
    );
    max / 1_000_000
}
