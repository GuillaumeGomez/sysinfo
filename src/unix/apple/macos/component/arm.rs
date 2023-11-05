// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::CStr;

use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
use core_foundation_sys::base::{kCFAllocatorDefault, CFRetain};
use core_foundation_sys::string::{
    kCFStringEncodingUTF8, CFStringCreateWithBytes, CFStringGetCStringPtr,
};

use crate::sys::inner::ffi::{
    kHIDPage_AppleVendor, kHIDUsage_AppleVendor_TemperatureSensor, kIOHIDEventTypeTemperature,
    matching, IOHIDEventFieldBase, IOHIDEventGetFloatValue, IOHIDEventSystemClientCopyServices,
    IOHIDEventSystemClientCreate, IOHIDEventSystemClientSetMatching, IOHIDServiceClientCopyEvent,
    IOHIDServiceClientCopyProperty, __IOHIDEventSystemClient, __IOHIDServiceClient,
    HID_DEVICE_PROPERTY_PRODUCT,
};
use crate::sys::utils::CFReleaser;
use crate::Component;

pub(crate) struct ComponentsInner {
    components: Vec<Component>,
    client: Option<CFReleaser<__IOHIDEventSystemClient>>,
}

impl ComponentsInner {
    pub(crate) fn new() -> Self {
        Self {
            components: vec![],
            client: None,
        }
    }

    pub(crate) fn from_vec(components: Vec<Component>) -> Self {
        Self {
            components,
            client: None,
        }
    }

    pub(crate) fn into_vec(self) -> Vec<Component> {
        self.components
    }

    pub(crate) fn list(&self) -> &[Component] {
        &self.components
    }

    pub(crate) fn list_mut(&mut self) -> &mut [Component] {
        &mut self.components
    }

    pub(crate) fn refresh_list(&mut self) {
        self.components.clear();

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

                let mut component = ComponentInner::new(name_str, None, None, service);
                component.refresh();

                self.components.push(Component { inner: component });
            }
        }
    }
}

pub(crate) struct ComponentInner {
    service: CFReleaser<__IOHIDServiceClient>,
    temperature: f32,
    label: String,
    max: f32,
    critical: Option<f32>,
}

impl ComponentInner {
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

    pub(crate) fn temperature(&self) -> f32 {
        self.temperature
    }

    pub(crate) fn max(&self) -> f32 {
        self.max
    }

    pub(crate) fn critical(&self) -> Option<f32> {
        self.critical
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) fn refresh(&mut self) {
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
