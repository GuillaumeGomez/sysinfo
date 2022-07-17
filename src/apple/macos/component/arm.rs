// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::CStr;

use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation_sys::base::{kCFAllocatorDefault, CFRelease};
use core_foundation_sys::string::{
    kCFStringEncodingUTF8, CFStringCreateWithBytes, CFStringGetCStringPtr,
};

use crate::apple::inner::ffi::{
    kHIDPage_AppleVendor, kHIDUsage_AppleVendor_TemperatureSensor, kIOHIDEventTypeTemperature,
    matching, IOHIDEventFieldBase, IOHIDEventGetFloatValue, IOHIDEventSystemClientCopyServices,
    IOHIDEventSystemClientCreate, IOHIDEventSystemClientRef, IOHIDEventSystemClientSetMatching,
    IOHIDServiceClientCopyEvent, IOHIDServiceClientCopyProperty, IOHIDServiceClientRef,
    HID_DEVICE_PROPERTY_PRODUCT,
};
use crate::ComponentExt;

pub(crate) struct Components {
    pub inner: Vec<Component>,
    client: Option<IOHIDEventSystemClientRef>,
    services: Option<CFArrayRef>,
}

impl Components {
    pub(crate) fn new() -> Self {
        Self {
            inner: vec![],
            client: None,
            services: None,
        }
    }

    pub(crate) fn refresh(&mut self) {
        self.inner.clear();

        let client = || -> IOHIDEventSystemClientRef {
            unsafe {
                let matches = matching(
                    kHIDPage_AppleVendor,
                    kHIDUsage_AppleVendor_TemperatureSensor,
                );
                if matches.is_null() {
                    return std::ptr::null() as _;
                }

                let client = IOHIDEventSystemClientCreate(kCFAllocatorDefault);
                if client.is_null() {
                    return std::ptr::null() as _;
                }

                let _ = IOHIDEventSystemClientSetMatching(client, matches);
                CFRelease(matches as _);
                client
            }
        }();

        unsafe {
            let services = IOHIDEventSystemClientCopyServices(client);
            if services.is_null() {
                return;
            }

            let key_ref = CFStringCreateWithBytes(
                kCFAllocatorDefault,
                HID_DEVICE_PROPERTY_PRODUCT.as_ptr(),
                HID_DEVICE_PROPERTY_PRODUCT.len() as _,
                kCFStringEncodingUTF8,
                false as _,
            );

            let count = CFArrayGetCount(services);

            for i in 0..count {
                let service = CFArrayGetValueAtIndex(services, i);
                if service.is_null() {
                    continue;
                }

                let name = IOHIDServiceClientCopyProperty(service as *const _, key_ref);
                if name.is_null() {
                    continue;
                }

                let name_ptr = CFStringGetCStringPtr(name as *const _, kCFStringEncodingUTF8);
                let name_str = CStr::from_ptr(name_ptr).to_string_lossy().to_string();
                CFRelease(name as _);

                self.inner
                    .push(Component::new(name_str, None, None, service as _));
            }

            CFRelease(key_ref as _);

            match self.client.replace(client) {
                Some(c) => {
                    if !c.is_null() {
                        CFRelease(c as _)
                    }
                }
                None => {}
            }
            match self.services.replace(services) {
                Some(s) => {
                    if !s.is_null() {
                        CFRelease(s as _)
                    }
                }
                None => {}
            }
        }
    }
}

impl Drop for Components {
    fn drop(&mut self) {
        match self.client.take() {
            Some(c) => {
                if !c.is_null() {
                    unsafe { CFRelease(c as _) }
                }
            }
            None => {}
        }

        match self.services.take() {
            Some(s) => {
                if !s.is_null() {
                    unsafe { CFRelease(s as _) }
                }
            }
            None => {}
        }
    }
}

unsafe impl Send for Components {}

unsafe impl Sync for Components {}

#[doc = include_str!("../../../../md_doc/component.md")]
pub struct Component {
    service: IOHIDServiceClientRef,
    temperature: f32,
    label: String,
    max: f32,
    critical: Option<f32>,
}

impl Component {
    pub(crate) fn new(
        label: String,
        max: Option<f32>,
        critical: Option<f32>,
        service: IOHIDServiceClientRef,
    ) -> Self {
        Self {
            service,
            label,
            max: max.unwrap_or(0.),
            critical,
            temperature: 0.,
        }
    }
}

unsafe impl Send for Component {}

unsafe impl Sync for Component {}

impl ComponentExt for Component {
    fn temperature(&self) -> f32 {
        self.temperature
    }

    fn max(&self) -> f32 {
        self.max
    }

    fn critical(&self) -> Option<f32> {
        self.critical
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn refresh(&mut self) {
        unsafe {
            let event = IOHIDServiceClientCopyEvent(
                self.service as *const _,
                kIOHIDEventTypeTemperature,
                0,
                0,
            );
            if !event.is_null() {
                self.temperature =
                    IOHIDEventGetFloatValue(event, IOHIDEventFieldBase(kIOHIDEventTypeTemperature))
                        as f32;
                CFRelease(event as _);
            }
        }
    }
}
