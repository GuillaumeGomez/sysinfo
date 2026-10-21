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

                //  M series GPUs do not use PCI. The parent of the accelerator is `AppleARMIODevice`.
                let is_m_series =
                    if IOObjectConformsTo(device.inner(), c"AppleARMIODevice".as_ptr() as *mut _) {
                        true
                    } else if IOObjectConformsTo(device.inner(), c"IOPCIDevice".as_ptr() as *mut _)
                        && is_gpu(device.inner(), &class_code_key)
                    {
                        false
                    } else {
                        continue;
                    };

                let pci = if is_m_series {
                    //  This fake PCI address is only for identification.
                    //  It cannot collide with valid PCI addresses because of device (5 bits) and function (3 bits).
                    PCI {
                        domain: 0xffffffff,
                        bus: 0xff,
                        device: 0xff,
                        function: 0xff,
                    }
                } else if let Some(prop) = IORegistryEntryCreateCFProperty(
                    device.inner(),
                    Some(&pcidebug_key),
                    kCFAllocatorDefault,
                    0,
                ) && let Ok(prop_ref) = prop.downcast::<CFString>()
                    && let pcidebug = prop_ref.to_string()
                    && let Some(pci) = parse_pcidebug(pcidebug.as_str())
                {
                    pci
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
                        accelerator.inner(),
                        Some(&model_key),
                        kCFAllocatorDefault,
                        0,
                    )
                    && let Ok(model_ref) = model.downcast::<CFString>()
                {
                    gpu.model = Some(model_ref.to_string());
                }

                if gpu.vendor.is_none()
                    && let Some(vendor_prop) = IORegistryEntryCreateCFProperty(
                        accelerator.inner(),
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

//  `pcidebug` format: either `busNum:deviceNum:functionNum` or `busNum:deviceNum:functionNum(secBusNum:subBusNum)` (decimal)
//  https://github.com/apple-oss-distributions/IOPCIFamily/blob/main/IOPCIConfigurator.cpp
fn parse_pcidebug(pcidebug: &str) -> Option<PCI> {
    let (address, _) = pcidebug.split_once('(').unwrap_or((pcidebug, ""));
    let mut parts = address.split(':');
    let bus: u8 = parts.next()?.parse().ok()?;
    let device: u8 = parts.next()?.parse().ok()?;
    let function: u8 = parts.next()?.parse().ok()?;

    if device > 0x1f {
        return None;
    }
    if function > 0x07 {
        return None;
    }

    Some(PCI {
        domain: 0,
        bus: bus as u32,
        device: device as u32,
        function: function as u32,
    })
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

#[cfg(test)]
mod tests {
    use crate::PCI;
    use crate::unix::sys::gpu::parse_pcidebug;

    #[test]
    fn test_parse_pcidebug() {
        assert_eq!(
            parse_pcidebug("1:2:3"),
            Some(PCI {
                domain: 0,
                bus: 1,
                device: 2,
                function: 3
            })
        );

        assert_eq!(
            parse_pcidebug("128:31:7"),
            Some(PCI {
                domain: 0,
                bus: 128,
                device: 31,
                function: 7
            })
        );

        assert_eq!(
            parse_pcidebug("1:2:3(4:5)"),
            Some(PCI {
                domain: 0,
                bus: 1,
                device: 2,
                function: 3
            })
        );

        assert_eq!(
            parse_pcidebug("255:31:7(1:2)"),
            Some(PCI {
                domain: 0,
                bus: 255,
                device: 31,
                function: 7
            })
        );

        assert_eq!(parse_pcidebug("256:31:7(1:2)"), None);
        assert_eq!(parse_pcidebug("128:32:7(1:2)"), None);
        assert_eq!(parse_pcidebug("128:31:8(1:2)"), None);
        assert_eq!(parse_pcidebug("ff:1f:07"), None);
        assert_eq!(parse_pcidebug("::"), None);
    }
}
