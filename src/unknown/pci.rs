// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{Error, PciAddress, PciDevice, PciDeviceClass};
use std::path::Path;

pub(crate) struct PciDevicesInner;

impl PciDevicesInner {
    pub(crate) fn new() -> Result<Self, Error> {
        Err(Error::Unsupported)
    }

    pub(crate) fn from_vec(_: Vec<PciDevice>) -> Self {
        Self
    }

    pub(crate) fn into_vec(self) -> Vec<PciDevice> {
        Vec::new()
    }

    pub(crate) fn list(&self) -> &[PciDevice] {
        &[]
    }

    pub(crate) fn list_mut(&mut self) -> &mut [PciDevice] {
        &mut []
    }

    pub(crate) fn refresh(&mut self, _: bool) {}
}

pub(crate) struct PciDeviceInner {}

impl PciDeviceInner {
    pub(crate) fn address(&self) -> &PciAddress {
        unreachable!()
    }

    pub(crate) fn vendor_id(&self) -> u16 {
        unreachable!()
    }

    pub(crate) fn vendor_name(&self) -> Option<&str> {
        unreachable!()
    }

    pub(crate) fn device_id(&self) -> u16 {
        unreachable!()
    }

    pub(crate) fn subsystem_vendor_id(&self) -> u16 {
        unreachable!()
    }

    pub(crate) fn subsystem_vendor_name(&self) -> Option<&str> {
        unreachable!()
    }

    pub(crate) fn subsystem_device_id(&self) -> u16 {
        unreachable!()
    }

    pub(crate) fn class(&self) -> &PciDeviceClass {
        unreachable!()
    }

    pub(crate) fn revision(&self) -> u8 {
        unreachable!()
    }

    pub(crate) fn driver(&self) -> Option<&Path> {
        unreachable!()
    }
}
