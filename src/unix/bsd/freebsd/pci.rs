// Take a look at the license at the top of the repository in the LICENSE file.

use crate::unix::sys::ffi::{pci_conf, pci_conf_io};
use crate::utils::pci_vendor_name;
use crate::{Error, PciAddress, PciDevice, PciDeviceClass, PciDevicesInner};
use std::mem::MaybeUninit;
use std::os::fd::AsRawFd;

fn refresh_pci_devices_from_ioctl(devices: &mut Vec<PciDevice>, remove_not_listed_devices: bool) {
    const BUFFER_LEN: usize = 256;
    //  see note in `ffi.rs` for more information
    const PCIOCGETCONF: libc::c_ulong = libc::_IOWR::<pci_conf_io>(b'p' as libc::c_ulong, 5);

    let Ok(file) = std::fs::File::open("/dev/pci") else {
        sysinfo_debug!("Failed to open /dev/pci");
        return;
    };

    let mut current_devices = Vec::new();
    let mut offset = 0;
    let mut generation = 0;
    let mut retry_count = 0;

    while retry_count <= 3 {
        let mut buffer: Vec<MaybeUninit<pci_conf>> = Vec::with_capacity(BUFFER_LEN);

        let mut request = pci_conf_io {
            pat_buf_len: 0,
            num_patterns: 0,
            patterns: std::ptr::null_mut(),
            match_buf_len: (BUFFER_LEN * std::mem::size_of::<pci_conf>()) as u32,
            num_matches: 0,
            matches: buffer.as_mut_ptr().cast(),
            offset,
            generation,
            status: 0,
        };

        let ret = unsafe {
            libc::ioctl(
                file.as_raw_fd(),
                PCIOCGETCONF,
                &mut request as *mut pci_conf_io,
            )
        };

        if ret < 0 {
            sysinfo_debug!("PCIOCGETCONF failed");
            return;
        }

        //  Check if status is PCI_GETCONF_LIST_CHANGED
        if request.status == 1 {
            retry_count += 1;
            current_devices.clear();
            offset = 0;
            generation = 0;

            if retry_count > 3 {
                sysinfo_debug!("PCI devices kept changing");
                return;
            }

            continue;
        }

        //  Check for PCI_GETCONF_ERROR or invalid reads
        if request.status == 3 || request.num_matches > BUFFER_LEN as u32 {
            sysinfo_debug!("PCI: ioctl read failed");
            return;
        }

        unsafe {
            buffer.set_len(request.num_matches as usize);
        }

        for entry in buffer {
            let entry = unsafe { entry.assume_init() };

            let address = PciAddress {
                domain: entry.pc_sel.pc_domain,
                bus: entry.pc_sel.pc_bus,
                device: entry.pc_sel.pc_dev,
                function: entry.pc_sel.pc_func,
            };

            let vendor_id = entry.pc_vendor;
            let device_id = entry.pc_device;
            let subsystem_vendor_id = entry.pc_subvendor;
            let subsystem_device_id = entry.pc_subdevice;
            let class = PciDeviceClass {
                class: entry.pc_class,
                subclass: entry.pc_subclass,
                programming_interface: entry.pc_progif,
            };
            let revision = entry.pc_revid;

            let vendor_name = pci_vendor_name(vendor_id).map(str::to_string);
            let subsystem_vendor_name = pci_vendor_name(subsystem_vendor_id).map(str::to_string);

            current_devices.push(new_pci_device(
                address,
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

        //  Check for PCI_GETCONF_LAST_DEVICE
        if request.status == 0 {
            break;
        }
    }

    for current_device in current_devices {
        if let Some(device) = devices
            .iter_mut()
            .find(|v| v.inner.address == current_device.inner.address)
        {
            device.inner.vendor_id = current_device.inner.vendor_id;
            device.inner.vendor_name = current_device.inner.vendor_name;
            device.inner.device_id = current_device.inner.device_id;
            device.inner.subsystem_vendor_id = current_device.inner.subsystem_vendor_id;
            device.inner.subsystem_vendor_name = current_device.inner.subsystem_vendor_name;
            device.inner.subsystem_device_id = current_device.inner.subsystem_device_id;
            device.inner.class = current_device.inner.class;
            device.inner.revision = current_device.inner.revision;
            device.inner.updated = true;
        } else {
            devices.push(current_device);
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
    device_id: u16,
    subsystem_vendor_id: u16,
    subsystem_vendor_name: Option<String>,
    subsystem_device_id: u16,
    class: PciDeviceClass,
    revision: u8,
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
        refresh_pci_devices_from_ioctl(&mut self.pci_devices, remove_not_listed_interfaces);
    }
}

pub(crate) struct PciDeviceInner {
    address: PciAddress,
    vendor_id: u16,
    vendor_name: Option<String>,
    device_id: u16,
    subsystem_vendor_id: u16,
    subsystem_vendor_name: Option<String>,
    subsystem_device_id: u16,
    class: PciDeviceClass,
    revision: u8,
    updated: bool,
}

impl PciDeviceInner {
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
        Some(self.subsystem_vendor_id)
    }

    pub(crate) fn subsystem_vendor_name(&self) -> Option<&str> {
        self.subsystem_vendor_name.as_deref()
    }

    pub(crate) fn subsystem_device_id(&self) -> Option<u16> {
        Some(self.subsystem_device_id)
    }

    pub(crate) fn class(&self) -> Option<&PciDeviceClass> {
        Some(&self.class)
    }

    pub(crate) fn revision(&self) -> Option<u8> {
        Some(self.revision)
    }

    pub(crate) fn driver(&self) -> Option<&str> {
        None
    }
}
