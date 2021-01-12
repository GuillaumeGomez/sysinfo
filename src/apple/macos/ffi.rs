//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use core_foundation_sys::array::__CFArray;
use core_foundation_sys::base::CFAllocatorRef;
use core_foundation_sys::dictionary::CFMutableDictionaryRef;
use core_foundation_sys::string::{CFStringEncoding, CFStringRef};
use core_foundation_sys::url::CFURLRef;
use libc::{c_char, size_t};

pub(crate) use super::super::ffi::*;

extern "C" {
    // IOKit is only available on MacOS: https://developer.apple.com/documentation/iokit

    pub fn IOMasterPort(a: i32, b: *mut mach_port_t) -> i32;

    pub fn IOServiceMatching(a: *const c_char) -> *mut c_void;

    pub fn IOServiceGetMatchingServices(
        a: mach_port_t,
        b: *mut c_void,
        c: *mut io_iterator_t,
    ) -> i32;

    pub fn IOIteratorNext(iterator: io_iterator_t) -> io_object_t;

    pub fn IOObjectRelease(obj: io_object_t) -> i32;

    pub fn IOServiceOpen(device: io_object_t, a: u32, t: u32, x: *mut io_connect_t) -> i32;

    pub fn IOServiceClose(a: io_connect_t) -> i32;

    pub fn IOConnectCallStructMethod(
        connection: mach_port_t,
        selector: u32,
        inputStruct: *const KeyData_t,
        inputStructCnt: size_t,
        outputStruct: *mut KeyData_t,
        outputStructCnt: *mut size_t,
    ) -> i32;
    // pub fn IORegistryEntryCreateCFProperties(
    //     entry: io_registry_entry_t,
    //     properties: *mut CFMutableDictionaryRef,
    //     allocator: CFAllocatorRef,
    //     options: IOOptionBits,
    // ) -> kern_return_t;
    // pub fn IORegistryEntryGetName(entry: io_registry_entry_t, name: *mut c_char) -> kern_return_t;

    pub fn CFStringCreateWithCStringNoCopy(
        alloc: *mut c_void,
        cStr: *const c_char,
        encoding: CFStringEncoding,
        contentsDeallocator: *mut c_void,
    ) -> CFStringRef;

    // Disk information functions are non-operational on iOS because of the sandboxing
    // restrictions of apps, so they don't can't filesystem information. This results in
    // mountedVolumeURLs and similar returning `nil`. Hence, they are MacOS specific here.

    pub fn DASessionCreate(allocator: CFAllocatorRef) -> DASessionRef;

    pub fn DADiskCreateFromVolumePath(
        allocator: CFAllocatorRef,
        session: DASessionRef,
        path: CFURLRef,
    ) -> DADiskRef;
    // pub fn DADiskGetBSDName(disk: DADiskRef) -> *const c_char;

    pub fn DADiskCopyDescription(disk: DADiskRef) -> CFMutableDictionaryRef;

    pub fn macos_get_disks() -> CFArrayRef;
}

pub type DADiskRef = *const __DADisk;
pub type CFArrayRef = *const __CFArray;

#[allow(non_camel_case_types)]
pub type io_iterator_t = io_object_t;

#[cfg(target_os = "macos")]
#[cfg_attr(feature = "debug", derive(Debug, Eq, Hash, PartialEq))]
#[repr(C)]
pub struct KeyData_vers_t {
    pub major: u8,
    pub minor: u8,
    pub build: u8,
    pub reserved: [u8; 1],
    pub release: u16,
}

#[cfg_attr(feature = "debug", derive(Debug, Eq, Hash, PartialEq))]
#[repr(C)]
pub struct KeyData_pLimitData_t {
    pub version: u16,
    pub length: u16,
    pub cpu_plimit: u32,
    pub gpu_plimit: u32,
    pub mem_plimit: u32,
}

#[cfg_attr(feature = "debug", derive(Debug, Eq, Hash, PartialEq))]
#[repr(C)]
pub struct KeyData_keyInfo_t {
    pub data_size: u32,
    pub data_type: u32,
    pub data_attributes: u8,
}

#[cfg_attr(feature = "debug", derive(Debug, Eq, Hash, PartialEq))]
#[repr(C)]
pub struct KeyData_t {
    pub key: u32,
    pub vers: KeyData_vers_t,
    pub p_limit_data: KeyData_pLimitData_t,
    pub key_info: KeyData_keyInfo_t,
    pub result: u8,
    pub status: u8,
    pub data8: u8,
    pub data32: u32,
    pub bytes: [i8; 32], // SMCBytes_t
}

pub const MACH_PORT_NULL: i32 = 0;
pub const KERNEL_INDEX_SMC: i32 = 2;
pub const SMC_CMD_READ_KEYINFO: u8 = 9;
pub const SMC_CMD_READ_BYTES: u8 = 5;

pub const KIO_RETURN_SUCCESS: i32 = 0;
