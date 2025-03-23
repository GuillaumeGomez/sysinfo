// Take a look at the license at the top of the repository in the LICENSE file.

// Reexport items defined in either macos or ios ffi module.
#[cfg(all(
    not(target_os = "ios"),
    any(
        feature = "disk",
        all(
            not(feature = "apple-sandbox"),
            any(feature = "component", feature = "system")
        ),
    ),
))]
pub use crate::sys::inner::ffi::*;

#[cfg(feature = "disk")]
#[link(name = "objc", kind = "dylib")]
extern "C" {
    pub fn objc_autoreleasePoolPop(pool: *mut libc::c_void);
    pub fn objc_autoreleasePoolPush() -> *mut libc::c_void;
}

#[cfg_attr(feature = "debug", derive(Eq, Hash, PartialEq))]
#[allow(unused)]
#[allow(non_camel_case_types)]
#[derive(Clone)]
#[repr(C)]
pub struct Val_t {
    pub key: [i8; 5],
    pub data_size: u32,
    pub data_type: [i8; 5], // UInt32Char_t
    pub bytes: [i8; 32],    // SMCBytes_t
}
