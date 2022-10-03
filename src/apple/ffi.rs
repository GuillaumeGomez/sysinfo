// Take a look at the license at the top of the repository in the LICENSE file.

// Reexport items defined in either macos or ios ffi module.
pub use crate::sys::inner::ffi::*;

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {}

#[cfg_attr(feature = "debug", derive(Eq, Hash, PartialEq))]
#[derive(Clone)]
#[repr(C)]
pub struct Val_t {
    pub key: [i8; 5],
    pub data_size: u32,
    pub data_type: [i8; 5], // UInt32Char_t
    pub bytes: [i8; 32],    // SMCBytes_t
}
