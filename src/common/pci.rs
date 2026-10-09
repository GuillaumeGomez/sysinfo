// Take a look at the license at the top of the repository in the LICENSE file.

use crate::Error;
use crate::common::PciAddress;
use std::cmp::Ordering;
use std::path::Path;

/// Interacting with PCI (Peripheral Component Interconnect) interfaces.
///
/// ```no_run
/// use sysinfo::PciDevices;
///
/// if let Ok(pci_devices) = PciDevices::new_with_refreshed_list() {
///     for pci_device in pci_devices.list() {
///         println!("{}", pci_device.address());
///     }
/// }
/// ```
pub struct PciDevices {
    pub(crate) inner: crate::PciDevicesInner,
}

impl PciDevices {
    /// Creates a new empty [`PciDevices`][`crate::PciDevices] type.
    ///
    /// If you want it to be filled directly, take a look at [`PciDevices::new_with_refreshed_list`].
    ///
    /// ```no_run
    /// use sysinfo::PciDevices;
    ///
    /// if let Ok(&mut pci_devices) = PciDevices::new() {
    ///     pci_devices.refresh(false);
    ///     for pci_device in pci_devices.list() {
    ///         println!("{}", pci_device.address());
    ///     }
    /// }
    /// ```
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            inner: crate::PciDevicesInner::new()?,
        })
    }

    /// Creates a new [`PciDevices`][crate::PciDevices`] type with the PCI device list loaded.
    ///
    /// ```no_run
    /// use sysinfo::PciDevices;
    ///
    /// if let Ok(pci_devices) = PciDevices::new_with_refreshed_list() {
    ///     for pci_device in pci_devices.list() {
    ///         println!("{}", pci_device.address());
    ///     }
    /// }
    /// ```
    pub fn new_with_refreshed_list() -> Result<Self, Error> {
        let mut pci_devices = Self::new()?;
        pci_devices.refresh(false);
        Ok(pci_devices)
    }

    /// Returns the disks list.
    ///
    /// ```no_run
    /// use sysinfo::PciDevices;
    ///
    /// if let Ok(pci_devices) = PciDevices::new_with_refreshed_list() {
    ///     for pci_device in pci_devices.list() {
    ///         println!("{}", pci_device.address());
    ///     }
    /// }
    /// ```
    pub fn list(&self) -> &[PciDevice] {
        self.inner.list()
    }

    /// Returns the disks list.
    ///
    /// ```no_run
    /// use sysinfo::PciDevices;
    ///
    /// if let Ok(mut pci_devices) = PciDevices::new_with_refreshed_list() {
    ///     for pci_device in pci_devices.list_mut() {
    ///         println!("{}", pci_device.address());
    ///     }
    /// }
    /// ```
    pub fn list_mut(&mut self) -> &mut [PciDevice] {
        self.inner.list_mut()
    }

    /// Refreshes the PCI devices' information.
    pub fn refresh(&mut self, remove_not_listed_interfaces: bool) {
        self.inner.refresh(remove_not_listed_interfaces);
    }
}

impl From<PciDevices> for Vec<PciDevice> {
    fn from(pci_devices: PciDevices) -> Self {
        pci_devices.inner.into_vec()
    }
}

impl From<Vec<PciDevice>> for PciDevices {
    fn from(pci_devices: Vec<PciDevice>) -> Self {
        Self {
            inner: crate::PciDevicesInner::from_vec(pci_devices),
        }
    }
}

impl<'a> IntoIterator for &'a PciDevices {
    type Item = &'a PciDevice;
    type IntoIter = std::slice::Iter<'a, PciDevice>;

    fn into_iter(self) -> Self::IntoIter {
        self.list().iter()
    }
}

impl<'a> IntoIterator for &'a mut PciDevices {
    type Item = &'a mut PciDevice;
    type IntoIter = std::slice::IterMut<'a, PciDevice>;

    fn into_iter(self) -> Self::IntoIter {
        self.list_mut().iter_mut()
    }
}

impl std::ops::Deref for PciDevices {
    type Target = [PciDevice];

    fn deref(&self) -> &Self::Target {
        self.list()
    }
}

impl std::ops::DerefMut for PciDevices {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.list_mut()
    }
}

/// Type containing a PCI device's information.
///
/// ```no_run
/// use sysinfo::PciDevices;
///
/// if let Ok(pci_devices) = PciDevices::new_with_refreshed_list() {
///     for pci_device in pci_devices.list() {
///         println!("{}", pci_device.address());
///     }
/// }
/// ```
pub struct PciDevice {
    pub(crate) inner: crate::PciDeviceInner,
}

impl PciDevice {
    /// Returns the address of the PCI device.
    ///
    /// ```no_run
    /// use sysinfo::PciDevices;
    ///
    /// if let Ok(pci_devices) = PciDevices::new_with_refreshed_list() {
    ///     for pci_device in pci_devices.list() {
    ///         println!("{}", pci_device.address());
    ///     }
    /// }
    /// ```
    pub fn address(&self) -> &PciAddress {
        self.inner.address()
    }

    /// Returns the vendor ID of the PCI device.
    ///
    /// ```no_run
    /// use sysinfo::PciDevices;
    ///
    /// if let Ok(pci_devices) = PciDevices::new_with_refreshed_list() {
    ///     for pci_device in pci_devices.list() {
    ///         println!("{}", pci_device.vendor_id());
    ///     }
    /// }
    /// ```
    pub fn vendor_id(&self) -> u16 {
        self.inner.vendor_id()
    }

