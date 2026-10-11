// Take a look at the license at the top of the repository in the LICENSE file.

use crate::unix::apple::utils::{IOReleaser, MAIN_PORT};
use crate::utils::pci_vendor_name;
use crate::{Error, PciAddress, PciDevice, PciDeviceClass, PciDevicesInner};
use objc2_core_foundation::{CFData, CFDictionary, CFRetained, CFString, kCFAllocatorDefault};
use objc2_io_kit::{
    IOIteratorNext, IORegistryEntryCreateCFProperty, IOServiceGetMatchingServices,
    IOServiceMatching, io_iterator_t, io_registry_entry_t, kIOReturnSuccess,
};
use std::path::Path;

fn refresh_pci_devices_from_ioreg(devices: &mut Vec<PciDevice>, remove_not_listed_devices: bool) {
    unsafe {
        let Some(matching) = IOServiceMatching(c"IOPCIDevice".as_ptr()) else {
            sysinfo_debug!("Failed to create IOPCIDevice matching dictionary");
            return;
        };
        let matching = CFRetained::<CFDictionary>::from(&matching);

        let mut iterator: io_iterator_t = 0;
        let result = IOServiceGetMatchingServices(*MAIN_PORT, Some(matching), &mut iterator);
        if result != kIOReturnSuccess {
            sysinfo_debug!("Error: IOServiceGetMatchingServices() = {result}");
            return;
        }

        let Some(iterator) = IOReleaser::new(iterator) else {
            sysinfo_debug!(
                "Error: IOServiceGetMatchingServices succeeded but returned invalid descriptor"
            );
            return;
        };

        let address_key = CFString::from_str("pcidebug");
        let vendor_id_key = CFString::from_str("vendor-id");
        let device_id_key = CFString::from_str("device-id");
        let subsystem_vendor_id_key = CFString::from_str("subsystem-vendor-id");
        let subsystem_device_id_key = CFString::from_str("subsystem-id");
        let class_key = CFString::from_str("class-code");
        let revision_key = CFString::from_str("revision-id");

        while let Some(device) = IOReleaser::new(IOIteratorNext(iterator.inner())) {
            let entry: io_registry_entry_t = device.inner();

            let Some(pci_address) = read_pci_address(entry, &address_key) else {
                continue;
            };

            let vendor_id = read_property_u32(entry, &vendor_id_key).unwrap_or(0) as u16;
            let device_id = read_property_u32(entry, &device_id_key).unwrap_or(0) as u16;
            let subsystem_vendor_id =
                read_property_u32(entry, &subsystem_vendor_id_key).unwrap_or(0) as u16;
            let subsystem_device_id =
                read_property_u32(entry, &subsystem_device_id_key).unwrap_or(0) as u16;
            let class = PciDeviceClass::from(read_property_u32(entry, &class_key).unwrap_or(0));
            let revision = read_property_u32(entry, &revision_key).unwrap_or(0) as u8;

            let vendor_name = pci_vendor_name(vendor_id).map(|v| v.to_string());
            let subsystem_vendor_name = pci_vendor_name(subsystem_vendor_id).map(|v| v.to_string());

            if let Some(existing_device) =
                devices.iter_mut().find(|d| d.inner.address == pci_address)
            {
                let inner = &mut existing_device.inner;
                inner.vendor_id = vendor_id;
                inner.vendor_name = vendor_name;
                inner.device = device_id;
                inner.subsystem_vendor_id = subsystem_vendor_id;
                inner.subsystem_vendor_name = subsystem_vendor_name;
                inner.subsystem_device = subsystem_device_id;
                inner.class = class;
                inner.revision = revision;
                inner.updated = true;
            } else {
                devices.push(new_pci_device(
                    pci_address,
                    vendor_id,
                    vendor_name,
                    device_id,
                    subsystem_vendor_id,
                    subsystem_vendor_name,
                    subsystem_device_id,
                    class,
                    revision,
                ));
            }
        }

        if remove_not_listed_devices {
            devices.retain_mut(|device| {
                if !device.inner.updated {
                    return false;
                }

                device.inner.updated = false;
                true
            });
        } else {
            for device in devices {
                device.inner.updated = false;
            }
        }
    }
}

