// Take a look at the license at the top of the repository in the LICENSE file.

use std::ptr::null;
use std::ffi::CStr;
use std::ffi::CString;

use core_foundation_sys::base::CFAllocatorRef;
use core_foundation_sys::base::kCFAllocatorDefault;
use core_foundation_sys::dictionary::CFMutableDictionaryRef;
use core_foundation_sys::dictionary::CFDictionaryRef;
use core_foundation_sys::dictionary::CFDictionaryCreate;
use core_foundation_sys::dictionary::kCFTypeDictionaryKeyCallBacks;
use core_foundation_sys::dictionary::kCFTypeDictionaryValueCallBacks;
use core_foundation_sys::string::CFStringEncoding;
use core_foundation_sys::string::CFStringRef;
use core_foundation_sys::string::CFStringCreateWithBytes;
use core_foundation_sys::string::kCFStringEncodingUTF8;
use core_foundation_sys::string::CFStringGetCStringPtr;
use core_foundation_sys::string::CFStringCreateWithCString;
use core_foundation_sys::array::__CFArray;
use core_foundation_sys::array::CFArrayCallBacks;
use core_foundation_sys::array::CFArrayRef;
use core_foundation_sys::array::CFArrayGetCount;
use core_foundation_sys::array::CFArrayGetValueAtIndex;
use core_foundation_sys::base::Boolean;
use core_foundation_sys::mach_port::CFIndex;
use core_foundation_sys::number::CFNumberCreate;
use core_foundation_sys::number::kCFNumberSInt32Type;

use libc::{c_char, c_void};
#[cfg(not(feature = "apple-sandbox"))]
use libc::{mach_port_t, size_t};

pub(crate) use crate::sys::ffi::*;

#[cfg(not(feature = "apple-sandbox"))]
extern "C" {
    // The proc_* PID functions are internal Apple APIs which are not
    // allowed in App store releases as Apple blocks any binary using them.

    // IOKit is only available on MacOS: https://developer.apple.com/documentation/iokit, and when not running inside
    // of the default macOS sandbox.
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
}

