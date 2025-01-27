// Take a look at the license at the top of the repository in the LICENSE file.

use std::ptr::NonNull;

use objc2_core_foundation::{
    kCFAllocatorDefault, CFArrayGetCount, CFArrayGetValueAtIndex, CFRetained, CFString,
};

use crate::sys::inner::ffi::{
    kHIDPage_AppleVendor, kHIDUsage_AppleVendor_TemperatureSensor, kIOHIDEventTypeTemperature,
    matching, IOHIDEventFieldBase, IOHIDEventGetFloatValue, IOHIDEventSystemClientCopyServices,
    IOHIDEventSystemClientCreate, IOHIDEventSystemClientSetMatching, IOHIDServiceClientCopyEvent,
    IOHIDServiceClientCopyProperty, HID_DEVICE_PROPERTY_PRODUCT,
};
use crate::unix::apple::ffi::{IOHIDEventSystemClient, IOHIDServiceClient};
use crate::Component;

pub(crate) struct ComponentsInner {
    pub(crate) components: Vec<Component>,
    client: Option<CFRetained<IOHIDEventSystemClient>>,
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

    #[allow(unreachable_code)]
    pub(crate) fn refresh(&mut self) {
        unsafe {
            let matches = match matching(
                kHIDPage_AppleVendor,
                kHIDUsage_AppleVendor_TemperatureSensor,
            ) {
                Some(m) => m,
                None => return,
            };

            if self.client.is_none() {
                let client = match IOHIDEventSystemClientCreate(kCFAllocatorDefault) {
                    Some(c) => CFRetained::from_raw(c),
                    None => return,
                };
                self.client = Some(client);
            }

            let client = self.client.as_ref().unwrap();

            let _ = IOHIDEventSystemClientSetMatching(client, &matches);

            let services = match IOHIDEventSystemClientCopyServices(client) {
                Some(s) => CFRetained::from_raw(s),
                None => return,
            };

            let key = CFString::from_static_str(HID_DEVICE_PROPERTY_PRODUCT);

            let count = CFArrayGetCount(&services);

            for i in 0..count {
                let service = CFArrayGetValueAtIndex(&services, i).cast::<IOHIDServiceClient>();
                if service.is_null() {
                    continue;
                }
                // The 'service' should never be freed since it is returned by a 'Get' call.
                // See issue https://github.com/GuillaumeGomez/sysinfo/issues/1279
                let service = CFRetained::retain(NonNull::from(&*service));

                let Some(name) = IOHIDServiceClientCopyProperty(&service, &key) else {
                    continue;
                };
                let name = CFRetained::from_raw(name);
                let name_str = name.to_string();

                if let Some(c) = self
                    .components
                    .iter_mut()
                    .find(|c| c.inner.label == name_str)
                {
                    c.refresh();
                    c.inner.updated = true;
                    continue;
                }

                let mut component = ComponentInner::new(name_str, None, None, service);
                component.refresh();

                self.components.push(Component { inner: component });
            }
        }
    }
}

pub(crate) struct ComponentInner {
    service: CFRetained<IOHIDServiceClient>,
    temperature: Option<f32>,
    label: String,
    max: f32,
    critical: Option<f32>,
    pub(crate) updated: bool,
}

unsafe impl Send for ComponentInner {}
unsafe impl Sync for ComponentInner {}

impl ComponentInner {
    pub(crate) fn new(
        label: String,
        max: Option<f32>,
        critical: Option<f32>,
        service: CFRetained<IOHIDServiceClient>,
    ) -> Self {
        Self {
            service,
            label,
            max: max.unwrap_or(0.),
            critical,
            temperature: None,
            updated: true,
        }
    }

    pub(crate) fn temperature(&self) -> Option<f32> {
        self.temperature
    }

    pub(crate) fn max(&self) -> Option<f32> {
        Some(self.max)
    }

    pub(crate) fn critical(&self) -> Option<f32> {
        self.critical
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) fn refresh(&mut self) {
        unsafe {
            let Some(event) =
                IOHIDServiceClientCopyEvent(&self.service, kIOHIDEventTypeTemperature, 0, 0)
            else {
                self.temperature = None;
                return;
            };
            let event = CFRetained::from_raw(event);

            let temperature =
                IOHIDEventGetFloatValue(&event, IOHIDEventFieldBase(kIOHIDEventTypeTemperature))
                    as _;
            self.temperature = Some(temperature);
            if temperature > self.max {
                self.max = temperature;
            }
        }
    }
}