//  pcidebug format: busNum:deviceNum:functionNum(secBusNum:subBusNum)
//  https://github.com/apple-oss-distributions/IOPCIFamily/blob/main/IOPCIConfigurator.cpp#L730
unsafe fn read_pci_address(entry: io_registry_entry_t, key: &CFString) -> Option<PciAddress> {
    if let Some(prop) =
        unsafe { IORegistryEntryCreateCFProperty(entry, Some(key), kCFAllocatorDefault, 0) }
        && let Ok(pcidebug) = prop.downcast::<CFString>()
    {
        let pcidebug = pcidebug.to_string();
        let address = if pcidebug.contains('(') {
            pcidebug.split_once('(').unwrap().0
        } else {
            pcidebug.as_str()
        };
        let mut parts = address.split(':');

        let bus: u8 = parts.next()?.parse().ok()?;
        let device: u8 = parts.next()?.parse().ok()?;
        let function: u8 = parts.next()?.parse().ok()?;

        Some(PciAddress {
            domain: 0, // TODO: to be checked
            bus: bus as u32,
            device: device as u32,
            function: function as u32,
        })
    } else {
        None
    }
}

unsafe fn read_property_u32(entry: io_registry_entry_t, key: &CFString) -> Option<u32> {
    let data = unsafe { read_property_data(entry, key)? };

    Some(u32::from_le_bytes([
        *data.first()?,
        *data.get(1)?,
        *data.get(2)?,
        *data.get(3)?,
    ]))
}

unsafe fn read_property_data(entry: io_registry_entry_t, key: &CFString) -> Option<Vec<u8>> {
    unsafe {
        let property = IORegistryEntryCreateCFProperty(entry, Some(key), kCFAllocatorDefault, 0)?;
        let property = property.downcast::<CFData>().ok()?;
        let property = property.as_bytes_unchecked();

        if !property.is_empty() {
            Some(property.to_vec())
        } else {
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn new_pci_device(
    address: PciAddress,
    vendor_id: u16,
    vendor_name: Option<String>,
    device: u16,
    subsystem_vendor_id: u16,
    subsystem_vendor_name: Option<String>,
    subsystem_device: u16,
    class: PciDeviceClass,
    revision: u8,
) -> PciDevice {
    PciDevice {
        inner: PciDeviceInner {
            address,
            vendor_id,
            vendor_name,
            device,
            subsystem_vendor_id,
            subsystem_vendor_name,
            subsystem_device,
            class,
            revision,
            updated: true,
        },
    }
}

impl PciDevicesInner {
    pub(crate) fn new() -> Result<Self, Error> {
        Ok(Self {
            pci_devices: Vec::new(),
        })
    }

    pub(crate) fn list(&self) -> &[PciDevice] {
        &self.pci_devices
    }

    pub(crate) fn list_mut(&mut self) -> &mut [PciDevice] {
        &mut self.pci_devices
    }

    pub(crate) fn refresh(&mut self, remove_not_listed_interfaces: bool) {
        refresh_pci_devices_from_ioreg(&mut self.pci_devices, remove_not_listed_interfaces);
    }
}

pub(crate) struct PciDeviceInner {
    address: PciAddress,
    vendor_id: u16,
    vendor_name: Option<String>,
    subsystem_vendor_id: u16,
    subsystem_vendor_name: Option<String>,
    device: u16,
    subsystem_device: u16,
    class: PciDeviceClass,
    revision: u8,
    updated: bool,
}

impl PciDeviceInner {
    pub(crate) fn address(&self) -> &PciAddress {
        &self.address
    }

    pub(crate) fn vendor_id(&self) -> u16 {
        self.vendor_id
    }

    pub(crate) fn vendor_name(&self) -> Option<&str> {
        self.vendor_name.as_deref()
    }

    pub(crate) fn device_id(&self) -> u16 {
        self.device
    }

    pub(crate) fn subsystem_vendor_id(&self) -> u16 {
        self.subsystem_vendor_id
    }

    pub(crate) fn subsystem_vendor_name(&self) -> Option<&str> {
        self.subsystem_vendor_name.as_deref()
    }

    pub(crate) fn subsystem_device_id(&self) -> u16 {
        self.subsystem_device
    }

    pub(crate) fn class(&self) -> &PciDeviceClass {
        &self.class
    }

    pub(crate) fn revision(&self) -> u8 {
        self.revision
    }

    pub(crate) fn driver(&self) -> Option<&Path> {
        None
    }
}
