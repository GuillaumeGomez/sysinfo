use crate::{PCIDeviceInner, PCIDevicesInner};
use std::fmt;

pub struct PCIDevices {
    pub(crate) inner: PCIDevicesInner,
}

impl PCIDevices {}

pub struct PCIDevice {
    pub(crate) inner: PCIDeviceInner,
}

impl PCIDevice {}

/// A PCI (Peripheral Component Interconnect) is an architecture used to identify and manage
/// hardware devices.
///
/// It is returned by [`Gpu::pci_address`](crate::Gpu::pci_address).
///
/// If you want to understand in details what a PCI is, I recommend:
/// <https://en.wikipedia.org/wiki/Peripheral_Component_Interconnect>.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PCIAddress {
    //  TODO correct size
    /// A PCI domain, also called "segment".
    pub domain: u32,
    /// A PCI bus.
    pub bus: u32,
    /// A PCI device.
    pub device: u32,
    /// A PCI function.
    pub function: u32,
}

impl fmt::Display for PCIAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self {
            domain,
            bus,
            device,
            function,
        } = self;
        write!(f, "{domain:04x}:{bus:02x}:{device:02x}.{function}")
    }
}

impl core::str::FromStr for PCIAddress {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn get_next_u32<'a>(
            iter: &mut impl Iterator<Item = &'a str>,
            missing_msg: &'static str,
            invalid_msg: &'static str,
        ) -> Result<u32, &'static str> {
            let Some(value) = iter.next() else {
                return Err(missing_msg);
            };
            value.parse::<u32>().map_err(|_| invalid_msg)
        }

        let mut iter = s.split(':');
        let domain = get_next_u32(&mut iter, "missing domain", "invalid domain")?;
        let bus = get_next_u32(&mut iter, "missing bus", "invalid bus")?;
        let Some(last) = iter.next() else {
            return Err("missing device");
        };
        if iter.next().is_some() {
            return Err("unexpected `:` after bus");
        };
        let mut iter = last.split('.');
        let device = get_next_u32(&mut iter, "missing device", "invalid device")?;
        let function = get_next_u32(&mut iter, "missing function", "invalid function")?;
        if iter.next().is_some() {
            return Err("unexpected `:` after function");
        };

        Ok(Self {
            domain,
            bus,
            device,
            function,
        })
    }
}
