// Take a look at the license at the top of the repository in the LICENSE file.

use core_foundation_sys::{
    array::CFArrayRef, dictionary::CFDictionaryRef, error::CFErrorRef, string::CFStringRef,
    url::CFURLRef,
};

// Reexport items defined in either macos or ios ffi module.
pub use crate::sys::inner::ffi::*;

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub fn CFURLCopyResourcePropertiesForKeys(
        url: CFURLRef,
        keys: CFArrayRef,
        error: *mut CFErrorRef,
    ) -> CFDictionaryRef;

    pub static kCFURLVolumeIsEjectableKey: CFStringRef;
    pub static kCFURLVolumeIsRemovableKey: CFStringRef;
    pub static kCFURLVolumeAvailableCapacityKey: CFStringRef;
    pub static kCFURLVolumeAvailableCapacityForImportantUsageKey: CFStringRef;
    pub static kCFURLVolumeTotalCapacityKey: CFStringRef;
    pub static kCFURLVolumeNameKey: CFStringRef;
    pub static kCFURLVolumeIsLocalKey: CFStringRef;
    pub static kCFURLVolumeIsInternalKey: CFStringRef;
    pub static kCFURLVolumeIsBrowsableKey: CFStringRef;
}

#[cfg_attr(feature = "debug", derive(Eq, Hash, PartialEq))]
#[derive(Clone)]
#[repr(C)]
pub struct Val_t {
    pub key: [i8; 5],
    pub data_size: u32,
    pub data_type: [i8; 5], // UInt32Char_t
    pub bytes: [i8; 32],    // SMCBytes_t
}
