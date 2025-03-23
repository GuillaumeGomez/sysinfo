// Take a look at the license at the top of the repository in the LICENSE file.

// Note: IOKit is only available on macOS up until very recent iOS versions: https://developer.apple.com/documentation/iokit

cfg_if! {
    if #[cfg(any(
        feature = "disk",
        all(
            not(feature = "apple-sandbox"),
            any(
                feature = "system",
                all(
                    feature = "component",
                    any(target_arch = "x86", target_arch = "x86_64")
                )
            )
        ),
    ))] {
        #[allow(non_camel_case_types)]
        pub type io_object_t = libc::mach_port_t;
        #[allow(non_camel_case_types)]
        pub type io_iterator_t = io_object_t;
        // Based on https://github.com/libusb/libusb/blob/bed8d3034eac74a6e1ba123b5c270ea63cb6cf1a/libusb/os/darwin_usb.c#L54-L55,
        // we can simply set it to 0 (and is the same value as its
        // replacement `kIOMainPortDefault`).
        #[allow(non_upper_case_globals)]
        pub const kIOMasterPortDefault: libc::mach_port_t = 0;
    }

    if #[cfg(any(
        all(feature = "system", not(feature = "apple-sandbox")),
        feature = "disk"
    ))] {
        #[allow(non_camel_case_types)]
        pub type io_registry_entry_t = io_object_t;
        #[allow(non_camel_case_types)]
        pub type io_name_t = *const libc::c_char;
        // This is a hack, `io_name_t` should normally be `[c_char; 128]` but Rust makes it very annoying
        // to deal with that so we go around it a bit.
        #[allow(non_camel_case_types, dead_code)]
        pub type io_name = [libc::c_char; 128];
    }
}

#[cfg(any(
    feature = "disk",
    all(not(feature = "apple-sandbox"), feature = "system",),
))]
pub type IOOptionBits = u32;

cfg_if! {
    if #[cfg(feature = "disk")] {
        #[allow(non_upper_case_globals)]
        pub const kIOServicePlane: &[u8] = b"IOService\0";
        #[allow(non_upper_case_globals)]
        pub const kIOPropertyDeviceCharacteristicsKey: &str = "Device Characteristics";
        #[allow(non_upper_case_globals)]
        pub const kIOPropertyMediumTypeKey: &str = "Medium Type";
        #[allow(non_upper_case_globals)]
        pub const kIOPropertyMediumTypeSolidStateKey: &str = "Solid State";
        #[allow(non_upper_case_globals)]
        pub const kIOPropertyMediumTypeRotationalKey: &str = "Rotational";
        #[allow(non_upper_case_globals)]
        pub const kIOBlockStorageDriverStatisticsKey: &str = "Statistics";
        #[allow(non_upper_case_globals)]
        pub const kIOBlockStorageDriverStatisticsBytesReadKey: &str = "Bytes (Read)";
        #[allow(non_upper_case_globals)]
        pub const kIOBlockStorageDriverStatisticsBytesWrittenKey: &str = "Bytes (Write)";
    }
}

// Note: Obtaining information about disks using IOKIt is allowed inside the default macOS App Sandbox.
#[cfg(any(
    feature = "disk",
    all(
        not(feature = "apple-sandbox"),
        any(
            feature = "system",
            all(
                feature = "component",
                any(target_arch = "x86", target_arch = "x86_64")
            )
        )
    ),
))]
#[link(name = "IOKit", kind = "framework")]
extern "C" {
    pub fn IOServiceGetMatchingServices(
        mainPort: libc::mach_port_t,
        matching: std::ptr::NonNull<objc2_core_foundation::CFMutableDictionary>, // CF_RELEASES_ARGUMENT
        existing: *mut io_iterator_t,
    ) -> libc::kern_return_t;
    #[cfg(all(
        not(feature = "apple-sandbox"),
        any(
            feature = "system",
            all(
                feature = "component",
                any(target_arch = "x86", target_arch = "x86_64")
            ),
        ),
    ))]
    pub fn IOServiceMatching(
        a: *const libc::c_char,
    ) -> Option<std::ptr::NonNull<objc2_core_foundation::CFMutableDictionary>>; // CF_RETURNS_RETAINED

    pub fn IOIteratorNext(iterator: io_iterator_t) -> io_object_t;

    pub fn IOObjectRelease(obj: io_object_t) -> libc::kern_return_t;

    #[cfg(any(feature = "system", feature = "disk"))]
    pub fn IORegistryEntryCreateCFProperty(
        entry: io_registry_entry_t,
        key: &objc2_core_foundation::CFString,
        allocator: Option<&objc2_core_foundation::CFAllocator>,
        options: IOOptionBits,
    ) -> Option<std::ptr::NonNull<objc2_core_foundation::CFType>>;
    #[cfg(feature = "disk")]
    pub fn IORegistryEntryGetParentEntry(
        entry: io_registry_entry_t,
        plane: io_name_t,
        parent: *mut io_registry_entry_t,
    ) -> libc::kern_return_t;
    #[cfg(feature = "disk")]
    pub fn IOBSDNameMatching(
        mainPort: libc::mach_port_t,
        options: u32,
        bsdName: *const libc::c_char,
    ) -> Option<std::ptr::NonNull<objc2_core_foundation::CFMutableDictionary>>; // CF_RETURNS_RETAINED
    #[cfg(all(feature = "system", not(feature = "apple-sandbox")))]
    pub fn IORegistryEntryGetName(
        entry: io_registry_entry_t,
        name: io_name_t,
    ) -> libc::kern_return_t;
    #[cfg(feature = "disk")]
    pub fn IOObjectConformsTo(
        object: io_object_t,
        className: *const libc::c_char,
    ) -> libc::boolean_t;
}

