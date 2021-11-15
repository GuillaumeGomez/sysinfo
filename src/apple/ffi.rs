// Take a look at the license at the top of the repository in the LICENSE file.

use libc::c_void;

// Reexport items defined in either macos or ios ffi module.
pub use crate::sys::inner::ffi::*;

#[repr(C)]
pub struct __DADisk(c_void);
#[repr(C)]
pub struct __DASession(c_void);

// #[allow(non_camel_case_types)]
// pub type io_name_t = [u8; 128];
// #[allow(non_camel_case_types)]
// pub type io_registry_entry_t = io_object_t;

// pub type IOOptionBits = u32;

#[cfg_attr(feature = "debug", derive(Eq, Hash, PartialEq))]
#[derive(Clone)]
#[repr(C)]
pub struct Val_t {
    pub key: [i8; 5],
    pub data_size: u32,
    pub data_type: [i8; 5], // UInt32Char_t
    pub bytes: [i8; 32],    // SMCBytes_t
}
