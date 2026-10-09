// Take a look at the license at the top of the repository in the LICENSE file.

use crate::unix::linux::utils::read_hex;
use crate::utils::pci_vendor_name;
use crate::{Error, PciAddress, PciDevice, PciDeviceClass, PciDevicesInner};
use std::path::{Path, PathBuf};

fn refresh_pci_devices_list_from_sysfs(
    devices: &mut Vec<PciDevice>,
    remove_not_listed_devices: bool,
    sysfs_bus_pci_devices: &Path,
) {
    let Ok(sys_dir) = std::fs::read_dir(sysfs_bus_pci_devices) else {
        return;
    };

    let mut num_buf = [0u8; (u64::MAX.ilog(16) + 2) as usize + 2];

    for entry in sys_dir.flatten() {
        let parent = &entry.path();
        let Some(address) = &entry
            .file_name()
            .to_string_lossy()
            .parse::<PciAddress>()
            .ok()
        else {
            continue;
        };

        let vendor = read_hex(parent, "vendor", &mut num_buf) as u16;
        let vendor_name = pci_vendor_name(vendor).map(|v| v.to_string());
        let device = read_hex(parent, "device", &mut num_buf) as u16;
        let subsystem_vendor = read_hex(parent, "subsystem_vendor", &mut num_buf) as u16;
        let subsystem_vendor_name = pci_vendor_name(vendor).map(|v| v.to_string());
        let subsystem_device = read_hex(parent, "subsystem_device", &mut num_buf) as u16;
        let class = PciDeviceClass::from(read_hex(parent, "class", &mut num_buf) as u32);
        let revision = read_hex(parent, "revision", &mut num_buf) as u8;

        let driver = std::fs::canonicalize(entry.path().join("driver")).ok();

        if let Some(existing_device) = devices.iter_mut().find(|d| d.inner.address == *address) {
            let inner = &mut existing_device.inner;

            inner.vendor_id = vendor;
            inner.vendor_name = vendor_name;
            inner.device = device;
            inner.subsystem_vendor_id = subsystem_vendor;
            inner.subsystem_vendor_name = subsystem_vendor_name;
            inner.subsystem_device = subsystem_device;
            inner.class = class;
            inner.revision = revision;
            inner.driver = driver;
            inner.updated = true;
        } else {
            devices.push(new_pci_device(
                address.clone(),
                vendor,
                vendor_name,
                device,
                subsystem_vendor,
                subsystem_vendor_name,
                subsystem_device,
                class,
                revision,
                driver,
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
    driver: Option<PathBuf>,
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
            driver,
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
        refresh_pci_devices_list_from_sysfs(
            &mut self.pci_devices,
            remove_not_listed_interfaces,
            "/sys/bus/pci/devices".as_ref(),
        );
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
    driver: Option<PathBuf>,
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
        self.driver.as_deref()
    }
}
