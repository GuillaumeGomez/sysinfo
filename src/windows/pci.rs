// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{Error, PciAddress, PciDevice, PciDeviceClass};
use std::path::Path;

pub(crate) struct PciDevicesInner {
    pub(crate) pci_devices: Vec<PciDevice>,
}

impl PciDevicesInner {
    pub(crate) fn from_vec(pci_devices: Vec<PciDevice>) -> Self {
        Self { pci_devices }
    }

    pub(crate) fn into_vec(self) -> Vec<PciDevice> {
        self.pci_devices
    }

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
        todo!()
    }
}

pub(crate) struct PciDeviceInner {}

impl PciDeviceInner {
    pub(crate) fn address(&self) -> &PciAddress {
        todo!()
    }

    pub(crate) fn vendor_id(&self) -> u16 {
        todo!()
    }

    pub(crate) fn vendor_name(&self) -> Option<&str> {
        todo!()
    }

    pub(crate) fn device_id(&self) -> u16 {
        todo!()
    }

    pub(crate) fn subsystem_vendor_id(&self) -> u16 {
        todo!()
    }

    pub(crate) fn subsystem_vendor_name(&self) -> Option<&str> {
        todo!()
    }

    pub(crate) fn subsystem_device_id(&self) -> u16 {
        todo!()
    }

    pub(crate) fn class(&self) -> &PciDeviceClass {
        todo!()
    }

    pub(crate) fn revision(&self) -> u8 {
        todo!()
    }

    pub(crate) fn driver(&self) -> Option<&Path> {
        todo!()
    }
}
