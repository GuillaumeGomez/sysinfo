// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::CStr;

use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
use core_foundation_sys::base::{kCFAllocatorDefault, CFRetain};
use core_foundation_sys::string::{
    kCFStringEncodingUTF8, CFStringCreateWithBytes, CFStringGetCStringPtr,
};

use crate::apple::inner::ffi::{
    kHIDPage_AppleVendor, kHIDUsage_AppleVendor_TemperatureSensor, kIOHIDEventTypeTemperature,
    matching, IOHIDEventFieldBase, IOHIDEventGetFloatValue, IOHIDEventSystemClientCopyServices,
    IOHIDEventSystemClientCreate, IOHIDEventSystemClientSetMatching, IOHIDServiceClientCopyEvent,
    IOHIDServiceClientCopyProperty, __IOHIDEventSystemClient, __IOHIDServiceClient,
    HID_DEVICE_PROPERTY_PRODUCT,
};
use crate::sys::utils::CFReleaser;
use crate::ComponentExt;

pub(crate) struct Components {
    pub inner: Vec<Component>,
    client: Option<CFReleaser<__IOHIDEventSystemClient>>,
}

impl Components {
    pub(crate) fn new() -> Self {
        Self {
            inner: vec![],
            client: None,
        }
    }

    pub(crate) fn refresh(&mut self) {
        self.inner.clear();

        unsafe {
            let matches = match CFReleaser::new(matching(
                kHIDPage_AppleVendor,
                kHIDUsage_AppleVendor_TemperatureSensor,
            )) {
                Some(m) => m,
                None => return,
            };

            if self.client.is_none() {
                let client =
                    match CFReleaser::new(IOHIDEventSystemClientCreate(kCFAllocatorDefault)) {
                        Some(c) => c,
                        None => return,
                    };
                // Without this call, client is freed during the execution of the program. It must be kept!
                CFRetain(client.inner() as _);
                self.client = Some(client);
            }

            let client = self.client.as_ref().unwrap();

            let _ = IOHIDEventSystemClientSetMatching(client.inner(), matches.inner());

            let services = match CFReleaser::new(IOHIDEventSystemClientCopyServices(client.inner()))
            {
                Some(s) => s,
                None => return,
            };

            let key_ref = match CFReleaser::new(CFStringCreateWithBytes(
                kCFAllocatorDefault,
                HID_DEVICE_PROPERTY_PRODUCT.as_ptr(),
                HID_DEVICE_PROPERTY_PRODUCT.len() as _,
                kCFStringEncodingUTF8,
                false as _,
            )) {
                Some(r) => r,
                None => return,
            };

            let count = CFArrayGetCount(services.inner());

            for i in 0..count {
                let service = match CFReleaser::new(
                    CFArrayGetValueAtIndex(services.inner(), i) as *const _
                ) {
                    Some(s) => s,
                    None => continue,
                };

                let name = match CFReleaser::new(IOHIDServiceClientCopyProperty(
                    service.inner(),
                    key_ref.inner(),
                )) {
                    Some(n) => n,
                    None => continue,
                };

                let name_ptr =
                    CFStringGetCStringPtr(name.inner() as *const _, kCFStringEncodingUTF8);
                let name_str = CStr::from_ptr(name_ptr).to_string_lossy().to_string();

                let mut component = Component::new(name_str, None, None, service);
                component.refresh();

                self.inner.push(component);
            }
        }
    }
}

unsafe impl Send for Components {}
unsafe impl Sync for Components {}

#[doc = include_str!("../../../../md_doc/component.md")]
pub struct Component {
    service: CFReleaser<__IOHIDServiceClient>,
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
        service: CFReleaser<__IOHIDServiceClient>,
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
            let event = match CFReleaser::new(IOHIDServiceClientCopyEvent(
                self.service.inner() as *const _,
                kIOHIDEventTypeTemperature,
                0,
                0,
            )) {
                Some(e) => e,
                None => return,
            };

            self.temperature = IOHIDEventGetFloatValue(
                event.inner(),
                IOHIDEventFieldBase(kIOHIDEventTypeTemperature),
            ) as _;
            if self.temperature > self.max {
                self.max = self.temperature;
            }
        }
    }
}
