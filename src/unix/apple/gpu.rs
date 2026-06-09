// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::utils::{IOReleaser, MAIN_PORT};
use crate::{Gpu, PCI};

use objc2_core_foundation::{
    CFData, CFDictionary, CFNumber, CFRange, CFRetained, CFString, kCFAllocatorDefault,
};
use objc2_io_kit::{
    IOIteratorNext, IOObjectConformsTo, IORegistryEntryCreateCFProperty,
    IORegistryEntryGetParentEntry, IOServiceGetMatchingServices, IOServiceMatching, io_iterator_t,
    io_object_t, kIOReturnSuccess, kIOServicePlane,
};
use std::ffi::CStr;

pub(crate) struct GpusInner {
    pub(crate) gpus: Vec<Gpu>,
}

impl GpusInner {
    pub(crate) fn new() -> Result<Self, crate::Error> {
        Ok(Self { gpus: Vec::new() })
    }

    pub(crate) fn refresh(&mut self) {
        unsafe {
            let Some(matching) = IOServiceMatching(c"IOAccelerator".as_ptr()) else {
                sysinfo_debug!("Failed to create IOAccelerator matching dictionary");
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
                    "GPU error: IOServiceGetMatchingServices succeeded but returned invalid descriptor"
                );
                return;
            };

            let model_key = CFString::from_str("model");
            let vendor_id_key = CFString::from_str("vendor-id");
            let class_code_key = CFString::from_str("class-code");
            let pcidebug_key = CFString::from_str("pcidebug");
            let perf_key = CFString::from_str("PerformanceStatistics");

            while let Some(accelerator) = IOReleaser::new(IOIteratorNext(iterator.inner())) {
                let mut device = 0;
                let ret = IORegistryEntryGetParentEntry(
                    accelerator.inner(),
                    kIOServicePlane.as_ptr() as *mut _,
                    &mut device,
                );
                if ret != kIOReturnSuccess || device == 0 {
                    continue;
                }
                let Some(device) = IOReleaser::new(device) else {
                    continue;
                };
                if !IOObjectConformsTo(device.inner(), c"IOPCIDevice".as_ptr() as *mut _)
                    || !is_gpu(device.inner(), &class_code_key)
                {
                    continue;
                }

                let pci = if let Some(prop) = IORegistryEntryCreateCFProperty(
                    device.inner(),
                    Some(&pcidebug_key),
                    kCFAllocatorDefault,
                    0,
                ) && let Ok(prop_ref) = prop.downcast::<CFData>()
                    && let [domain, bus, device, func, ..] = prop_ref.as_bytes_unchecked()
                {
                    PCI {
                        domain: *domain as _,
                        bus: *bus as _,
                        device: *device as _,
                        function: *func as _,
                    }
                } else {
                    continue;
                };

                let gpu = match self.gpus.iter_mut().find(|g| g.inner.pci == pci) {
                    Some(g) => {
                        g.inner.updated = true;
                        &mut g.inner
                    }
                    None => {
                        self.gpus.push(Gpu {
                            inner: GpuInner {
                                pci,
                                model: None,
                                vendor: None,
                                usage: None,
                                updated: true,
                            },
                        });
                        &mut self.gpus.last_mut().unwrap().inner
                    }
                };

                if gpu.model.is_none()
                    && let Some(model) = IORegistryEntryCreateCFProperty(
                        device.inner(),
                        Some(&model_key),
                        kCFAllocatorDefault,
                        0,
                    )
                    && let Ok(model_ref) = model.downcast::<CFData>()
                    && let bytes = model_ref.as_bytes_unchecked()
                    && !bytes.is_empty()
                    && let Ok(c_str) = CStr::from_bytes_with_nul(bytes)
                {
                    gpu.model = Some(c_str.to_string_lossy().into_owned());
                }

                if gpu.vendor.is_none()
                    && let Some(vendor_prop) = IORegistryEntryCreateCFProperty(
                        device.inner(),
                        Some(&vendor_id_key),
                        kCFAllocatorDefault,
                        0,
                    )
                    && let Ok(vendor_ref) = vendor_prop.downcast::<CFData>()
                    && vendor_ref.length() >= 4
                {
                    let mut vendor_id: u32 = 0;
                    vendor_ref.bytes(
                        CFRange {
                            location: 0,
                            length: 4,
                        },
                        &mut vendor_id as *mut _ as *mut _,
                    );
                    gpu.vendor = crate::utils::gpu_vendor_name(vendor_id).map(|s| s.to_owned());
                }

                if let Some(usage) = IORegistryEntryCreateCFProperty(
                    accelerator.inner(),
                    Some(&perf_key),
                    kCFAllocatorDefault,
                    0,
                ) && let Some(usage_dict) = usage.downcast_ref::<CFDictionary>()
                    && let usage_dict = usage_dict.cast_unchecked::<CFString, CFNumber>()
                    && let Some(usage) = usage_dict
                        .get(&CFString::from_str("Device Utilization %"))
                        .or_else(|| usage_dict.get(&CFString::from_str("Device Utilization")))
                    && let Ok(usage) = usage.downcast::<CFNumber>()
                    && let Some(usage) = usage.as_i64()
                {
                    gpu.usage = Some(usage as f32);
                }
            }
        }
    }
}

fn vendor_name(vendor_id: u32) -> Option<String> {
    // Very restricted list. To get more, take a look at the `pci.ids` index.
    Some(
        match vendor_id {
            0x106B => "Apple Inc.",
            0x1002 => "Advanced Micro Devices, Inc. [AMD/ATI]",
            0x10DE => "NVIDIA Corporation",
            0x8086 => "Intel Corporation",
            0x13b5 => "ARM",
            0x168c | 0x1969 => "Qualcomm Atheros",
            0x5143 => "Qualcomm Inc",
            0x17cb => "Qualcomm Technologies, Inc",
            _ => return None,
        }
        .to_owned(),
    )
}

unsafe fn is_gpu(device: io_object_t, class_code_key: &CFString) -> bool {
    unsafe {
        if let Some(prop) =
            IORegistryEntryCreateCFProperty(device, Some(class_code_key), kCFAllocatorDefault, 0)
            && let Ok(prop_ref) = prop.downcast::<CFData>()
            // Standard PCI layout in Open Firmware maps class code to bytes [1] and [2]
            // or a raw 32-bit integer where the high/mid bytes dictate the base class.
            //
            // Base class 0x03 = Display Controller
            && let [_, _, 0x03, ..] = prop_ref.as_bytes_unchecked()
        {
            true
        } else {
            false
        }
    }
}

pub(crate) struct GpuInner {
    pci: PCI,
    vendor: Option<String>,
    model: Option<String>,
    usage: Option<f32>,
    pub(crate) updated: bool,
}

impl GpuInner {
    pub(crate) fn pci(&self) -> &PCI {
        &self.pci
    }
    pub(crate) fn vendor(&self) -> Option<&str> {
        self.vendor.as_deref()
    }
    pub(crate) fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }
    pub(crate) fn usage(&self) -> Option<f32> {
        self.usage
    }
    pub(crate) fn total_memory(&self) -> Option<u64> {
        None
    }
    pub(crate) fn used_memory(&self) -> Option<u64> {
        None
    }
}