#[cfg(all(
    not(feature = "apple-sandbox"),
    any(
        feature = "system",
        all(
            feature = "component",
            any(target_arch = "x86", target_arch = "x86_64")
        ),
    ),
))]
pub const KIO_RETURN_SUCCESS: i32 = 0;

#[cfg(all(
    not(feature = "apple-sandbox"),
    all(
        feature = "component",
        any(target_arch = "x86", target_arch = "x86_64")
    ),
))]
mod io_service {
    use super::io_object_t;
    use libc::{kern_return_t, mach_port_t, size_t, task_t};

    #[allow(non_camel_case_types)]
    pub type io_connect_t = io_object_t;

    #[allow(non_camel_case_types)]
    pub type io_service_t = io_object_t;

    #[allow(non_camel_case_types)]
    pub type task_port_t = task_t;

    extern "C" {
        pub fn IOServiceOpen(
            device: io_service_t,
            owning_task: task_port_t,
            type_: u32,
            connect: *mut io_connect_t,
        ) -> kern_return_t;

        pub fn IOServiceClose(a: io_connect_t) -> kern_return_t;

        #[allow(dead_code)]
        pub fn IOConnectCallStructMethod(
            connection: mach_port_t,
            selector: u32,
            inputStruct: *const KeyData_t,
            inputStructCnt: size_t,
            outputStruct: *mut KeyData_t,
            outputStructCnt: *mut size_t,
        ) -> kern_return_t;
    }

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

    #[allow(dead_code)]
    pub const KERNEL_INDEX_SMC: i32 = 2;

    #[allow(dead_code)]
    pub const SMC_CMD_READ_KEYINFO: u8 = 9;

    #[allow(dead_code)]
    pub const SMC_CMD_READ_BYTES: u8 = 5;
}

#[cfg(feature = "apple-sandbox")]
mod io_service {}

#[cfg(all(
    feature = "component",
    not(feature = "apple-sandbox"),
    target_arch = "aarch64"
))]
mod io_service {
    use std::ptr::NonNull;

    use objc2_core_foundation::{
        cf_type, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks, CFAllocator,
        CFArray, CFDictionary, CFDictionaryCreate, CFNumber, CFRetained, CFString,
    };

    #[repr(C)]
    pub struct IOHIDServiceClient(libc::c_void);

    cf_type!(
        #[encoding_name = "__IOHIDServiceClient"]
        unsafe impl IOHIDServiceClient {}
    );

    #[repr(C)]
    pub struct IOHIDEventSystemClient(libc::c_void);

    cf_type!(
        #[encoding_name = "__IOHIDEventSystemClient"]
        unsafe impl IOHIDEventSystemClient {}
    );

    #[repr(C)]
    pub struct IOHIDEvent(libc::c_void);

    cf_type!(
        #[encoding_name = "__IOHIDEvent"]
        unsafe impl IOHIDEvent {}
    );

    #[allow(non_upper_case_globals)]
    pub const kIOHIDEventTypeTemperature: i64 = 15;

    #[inline]
    #[allow(non_snake_case)]
    pub fn IOHIDEventFieldBase(event_type: i64) -> i64 {
        event_type << 16
    }

    #[cfg(not(feature = "apple-sandbox"))]
    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        pub fn IOHIDEventSystemClientCreate(
            allocator: Option<&CFAllocator>,
        ) -> Option<NonNull<IOHIDEventSystemClient>>;

        pub fn IOHIDEventSystemClientSetMatching(
            client: &IOHIDEventSystemClient,
            matches: &CFDictionary,
        ) -> i32;

        pub fn IOHIDEventSystemClientCopyServices(
            client: &IOHIDEventSystemClient,
        ) -> Option<NonNull<CFArray>>;

        pub fn IOHIDServiceClientCopyProperty(
            service: &IOHIDServiceClient,
            key: &CFString,
        ) -> Option<NonNull<CFString>>;

        pub fn IOHIDServiceClientCopyEvent(
            service: &IOHIDServiceClient,
            v0: i64,
            v1: i32,
            v2: i64,
        ) -> Option<NonNull<IOHIDEvent>>;

        pub fn IOHIDEventGetFloatValue(event: &IOHIDEvent, field: i64) -> f64;
    }

    pub(crate) const HID_DEVICE_PROPERTY_PRODUCT: &str = "Product";

    pub(crate) const HID_DEVICE_PROPERTY_PRIMARY_USAGE: &str = "PrimaryUsage";
    pub(crate) const HID_DEVICE_PROPERTY_PRIMARY_USAGE_PAGE: &str = "PrimaryUsagePage";

    #[allow(non_upper_case_globals)]
    pub(crate) const kHIDPage_AppleVendor: i32 = 0xff00;

    #[allow(non_upper_case_globals)]
    pub(crate) const kHIDUsage_AppleVendor_TemperatureSensor: i32 = 0x0005;

    pub(crate) fn matching(page: i32, usage: i32) -> Option<CFRetained<CFDictionary>> {
        unsafe {
            let keys = [
                CFString::from_static_str(HID_DEVICE_PROPERTY_PRIMARY_USAGE_PAGE),
                CFString::from_static_str(HID_DEVICE_PROPERTY_PRIMARY_USAGE),
            ];

            let nums = [CFNumber::new_i32(page), CFNumber::new_i32(usage)];

            CFDictionaryCreate(
                None,
                &keys as *const _ as *mut _,
                &nums as *const _ as *mut _,
                2,
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            )
        }
    }
}

#[cfg(all(
    feature = "component",
    not(feature = "apple-sandbox"),
    any(target_arch = "aarch64", target_arch = "x86", target_arch = "x86_64")
))]
pub use io_service::*;