extern "C" {
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
pub type DASessionRef = *const __DASession;

// We need to wrap `DASessionRef` to be sure `System` remains Send+Sync.
pub struct SessionWrap(pub DASessionRef);

unsafe impl Send for SessionWrap {}
unsafe impl Sync for SessionWrap {}

#[cfg(not(feature = "apple-sandbox"))]
mod io_service {
    use super::mach_port_t;

    #[allow(non_camel_case_types)]
    pub type io_object_t = mach_port_t;
    #[allow(non_camel_case_types)]
    pub type io_connect_t = io_object_t;
    #[allow(non_camel_case_types)]
    pub type io_iterator_t = io_object_t;

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

    pub const KERNEL_INDEX_SMC: i32 = 2;
    pub const SMC_CMD_READ_KEYINFO: u8 = 9;
    pub const SMC_CMD_READ_BYTES: u8 = 5;

    pub const KIO_RETURN_SUCCESS: i32 = 0;
}

#[cfg(feature = "apple-sandbox")]
mod io_service {}

pub use io_service::*;

#[repr(C)]
pub struct __IOHIDServiceClient(libc::c_void);

pub type IOHIDServiceClientRef = *const __IOHIDServiceClient;

#[repr(C)]
pub struct __IOHIDEvent(libc::c_void);

pub type IOHIDEventRef = *const __IOHIDEvent;

pub const kIOHIDEventTypeTemperature: i64 = 15;

pub const kIOHIDEventTypePower: i64 = 15;

pub fn IOHIDEventFieldBase(event_type: i64) -> i64 {
    event_type << 16
}

#[cfg(not(feature = "apple-sandbox"))]
extern "C" {
    pub fn IOHIDEventSystemClientCreate(allocator: CFAllocatorRef) -> *mut libc::c_void;
    
    pub fn IOHIDEventSystemClientSetMatching(client: *mut libc::c_void, matches: CFDictionaryRef) -> i32;

    pub fn IOHIDEventSystemClientCopyServices(client: *mut libc::c_void) -> CFArrayRef;

    pub fn IOHIDServiceClientCopyProperty(service: IOHIDServiceClientRef, key: CFStringRef) -> CFStringRef;

    pub fn IOHIDServiceClientCopyEvent(service: IOHIDServiceClientRef, v0: i64, v1: i32, v2: i64) -> IOHIDEventRef;

    pub fn IOHIDEventGetFloatValue(event: IOHIDEventRef, field: i64) -> f64;
}

pub type CFMutableArrayRef = *mut __CFArray;

extern "C" {
    pub fn CFArrayCreateMutable(allocator: CFAllocatorRef, capacity: CFIndex, callbacks: *const CFArrayCallBacks) -> CFMutableArrayRef;

    pub fn CFArrayAppendValue(the_array: CFMutableArrayRef, value: *const libc::c_void);
}

pub(crate) const HID_DEVICE_PROPERTY_VENDOR_ID: &str = "VendorId";
pub(crate) const HID_DEVICE_PROPERTY_PRODUCT_ID: &str = "ProductID";
pub(crate) const HID_DEVICE_PROPERTY_PRODUCT: &str = "Product";
pub(crate) const HID_DEVICE_PROPERTY_PRIMARY_USAGE: &str = "PrimaryUsage";
pub(crate) const HID_DEVICE_PROPERTY_PRIMARY_USAGE_PAGE: &str = "PrimaryUsagePage";
pub(crate) const HID_DEVICE_PROPERTY_MAX_INPUT_REPORT_SIZE: &str = "MaxInputReportSize";
pub(crate) const HID_DEVICE_PROPERTY_MAX_OUTPUT_REPORT_SIZE: &str = "MaxOutputReportSize";
pub(crate) const HID_DEVICE_PROPERTY_REPORT_ID: &str = "ReportID";

pub(crate) const kHIDPage_AppleVendor: i32 = 0xff00;
pub(crate) const kHIDUsage_AppleVendor_TemperatureSensor: i32 = 0x0005;

pub(crate) fn matching(page: i32, usage: i32) -> CFDictionaryRef {
    unsafe {
        let primary_usage_page = CString::new(HID_DEVICE_PROPERTY_PRIMARY_USAGE_PAGE).unwrap();
        let primary_usage = CString::new(HID_DEVICE_PROPERTY_PRIMARY_USAGE).unwrap();

        let keys = [
            CFStringCreateWithCString(
                null() as *const _, 
                primary_usage_page.as_ptr(), 
                0
            ),
            CFStringCreateWithCString(
                null() as *const _, 
                primary_usage.as_ptr(), 
                0
            ),
        ];

        let nums = [
            CFNumberCreate(
                null(), 
                kCFNumberSInt32Type, 
                &page as *const _ as *const libc::c_void
            ),
            CFNumberCreate(
                null(), 
                kCFNumberSInt32Type, 
                &usage as *const _ as *const libc::c_void
            ),
        ];

        CFDictionaryCreate(
            null(), 
            &keys as *const _ as *const _,
            &nums as *const _ as *const _, 
            2,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks
        )
    }
}

pub fn get_product_names(sensors: CFDictionaryRef) -> Vec<String> {
    unsafe {
        let system = IOHIDEventSystemClientCreate(kCFAllocatorDefault);

        let _rv = IOHIDEventSystemClientSetMatching(system, sensors);

        let services = IOHIDEventSystemClientCopyServices(system);

        let count = CFArrayGetCount(services);

        let key_ref = CFStringCreateWithBytes(
            kCFAllocatorDefault, 
            HID_DEVICE_PROPERTY_PRODUCT.as_ptr(), 
            HID_DEVICE_PROPERTY_PRODUCT.len() as CFIndex, 
            kCFStringEncodingUTF8,
            false as Boolean
        );

        let mut names = Vec::with_capacity(count as usize);

        for i in 0..count {
            let service = CFArrayGetValueAtIndex(services, i);
            let name = IOHIDServiceClientCopyProperty(service as *const _, key_ref);
            if name != null() {
                // CFArrayAppendValue(array, name as *const _);
                let name_ptr = CFStringGetCStringPtr(name as *const _, kCFStringEncodingUTF8);
                let name = CStr::from_ptr(name_ptr).to_string_lossy();
                names.push(name);
            } 
        }

        names.into_iter().map(|i| i.to_string()).collect::<Vec<_>>()
    }
}
