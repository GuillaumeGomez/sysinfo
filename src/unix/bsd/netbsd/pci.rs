// Take a look at the license at the top of the repository in the LICENSE file.

use crate::unix::bsd::ffi::{PCI_IO_DRVNAME_LEN, pci_drvname};
use crate::unix::sys::ffi::{
    PCI_BHLC_REG, PCI_CLASS_REG, PCI_HDRTYPE_MASK, PCI_HDRTYPE_SHIFT, PCI_ID_REG, PCI_IOC_BUSINFO,
    PCI_SUBSYS_ID_REG, pcibus_conf_read, pciio_businfo,
};
use crate::utils::pci_vendor_name;
use crate::{Error, PciAddress, PciDevice, PciDeviceClass, PciDevicesInner};
use libc::{c_char, ioctl};
use std::ffi::CStr;
use std::fs::File;
use std::os::fd::AsRawFd;

pub fn refresh_pci_device(devices: &mut Vec<PciDevice>, remove_not_listed_devices: bool) {
    let Ok(dir) = std::fs::read_dir("/dev") else {
        sysinfo_debug!("Failed to open /dev");
        return;
    };

    let mut checked_bus = [false; 256];

    for entry in dir.flatten() {
        let filename = entry.file_name();
        if !filename.to_string_lossy().trim().starts_with("pci") {
            continue;
        }

        let Ok(interface) = File::open(entry.path()) else {
            continue;
        };

        let mut pci_bus_info: pciio_businfo = unsafe { std::mem::zeroed() };

        let ret = unsafe { ioctl(interface.as_raw_fd(), PCI_IOC_BUSINFO, &mut pci_bus_info) };
        if ret < 0 {
            sysinfo_debug!("PCI_IOC_BUSINFO failed");
            return;
        }

        if pci_bus_info.busno >= 256 || checked_bus[pci_bus_info.busno as usize] {
            continue;
        }
        checked_bus[pci_bus_info.busno as usize] = true;

        let bus = pci_bus_info.busno;

        for device in 0..pci_bus_info.maxdev {
            let mut bhlcr: u32 = 0;

            let ret = unsafe {
                pcibus_conf_read(
                    interface.as_raw_fd(),
                    bus,
                    device,
                    0,
                    PCI_BHLC_REG,
                    &mut bhlcr,
                )
            };
            if ret != 0 {
                continue;
            }

            //  PCI_HDRTYPE_MULTIFN(bhlcr)
            let func_max = if ((((bhlcr) >> PCI_HDRTYPE_SHIFT) & PCI_HDRTYPE_MASK) & 0x80) != 0 {
                7
            } else {
                0
            };

            for func in 0..=func_max {
                let mut id: u32 = 0;

                let ret = unsafe {
                    pcibus_conf_read(
                        interface.as_raw_fd(),
                        bus,
                        device,
                        func,
                        PCI_ID_REG,
                        &mut id,
                    )
                };
                if ret != 0 {
                    continue;
                }

                let hdrtype_type = (((bhlcr) >> PCI_HDRTYPE_SHIFT) & PCI_HDRTYPE_MASK) & 0x7f;
                let subsystem_id = if hdrtype_type == 0 {
                    let mut subsysid: u32 = 0;

                    let ret = unsafe {
                        pcibus_conf_read(
                            interface.as_raw_fd(),
                            bus,
                            device,
                            func,
                            PCI_SUBSYS_ID_REG,
                            &mut subsysid,
                        )
                    };
                    if ret != 0 {
                        continue;
                    }

                    Some(subsysid)
                } else {
                    //  PCI_SUBSYS_ID_REG collides another register for type 1 devices
                    None
                };

                let mut class: u32 = 0;

                let ret = unsafe {
                    pcibus_conf_read(
                        interface.as_raw_fd(),
                        bus,
                        device,
                        func,
                        PCI_CLASS_REG,
                        &mut class,
                    )
                };
                if ret != 0 {
                    continue;
                }

                let mut driver_buffer = [0 as c_char; PCI_IO_DRVNAME_LEN + 1];
                let ret = unsafe {
                    pci_drvname(
                        interface.as_raw_fd(),
                        device,
                        func,
                        driver_buffer.as_mut_ptr(),
                        PCI_IO_DRVNAME_LEN,
                    )
                };
                let driver = if ret == 0 {
                    Some(unsafe {
                        CStr::from_ptr(driver_buffer.as_ptr())
                            .to_string_lossy()
                            .into_owned()
                    })
                } else {
                    None
                };

                let address = PciAddress {
                    domain: 0, //  `pcictl` does not provide domain. Probably does not exist on NetBSD.
                    bus,
                    device,
                    function: func,
                };

                let revision = class as u8;
                let class = PciDeviceClass::from(class >> 8);

                let vendor_id = id as u16;
                let device_id = (id >> 16) as u16;

                let subsystem_vendor_id = subsystem_id.map(|v| v as u16);
                let subsystem_device_id = subsystem_id.map(|v| (v >> 16) as u16);

                let vendor_name = pci_vendor_name(vendor_id).map(str::to_string);
                let subsystem_vendor_name = subsystem_vendor_id
                    .and_then(pci_vendor_name)
                    .map(str::to_string);

                if let Some(device) = devices.iter_mut().find(|v| v.address() == &address) {
                    let inner = &mut device.inner;
                    inner.vendor_id = vendor_id;
                    inner.vendor_name = vendor_name;
                    inner.device_id = device_id;
                    inner.subsystem_vendor_id = subsystem_vendor_id;
                    inner.subsystem_vendor_name = subsystem_vendor_name;
                    inner.subsystem_device_id = subsystem_device_id;
                    inner.class = class;
                    inner.revision = revision;
                    inner.driver = driver;
                    inner.updated = true;
                } else {
                    devices.push(new_pci_device(
                        address,
                        vendor_id,
                        vendor_name,
                        device_id,
                        subsystem_vendor_id,
                        subsystem_vendor_name,
                        subsystem_device_id,
                        class,
                        revision,
                        driver,
                    ));
                }
            }
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
        refresh_pci_device(&mut self.pci_devices, remove_not_listed_interfaces);
    }
}

#[allow(clippy::too_many_arguments)]
fn new_pci_device(
    address: PciAddress,
    vendor_id: u16,
    vendor_name: Option<String>,
    device_id: u16,
    subsystem_vendor_id: Option<u16>,
    subsystem_vendor_name: Option<String>,
    subsystem_device_id: Option<u16>,
    class: PciDeviceClass,
    revision: u8,
    driver: Option<String>,
) -> PciDevice {
    PciDevice {
        inner: PciDeviceInner {
            address,
            vendor_id,
            vendor_name,
            device_id,
            subsystem_vendor_id,
            subsystem_vendor_name,
            subsystem_device_id,
            class,
            revision,
            driver,
            updated: true,
        },
    }
}

pub(crate) struct PciDeviceInner {
    address: PciAddress,
    vendor_id: u16,
    vendor_name: Option<String>,
    device_id: u16,
    subsystem_vendor_id: Option<u16>,
    subsystem_vendor_name: Option<String>,
    subsystem_device_id: Option<u16>,
    class: PciDeviceClass,
    revision: u8,
    driver: Option<String>,
    updated: bool,
}

impl crate::sys::PciDeviceInner {
    pub(crate) fn address(&self) -> &PciAddress {
        &self.address
    }

    pub(crate) fn vendor_id(&self) -> Option<u16> {
        Some(self.vendor_id)
    }

    pub(crate) fn vendor_name(&self) -> Option<&str> {
        self.vendor_name.as_deref()
    }

    pub(crate) fn device_id(&self) -> Option<u16> {
        Some(self.device_id)
    }

    pub(crate) fn subsystem_vendor_id(&self) -> Option<u16> {
        self.subsystem_vendor_id
    }

    pub(crate) fn subsystem_vendor_name(&self) -> Option<&str> {
        self.subsystem_vendor_name.as_deref()
    }

    pub(crate) fn subsystem_device_id(&self) -> Option<u16> {
        self.subsystem_device_id
    }

    pub(crate) fn class(&self) -> Option<&PciDeviceClass> {
        Some(&self.class)
    }

    pub(crate) fn revision(&self) -> Option<u8> {
        Some(self.revision)
    }

    pub(crate) fn driver(&self) -> Option<&str> {
        self.driver.as_deref()
    }
}
