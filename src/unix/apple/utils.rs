// Take a look at the license at the top of the repository in the LICENSE file.

#[cfg(feature = "system")]
pub(crate) unsafe fn get_sys_value(
    mut len: usize,
    value: *mut libc::c_void,
    mib: &mut [i32],
) -> bool {
    unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as _,
            value,
            &mut len as *mut _,
            std::ptr::null_mut(),
            0,
        ) == 0
    }
}

#[cfg(feature = "system")]
pub(crate) unsafe fn get_sys_value_by_name(
    name: &[u8],
    len: &mut usize,
    value: *mut libc::c_void,
) -> bool {
    unsafe {
        libc::sysctlbyname(
            name.as_ptr() as *const _,
            value,
            len,
            std::ptr::null_mut(),
            0,
        ) == 0
    }
}

cfg_select! {
    any(
        feature = "gpu",
        all(feature = "disk", target_os = "macos"),
        all(feature = "system", target_os = "macos", not(feature = "apple-sandbox")),
        all(
            target_os = "macos",
            not(feature = "apple-sandbox"),
            feature = "component",
            any(target_arch = "x86", target_arch = "x86_64"),
        )
    ) => {
        type IoObject = std::num::NonZeroU32;

        pub(crate) struct IOReleaser(IoObject);

        impl IOReleaser {
            pub(crate) fn new(obj: u32) -> Option<Self> {
                IoObject::new(obj).map(Self)
            }

            #[cfg(feature = "disk")]
            #[cfg(target_os = "macos")]
            pub(crate) unsafe fn new_unchecked(obj: u32) -> Self {
                // Chance at catching in-development mistakes
                debug_assert_ne!(obj, 0);
                unsafe { Self(IoObject::new_unchecked(obj)) }
            }

            #[inline]
            pub(crate) fn inner(&self) -> u32 {
                self.0.get()
            }
        }

        impl Drop for IOReleaser {
            fn drop(&mut self) {
                objc2_io_kit::IOObjectRelease(self.0.get() as _);
            }
        }

        // Use kIOMasterPortDefault on macOS to support older OS versions.
        #[allow(deprecated)]
        #[cfg(target_os = "macos")]
        pub(crate) static MAIN_PORT: &libc::mach_port_t = unsafe { &objc2_io_kit::kIOMasterPortDefault };

        // iOS, watchOS, tvOS and visionOS only have the newer symbol.
        #[cfg(not(target_os = "macos"))]
        pub(crate) static MAIN_PORT: &libc::mach_port_t = unsafe { &objc2_io_kit::kIOMainPortDefault };
    }
    _ => {}
}
