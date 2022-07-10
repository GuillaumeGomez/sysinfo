// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::CStr;

use core_foundation_sys::base::kCFAllocatorDefault;
use core_foundation_sys::string::CFStringCreateWithBytes;
use core_foundation_sys::string::CFStringGetCStringPtr;
use core_foundation_sys::string::kCFStringEncodingUTF8;
use core_foundation_sys::array::CFArrayGetCount;
use core_foundation_sys::array::CFArrayGetValueAtIndex;

use crate::ComponentExt;
use crate::apple::inner::ffi::IOHIDEventSystemClientCopyServices;
use crate::apple::inner::ffi::IOHIDEventSystemClientCreate;
use crate::apple::inner::ffi::IOHIDEventSystemClientSetMatching;
use crate::apple::inner::ffi::IOHIDServiceClientCopyProperty;
use crate::apple::inner::ffi::IOHIDServiceClientRef;
use crate::apple::inner::ffi::IOHIDServiceClientCopyEvent;
use crate::apple::inner::ffi::IOHIDEventGetFloatValue;
use crate::apple::inner::ffi::kHIDPage_AppleVendor;
use crate::apple::inner::ffi::kHIDUsage_AppleVendor_TemperatureSensor;
use crate::apple::inner::ffi::kIOHIDEventTypeTemperature;
use crate::apple::inner::ffi::IOHIDEventFieldBase;
use crate::apple::inner::ffi::matching;
use crate::apple::inner::ffi::HID_DEVICE_PROPERTY_PRODUCT;

pub(crate) fn temperatures() -> Vec<Component> {
    let mut components = Vec::new();

    unsafe {
        let matches = matching(kHIDPage_AppleVendor, kHIDUsage_AppleVendor_TemperatureSensor);
        if matches.is_null() {
            return components;
        }

        let client = IOHIDEventSystemClientCreate(kCFAllocatorDefault);
        if client.is_null() {
            return components;
        }

        let _ = IOHIDEventSystemClientSetMatching(client, matches);

        let services = IOHIDEventSystemClientCopyServices(client);
        if services.is_null() {
            return components;
        }

        let key_ref = CFStringCreateWithBytes(
            kCFAllocatorDefault, 
            HID_DEVICE_PROPERTY_PRODUCT.as_ptr(), 
            HID_DEVICE_PROPERTY_PRODUCT.len() as _, 
            kCFStringEncodingUTF8,
            false as _
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
            let name = CStr::from_ptr(name_ptr).to_string_lossy();
            let component = Component::new(name.to_string(), None, None, service as IOHIDServiceClientRef);
            components.push(component);
        }
    }

    components
}

#[doc = include_str!("../../../../md_doc/component.md")]
pub struct Component {
    service: IOHIDServiceClientRef,
    temperature: f32,
    label: String,
    max: f32,
    critical: Option<f32>
}

impl Component { 
    pub(crate) fn new(
        label: String,
        max: Option<f32>,
        critical: Option<f32>,
        service: IOHIDServiceClientRef
    ) -> Self {
        Self {
            service,
            label,
            max: max.unwrap_or(0.),
            critical,
            temperature: 0.
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
            let event = IOHIDServiceClientCopyEvent(self.service as *const _, kIOHIDEventTypeTemperature, 0, 0);
            if !event.is_null() {
                self.temperature = IOHIDEventGetFloatValue(event, IOHIDEventFieldBase(kIOHIDEventTypeTemperature)) as f32;
            }
        }
    }
}