    /// Returns the vendor name of the PCI device.
    ///
    /// Returns `None` when the vendor ID is not found in the database.
    ///
    /// ```no_run
    /// use sysinfo::PciDevices;
    ///
    /// if let Ok(pci_devices) = PciDevices::new_with_refreshed_list() {
    ///     for pci_device in pci_devices.list() {
    ///         println!("{:?}", pci_device.vendor_name());
    ///     }
    /// }
    /// ```
    pub fn vendor_name(&self) -> Option<&str> {
        self.inner.vendor_name()
    }

    /// Returns the device ID of the PCI device.
    ///
    /// It is often used together with the vendor ID to identify the PCI device.
    ///
    /// ```no_run
    /// use sysinfo::PciDevices;
    ///
    /// if let Ok(pci_devices) = PciDevices::new_with_refreshed_list() {
    ///     for pci_device in pci_devices.list() {
    ///         println!("{}", pci_device.device_id());
    ///     }
    /// }
    /// ```
    pub fn device_id(&self) -> u16 {
        self.inner.device_id()
    }

    /// Returns the subsystem vendor ID of the PCI device.
    ///
    /// The subsystem vendor is typically the manufacturer of the subsystem (i.e. the board or the
    /// system integrating it), while the vendor is typically responsible for the device or chip.
    ///
    /// ```no_run
    /// use sysinfo::PciDevices;
    ///
    /// if let Ok(pci_devices) = PciDevices::new_with_refreshed_list() {
    ///     for pci_device in pci_devices.list() {
    ///         println!("{}", pci_device.subsystem_vendor_id());
    ///     }
    /// }
    /// ```
    pub fn subsystem_vendor_id(&self) -> u16 {
        self.inner.subsystem_vendor_id()
    }

    /// Returns the subsystem vendor name of the PCI device.
    ///
    /// Returns `None` when the subsystem vendor ID is not found in the database.
    ///
    /// The subsystem vendor is typically the manufacturer of the subsystem (i.e. the board or the
    /// system integrating it), while the vendor is typically responsible for the device or chip.
    ///
    /// ```no_run
    /// use sysinfo::PciDevices;
    ///
    /// if let Ok(pci_devices) = PciDevices::new_with_refreshed_list() {
    ///     for pci_device in pci_devices.list() {
    ///         println!("{:?}", pci_device.subsystem_vendor_name());
    ///     }
    /// }
    /// ```
    pub fn subsystem_vendor_name(&self) -> Option<&str> {
        self.inner.subsystem_vendor_name()
    }

    /// Returns the subsystem device ID of the PCI device.
    ///
    /// It is often used together with the subsystem vendor id to identify the system or board
    /// implementation PCI device.
    ///
    /// ```no_run
    /// use sysinfo::PciDevices;
    ///
    /// if let Ok(pci_devices) = PciDevices::new_with_refreshed_list() {
    ///     for pci_device in pci_devices.list() {
    ///         println!("{}", pci_device.subsystem_device_id());
    ///     }
    /// }
    /// ```
    pub fn subsystem_device_id(&self) -> u16 {
        self.inner.subsystem_device_id()
    }

    /// Returns the device class (i.e. type) of the PCI device.
    ///
    /// ```no_run
    /// use sysinfo::PciDevices;
    ///
    /// if let Ok(pci_devices) = PciDevices::new_with_refreshed_list() {
    ///     for pci_device in pci_devices.list() {
    ///         println!("{}", pci_device.class());
    ///     }
    /// }
    /// ```
    pub fn class(&self) -> &PciDeviceClass {
        self.inner.class()
    }

    /// Returns the device revision of the PCI device.
    ///
    /// ```no_run
    /// use sysinfo::PciDevices;
    ///
    /// if let Ok(pci_devices) = PciDevices::new_with_refreshed_list() {
    ///     for pci_device in pci_devices.list() {
    ///         println!("{}", pci_device.revision());
    ///     }
    /// }
    /// ```
    pub fn revision(&self) -> u8 {
        self.inner.revision()
    }

    /// Returns the path to driver used by the PCI device.
    ///
    /// ```no_run
    /// use sysinfo::PciDevices;
    ///
    /// if let Ok(pci_devices) = PciDevices::new_with_refreshed_list() {
    ///     for pci_device in pci_devices.list() {
    ///         println!("{:?}", pci_device.driver());
    ///     }
    /// }
    /// ```
    pub fn driver(&self) -> Option<&Path> {
        self.inner.driver()
    }
}

impl PartialEq for PciDevice {
    fn eq(&self, other: &Self) -> bool {
        self.address() == other.address()
    }
}

impl Eq for PciDevice {}

impl PartialOrd for PciDevice {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PciDevice {
    fn cmp(&self, other: &Self) -> Ordering {
        self.address().cmp(other.address())
    }
}

/// Class information for a PCI device.
///
/// PCI device classes are organized hierarchically: class > subclass > programming_interface.
#[derive(Clone, Copy, Debug)]
pub struct PciDeviceClass {
    /// The PCI device class, identifying the general function of the PCI device.
    pub class: u8,
    /// The PCI device subclass, providing specific classification within the class.
    pub subclass: u8,
    /// The programming_interface, providing specific classification within the subclass.
    pub programming_interface: u8,
}

impl From<u32> for PciDeviceClass {
    fn from(value: u32) -> Self {
        Self {
            class: ((value >> 16) & 0xff) as u8,
            subclass: ((value >> 8) & 0xff) as u8,
            programming_interface: (value & 0xff) as u8,
        }
    }
}

impl From<PciDeviceClass> for u32 {
    fn from(value: PciDeviceClass) -> Self {
        u32::from(&value)
    }
}

impl From<&PciDeviceClass> for u32 {
    fn from(value: &PciDeviceClass) -> Self {
        ((value.class as u32) << 16)
            | ((value.subclass as u32) << 8)
            | (value.programming_interface as u32)
    }
}

impl std::fmt::Display for PciDeviceClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:06x}", u32::from(self))
    }
}
