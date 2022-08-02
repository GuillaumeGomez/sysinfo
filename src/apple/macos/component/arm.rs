// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::CStr;

use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
use core_foundation_sys::base::kCFAllocatorDefault;
use core_foundation_sys::string::{
    kCFStringEncodingUTF8, CFStringCreateWithBytes, CFStringGetCStringPtr,
};

use super::super::CFReleaser;
use crate::apple::inner::ffi::{
    kHIDPage_AppleVendor, kHIDUsage_AppleVendor_TemperatureSensor, kIOHIDEventTypeTemperature,
    matching, IOHIDEventFieldBase, IOHIDEventGetFloatValue, IOHIDEventSystemClientCopyServices,
    IOHIDEventSystemClientCreate, IOHIDEventSystemClientSetMatching, IOHIDServiceClientCopyEvent,
    IOHIDServiceClientCopyProperty, __IOHIDEventSystemClient, __IOHIDServiceClient,
    HID_DEVICE_PROPERTY_PRODUCT,
};
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

        let client = || -> Option<CFReleaser<_>> {
            unsafe {
                let matches = CFReleaser::new(matching(
                    kHIDPage_AppleVendor,
                    kHIDUsage_AppleVendor_TemperatureSensor,
                ))?;

                let client = CFReleaser::new(IOHIDEventSystemClientCreate(kCFAllocatorDefault))?;

                let _ = IOHIDEventSystemClientSetMatching(client.inner(), matches.inner());
                Some(client)
            }
        }();

        if client.is_none() {
            return;
        }
        let client = client.unwrap();

        unsafe {
            let services = IOHIDEventSystemClientCopyServices(client.inner());
            if services.is_null() {
                return;
            }

            let key_ref = CFReleaser::new(CFStringCreateWithBytes(
                kCFAllocatorDefault,
                HID_DEVICE_PROPERTY_PRODUCT.as_ptr(),
                HID_DEVICE_PROPERTY_PRODUCT.len() as _,
                kCFStringEncodingUTF8,
                false as _,
            ));
            if key_ref.is_none() {
                return;
            }
            let key_ref = key_ref.unwrap();

            let count = CFArrayGetCount(services);

            for i in 0..count {
                let service = CFReleaser::new(CFArrayGetValueAtIndex(services, i) as *const _);
                if service.is_none() {
                    continue;
                }
                let service = service.unwrap();

                let name = CFReleaser::new(IOHIDServiceClientCopyProperty(
                    service.inner(),
                    key_ref.inner(),
                ));
                if name.is_none() {
                    continue;
                }
                let name = name.unwrap();

                let name_ptr =
                    CFStringGetCStringPtr(name.inner() as *const _, kCFStringEncodingUTF8);
                let name_str = CStr::from_ptr(name_ptr).to_string_lossy().to_string();

                self.inner
                    .push(Component::new(name_str, None, None, service));
            }

            self.client.replace(client);
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
            let event = CFReleaser::new(IOHIDServiceClientCopyEvent(
                self.service.inner() as *const _,
                kIOHIDEventTypeTemperature,
                0,
                0,
            ));
            if event.is_none() {
                return;
            }
            let event = event.unwrap();

            self.temperature = IOHIDEventGetFloatValue(
                event.inner(),
                IOHIDEventFieldBase(kIOHIDEventTypeTemperature),
            ) as f32;
        }
    }
}
