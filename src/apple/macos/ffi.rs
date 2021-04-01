//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use core_foundation_sys::base::CFAllocatorRef;
use core_foundation_sys::dictionary::CFMutableDictionaryRef;
use core_foundation_sys::string::{CFStringEncoding, CFStringRef};

#[cfg(not(feature = "apple-app-store"))]
use libc::c_int;
use libc::{c_char, c_void, mach_port_t, size_t};

pub(crate) use crate::sys::ffi::*;

extern "C" {
    #[cfg(not(feature = "apple-app-store"))]
    pub fn mach_absolute_time() -> u64;

    // The proc_* PID functions are internal Apple APIs which are not
    // allowed in App store releases as Apple blocks any binary using them.

    #[cfg(not(feature = "apple-app-store"))]
    pub fn proc_pidinfo(
        pid: c_int,
        flavor: c_int,
        arg: u64,
        buffer: *mut c_void,
        buffersize: c_int,
    ) -> c_int;

    #[cfg(not(feature = "apple-app-store"))]
    pub fn proc_listallpids(buffer: *mut c_void, buffersize: c_int) -> c_int;
    //pub fn proc_listpids(kind: u32, x: u32, buffer: *mut c_void, buffersize: c_int) -> c_int;
    //pub fn proc_name(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;
    //pub fn proc_regionfilename(pid: c_int, address: u64, buffer: *mut c_void,
    //                           buffersize: u32) -> c_int;

    #[cfg(not(feature = "apple-app-store"))]
    pub fn proc_pidpath(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;

    #[cfg(not(feature = "apple-app-store"))]
    pub fn proc_pid_rusage(pid: c_int, flavor: c_int, buffer: *mut c_void) -> c_int;

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

    // pub fn DADiskCreateFromVolumePath(
    //     allocator: CFAllocatorRef,
    //     session: DASessionRef,
    //     path: CFURLRef,
    // ) -> DADiskRef;
    pub fn DADiskCreateFromBSDName(
        allocator: CFAllocatorRef,
        session: DASessionRef,
        path: *const c_char,
    ) -> DADiskRef;
    // pub fn DADiskGetBSDName(disk: DADiskRef) -> *const c_char;

    pub fn DADiskCopyDescription(disk: DADiskRef) -> CFMutableDictionaryRef;
}

pub type DADiskRef = *const __DADisk;

#[allow(non_camel_case_types)]
pub type io_object_t = mach_port_t;
#[allow(non_camel_case_types)]
pub type io_connect_t = io_object_t;
#[allow(non_camel_case_types)]
pub type io_iterator_t = io_object_t;

pub type DASessionRef = *const __DASession;

// We need to wrap `DASessionRef` to be sure `System` remains Send+Sync.
pub struct SessionWrap(pub DASessionRef);

unsafe impl Send for SessionWrap {}
unsafe impl Sync for SessionWrap {}

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

//pub const PROC_ALL_PIDS: c_uint = 1;
#[cfg(not(feature = "apple-app-store"))]
pub const PROC_PIDTBSDINFO: c_int = 3;

#[cfg(not(feature = "apple-app-store"))]
pub const PROC_PIDPATHINFO_MAXSIZE: u32 = 4096;
