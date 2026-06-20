// Take a look at the license at the top of the repository in the LICENSE file.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::{File, read_dir};
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;

use crate::{Gpu, PCI};

pub(crate) struct GpusInner {
    pub(crate) gpus: Vec<Gpu>,
    nvm_lib: Option<self::nvidia::NvmlLib>,
    vulkan_lib: Option<self::vulkan::Vulkan>,
    // The key is the vendor ID.
    device_map: HashMap<u32, Devices>,
    filled_device_map: bool,
}

struct Devices {
    name: String,
    // One ID can have multiple devices but we ignore it.
    devices: HashMap<u32, String>,
}

impl Devices {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            devices: HashMap::new(),
        }
    }
}

pub(crate) struct GpuInner {
    total_memory: Option<u64>,
    used_memory: Option<u64>,
    usage: Option<f32>,
    model: Option<String>,
    vendor: String,
    pci: PCI,
    pub(crate) updated: bool,
}

impl GpuInner {
    fn new(pci: PCI, vendor: &str, model: Option<&String>) -> Self {
        Self {
            total_memory: None,
            used_memory: None,
            usage: None,
            model: model.cloned(),
            vendor: vendor.to_owned(),
            pci,
            updated: true,
        }
    }

    pub(crate) fn pci(&self) -> &PCI {
        &self.pci
    }
    pub(crate) fn vendor(&self) -> Option<&str> {
        Some(self.vendor.as_str())
    }
    pub(crate) fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }
    pub(crate) fn usage(&self) -> Option<f32> {
        self.usage
    }
    pub(crate) fn total_memory(&self) -> Option<u64> {
        self.total_memory
    }
    pub(crate) fn used_memory(&self) -> Option<u64> {
        self.used_memory
    }
}

impl GpusInner {
    pub(crate) fn new() -> Result<Self, crate::Error> {
        Ok(Self {
            gpus: Vec::new(),
            nvm_lib: None,
            device_map: HashMap::new(),
            vulkan_lib: None,
            filled_device_map: false,
        })
    }

    /// Returns the vendor name and the vendor's known devices.
    fn get_vendor_and_devices(
        &self,
        vendor_id: u32,
    ) -> Option<(&str, Option<&HashMap<u32, String>>)> {
        match self.device_map.get(&vendor_id) {
            Some(devices) => Some((&devices.name, Some(&devices.devices))),
            None => crate::utils::gpu_vendor_name(vendor_id).map(|vendor| (vendor, None)),
        }
    }

    pub(crate) fn refresh(&mut self) {
        let mut need_vulkan = false;
        let mut need_nvidia = false;
        self.get_non_nvidia_gpus(&mut need_vulkan, &mut need_nvidia);
        if need_nvidia {
            self.nvm_lib = unsafe { self::nvidia::NvmlLib::load() };
            if let Some(nvml) = &self.nvm_lib {
                nvml.get_nvidia_gpus(&mut self.gpus);
            }
        }
        if need_vulkan {
            if self.vulkan_lib.is_none() {
                self.vulkan_lib = vulkan::Vulkan::load();
            }
            if let Some(ref vulkan) = self.vulkan_lib {
                vulkan.get_vulkan_memory(&mut self.gpus);
            }
        }
    }

    fn get_non_nvidia_gpus(&mut self, need_vulkan: &mut bool, need_nvidia: &mut bool) {
        let drm_path = "/sys/class/drm";
        let Ok(items) = read_dir(drm_path) else {
            sysinfo_debug!("failed to read `{drm_path}`");
            return;
        };
        let mut buffer = String::new();
        for entry in items.flatten() {
            let name = entry.file_name();
            if !name.as_encoded_bytes().starts_with(b"card")
                || name.as_encoded_bytes().contains(&b'-')
            {
                continue;
            }
            let device = entry.path().join("device");
            let Some(pci) = device
                .read_link()
                .ok()
                .and_then(|path| path.file_name()?.to_string_lossy().parse::<PCI>().ok())
            else {
                continue;
            };
            // If this GPU is already in our GPU list, no need to re-add it.
            if let Some(gpu) = self.gpus.iter_mut().find(|gpu| gpu.inner.pci == pci) {
                if gpu.inner.vendor.contains("AMD") {
                    get_amd_info(&mut gpu.inner, &mut buffer, &device);
                }
                // Since NVIDIA GPUs are updated outside of this function, we don't change the
                // `updated` field for them here.
                if !gpu.inner.vendor.contains("NVIDIA") {
                    gpu.inner.updated = true;
                } else {
                    *need_nvidia = true;
                }
                continue;
            }
            // We filter out everything that isn't a GPU. Refer to the end of the
            // `/usr/share/hwdata/pci.ids` file (where lines start with "C ") to see the list of
            // all kinds of devices.
            if read_file(device.join("class"), &mut buffer).is_err() {
                continue;
            }
            if !buffer.trim().starts_with("0x03") {
                // Not a GPU.
                continue;
            }
            if read_file(device.join("vendor"), &mut buffer).is_err() {
                continue;
            }
            let Some(vendor) = buffer
                .trim()
                .strip_prefix("0x")
                .and_then(|buf| u32::from_str_radix(buf, 16).ok())
            else {
                continue;
            };
            if !self.filled_device_map {
                fill_device_map(&mut self.device_map, &mut buffer);
                self.filled_device_map = true;
            }
            if let Some((vendor, devices)) = self.get_vendor_and_devices(vendor) {
                if vendor.contains("NVIDIA") {
                    // NVIDIA GPUs are retrieved elsewhere.
                    *need_nvidia = true;
                    continue;
                }
                let model = devices.and_then(|devices| {
                    read_file(device.join("device"), &mut buffer)
                        .ok()
                        .and_then(|_| {
                            u32::from_str_radix(buffer.trim().strip_prefix("0x")?, 16).ok()
                        })
                        .and_then(|device_id| devices.get(&device_id))
                });
                let mut gpu = GpuInner::new(pci, vendor, model);
                if vendor.contains("AMD") {
                    // We special-case AMD since we know how to retrieve its information with the
                    // sysfs only.
                    get_amd_info(&mut gpu, &mut buffer, &device);
                } else {
                    *need_vulkan = true;
                }
                self.gpus.push(Gpu { inner: gpu });
            }
        }
    }
}

fn read_file<P: AsRef<Path>>(path: P, buffer: &mut String) -> io::Result<usize> {
    buffer.clear();
    let mut f = File::open(path)?;

    f.read_to_string(buffer)
}

struct CustomBufReader<'a> {
    buffer: &'a mut String,
    reader: BufReader<File>,
}

impl<'a> Iterator for CustomBufReader<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.buffer.clear();
            if self.reader.read_line(self.buffer).ok()? == 0 {
                return None;
            } else if self.buffer.starts_with("C ") {
                // We reached the end of the device list.
                return None;
            } else if !self.buffer.starts_with('#') {
                // SAFETY: This is because of `Iterator` trait limitations: we cannot return a
                // reference to the type's fields because that would require changing `next` to
                // `fn next(&'a mut self) -> ...`, which isn't allowed by the compiler. So instead,
                // we `transmute`...
                return Some(unsafe { std::mem::transmute::<&str, &'a str>(self.buffer.as_str()) });
            }
        }
    }
}

#[allow(clippy::while_let_on_iterator)]
fn find_first_vendor(reader: &mut CustomBufReader<'_>) -> Option<(u32, Devices)> {
    while let Some(line) = reader.next() {
        // We ignore everything that isn't a vendor.
        if line.starts_with('\t') {
            continue;
        }
        if let Some((id, vendor_name)) = get_id_and_name(line) {
            return Some((id, Devices::new(vendor_name)));
        }
    }
    None
}

fn get_id_and_name(line: &str) -> Option<(u32, &str)> {
    let (id, name) = line.split_once(' ')?;
    let id = u32::from_str_radix(id, 16).ok()?;
    Some((id, name.trim()))
}

fn fill_device_map(device_map: &mut HashMap<u32, Devices>, buffer: &mut String) {
    let Ok(f) = File::open("/usr/share/hwdata/pci.ids") else {
        return;
    };
    let mut reader = CustomBufReader {
        buffer,
        reader: BufReader::new(f),
    };
    let Some((mut vendor_id, mut devices)) = find_first_vendor(&mut reader) else {
        return;
    };

    for line in reader {
        match line.strip_prefix('\t') {
            None => {
                if let Some((id, vendor_name)) = get_id_and_name(line) {
                    device_map.insert(vendor_id, devices);
                    vendor_id = id;
                    devices = Devices::new(vendor_name);
                }
            }
            Some(line) => {
                // We ignore sub-systems.
                if !line.starts_with('\t')
                    && let Some((id, model_name)) = get_id_and_name(line)
                {
                    devices.devices.insert(id, model_name.to_owned());
                }
            }
        }
    }
}

fn get_amd_info(gpu: &mut GpuInner, buffer: &mut String, path: &Path) {
    if read_file(path.join("gpu_busy_percent"), buffer).is_ok() {
        gpu.usage = buffer.trim().parse::<f32>().ok();
    }
    if read_file(path.join("mem_info_vram_used"), buffer).is_ok() {
        gpu.used_memory = buffer.trim().parse::<u64>().ok();
    }
    if read_file(path.join("mem_info_vram_total"), buffer).is_ok() {
        gpu.total_memory = buffer.trim().parse::<u64>().ok();
    }
}

macro_rules! load_sym {
    ($lib_handle:ident, $sym_name:literal, $cast_into:ty $(,)?) => {{
        let sym_name = $sym_name;
        let sym = libc::dlsym($lib_handle, sym_name.as_ptr());
        if sym.is_null() {
            sysinfo_debug!("Failed to find symbol: {sym_name:?}");
            None
        } else {
            Some(std::mem::transmute::<*mut c_void, $cast_into>(sym))
        }
    }};
}

fn convert_to_string(data: &[libc::c_char]) -> Option<String> {
    data.split(|c| *c == 0).next().map(|s| unsafe {
        let s: &[u8] = std::slice::from_raw_parts(s.as_ptr() as *const u8, s.len());
        String::from_utf8_lossy(s).into_owned()
    })
}

fn convert_to_str(data: &[libc::c_char]) -> Option<Cow<'_, str>> {
    data.split(|c| *c == 0).next().map(|s| unsafe {
        let s: &[u8] = std::slice::from_raw_parts(s.as_ptr() as *const u8, s.len());
        String::from_utf8_lossy(s)
    })
}

mod nvidia {
    use super::*;
    use libc::{c_char, c_int, c_uint, c_ulonglong, c_void};
    use std::mem::MaybeUninit;
    use std::ptr::null_mut;

    #[repr(C)]
    struct NvmlUtilization {
        gpu: c_uint,
        memory: c_uint,
    }

    #[repr(C)]
    struct NvmlMemory {
        total: c_ulonglong,
        free: c_ulonglong,
        used: c_ulonglong,
    }

    #[repr(C)]
    struct NvmlPciInfo {
        bus_id_legacy: [c_char; 16],
        domain: c_uint,
        bus: c_uint,
        device: c_uint,
        pci_device_id: c_uint,
        pci_sub_system_id: c_uint,
        bus_id: [c_char; 32],
    }

    type NvmlReturn = c_int;
    type NvmlDeviceHandle = *mut c_void;

    type NvmlInitFn = unsafe extern "C" fn() -> NvmlReturn;
    type NvmlShutdownFn = unsafe extern "C" fn() -> NvmlReturn;
    type NvmlDeviceGetCountFn = unsafe extern "C" fn(*mut c_uint) -> NvmlReturn;
    type NvmlDeviceGetHandleByIndexFn =
        unsafe extern "C" fn(c_uint, *mut NvmlDeviceHandle) -> NvmlReturn;
    type NvmlDeviceGetNameFn =
        unsafe extern "C" fn(NvmlDeviceHandle, *mut c_char, c_uint) -> NvmlReturn;
    type NvmlDeviceGetUtilizationRatesFn =
        unsafe extern "C" fn(NvmlDeviceHandle, *mut NvmlUtilization) -> NvmlReturn;
    type NvmlDeviceGetMemoryInfoFn =
        unsafe extern "C" fn(NvmlDeviceHandle, *mut NvmlMemory) -> NvmlReturn;
    type NvmlDeviceGetPciInfo =
        unsafe extern "C" fn(NvmlDeviceHandle, *mut NvmlPciInfo) -> NvmlReturn;

    const NVML_SUCCESS: NvmlReturn = 0;
    const NVML_DEVICE_NAME_V2_BUFFER_SIZE: usize = 96;

    pub(crate) struct NvmlLib {
        lib_handle: *mut c_void,
        init: NvmlInitFn,
        shutdown: NvmlShutdownFn,
        get_count: NvmlDeviceGetCountFn,
        get_handle: NvmlDeviceGetHandleByIndexFn,
        get_name: NvmlDeviceGetNameFn,
        get_utilization: NvmlDeviceGetUtilizationRatesFn,
        get_memory_info: NvmlDeviceGetMemoryInfoFn,
        get_pci_info: NvmlDeviceGetPciInfo,
    }

    impl NvmlLib {
        pub(crate) unsafe fn load() -> Option<Self> {
            let lib_name = c"libnvidia-ml.so.1";
            unsafe {
                let lib_handle = libc::dlopen(lib_name.as_ptr(), libc::RTLD_NOW);
                if lib_handle.is_null() {
                    sysinfo_debug!("Failed to find or load {lib_name:?}");
                    return None;
                }
                match Self::load_symbols(lib_handle) {
                    Some(ret) => Some(ret),
                    None => {
                        libc::dlclose(lib_handle);
                        None
                    }
                }
            }
        }

        unsafe fn load_symbols(lib_handle: *mut c_void) -> Option<Self> {
            unsafe {
                Some(Self {
                    lib_handle,
                    init: load_sym!(lib_handle, c"nvmlInit_v2", NvmlInitFn)?,
                    shutdown: load_sym!(lib_handle, c"nvmlShutdown", NvmlShutdownFn)?,
                    get_count: load_sym!(
                        lib_handle,
                        c"nvmlDeviceGetCount_v2",
                        NvmlDeviceGetCountFn,
                    )?,
                    get_handle: load_sym!(
                        lib_handle,
                        c"nvmlDeviceGetHandleByIndex_v2",
                        NvmlDeviceGetHandleByIndexFn,
                    )?,
                    get_name: load_sym!(lib_handle, c"nvmlDeviceGetName", NvmlDeviceGetNameFn)?,
                    get_utilization: load_sym!(
                        lib_handle,
                        c"nvmlDeviceGetUtilizationRates",
                        NvmlDeviceGetUtilizationRatesFn,
                    )?,
                    get_memory_info: load_sym!(
                        lib_handle,
                        c"nvmlDeviceGetMemoryInfo",
                        NvmlDeviceGetMemoryInfoFn,
                    )?,
                    get_pci_info: load_sym!(
                        lib_handle,
                        c"nvmlDeviceGetPciInfo_v3",
                        NvmlDeviceGetPciInfo,
                    )?,
                })
            }
        }

        pub(crate) fn get_nvidia_gpus(&self, gpus: &mut Vec<Gpu>) {
            unsafe {
                if let Some(query) = NvmlLibQuery::new(self) {
                    query.query(gpus);
                }
            }
        }
    }

    impl Drop for NvmlLib {
        fn drop(&mut self) {
            unsafe {
                libc::dlclose(self.lib_handle);
            }
        }
    }

    struct NvmlLibQuery<'a> {
        inner: &'a NvmlLib,
    }

    impl<'a> NvmlLibQuery<'a> {
        fn new(inner: &'a NvmlLib) -> Option<Self> {
            unsafe {
                if (inner.init)() != NVML_SUCCESS {
                    sysinfo_debug!("Failed to initialize NVML context.");
                    return None;
                }
                Some(Self { inner })
            }
        }

        unsafe fn query(&self, gpus: &mut Vec<Gpu>) {
            unsafe {
                let mut device_count: c_uint = 0;

                if (self.inner.get_count)(&mut device_count) != NVML_SUCCESS {
                    sysinfo_debug!("Failed to retrieve device count.");
                    return;
                }

                for index in 0..device_count {
                    let mut device_handle: NvmlDeviceHandle = null_mut();

                    if (self.inner.get_handle)(index, &mut device_handle) != NVML_SUCCESS {
                        sysinfo_debug!("Failed to get handle for GPU {index}");
                        continue;
                    }

                    let mut pci_info = MaybeUninit::<NvmlPciInfo>::uninit();
                    if (self.inner.get_pci_info)(device_handle, pci_info.as_mut_ptr())
                        != NVML_SUCCESS
                    {
                        sysinfo_debug!("Failed to get PCI for GPU {index}");
                        continue;
                    }
                    let pci_info = pci_info.assume_init();
                    let Some(pci) =
                        convert_to_str(&pci_info.bus_id).and_then(|pci| pci.parse::<PCI>().ok())
                    else {
                        continue;
                    };

                    let gpu = match gpus.iter_mut().find(|gpu| gpu.inner.pci == pci) {
                        Some(gpu) => &mut gpu.inner,
                        None => {
                            let mut gpu = GpuInner::new(pci.to_owned(), "NVIDIA Corporation", None);

                            let mut name_buffer = [0 as c_char; NVML_DEVICE_NAME_V2_BUFFER_SIZE];
                            if (self.inner.get_name)(
                                device_handle,
                                name_buffer.as_mut_ptr(),
                                NVML_DEVICE_NAME_V2_BUFFER_SIZE as c_uint,
                            ) == NVML_SUCCESS
                            {
                                gpu.model = convert_to_string(&name_buffer);
                            }

                            gpus.push(Gpu { inner: gpu });
                            &mut gpus.last_mut().unwrap().inner
                        }
                    };

                    let mut utilization = NvmlUtilization { gpu: 0, memory: 0 };
                    if (self.inner.get_utilization)(device_handle, &mut utilization) == NVML_SUCCESS
                    {
                        gpu.usage = Some(utilization.gpu as f32);
                    }

                    let mut mem_info = NvmlMemory {
                        total: 0,
                        free: 0,
                        used: 0,
                    };
                    if (self.inner.get_memory_info)(device_handle, &mut mem_info) == NVML_SUCCESS {
                        gpu.total_memory = Some(mem_info.total);
                        gpu.used_memory = Some(mem_info.used);
                    }
                }
            }
        }
    }

    impl<'a> Drop for NvmlLibQuery<'a> {
        fn drop(&mut self) {
            unsafe {
                (self.inner.shutdown)();
            }
        }
    }
}

mod vulkan {
    use super::*;
    use std::ffi::{CStr, c_char, c_void};
    use std::mem::{MaybeUninit, transmute};
    use std::ptr::{null, null_mut};

    type VkFlags = u32;
    type VkResult = i32;
    type VkDeviceSize = u64;
    type VkSampleCountFlags = VkFlags;

    const VK_SUCCESS: VkResult = 0;
    const VK_MAX_PHYSICAL_DEVICE_NAME_SIZE: usize = 256;
    const VK_UUID_SIZE: usize = 16;
    const VK_MAX_MEMORY_HEAPS: usize = 16;
    const VK_MAX_MEMORY_TYPES: usize = 32;

    const VK_MEMORY_HEAP_DEVICE_LOCAL_BIT: u32 = 0x00000001;

    const VK_STRUCTURE_TYPE_APPLICATION_INFO: u32 = 0;

    const VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2: i32 = 1000059001;
    const VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_MEMORY_PROPERTIES_2: i32 = 1000059006;
    const VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PCI_BUS_INFO_PROPERTIES_EXT: i32 = 1000212000;
    const VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_MEMORY_BUDGET_PROPERTIES_EXT: i32 = 1000237000;

    const VK_KHR_GET_PHYSICAL_DEVICE_PROPERTIES_2_EXTENSION_NAME: &CStr =
        c"VK_KHR_get_physical_device_properties2";
    const VK_EXT_MEMORY_BUDGET_EXTENSION_NAME: &CStr = c"VK_EXT_memory_budget";

    type VkInstance = *mut c_void;
    type VkPhysicalDevice = *mut c_void;

    #[repr(C)]
    struct VkApplicationInfo {
        s_type: u32,
        p_next: *const c_void,
        p_application_name: *const c_char,
        application_version: u32,
        p_engine_name: *const c_char,
        engine_version: u32,
        api_version: u32,
    }

    #[repr(C)]
    struct VkInstanceCreateInfo {
        s_type: u32,
        p_next: *const c_void,
        flags: VkFlags,
        p_application_info: *const VkApplicationInfo,
        enabled_layer_count: u32,
        pp_enabled_layer_names: *const *const c_char,
        enabled_extension_count: u32,
        pp_enabled_extension_names: *const *const c_char,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct VkPhysicalDeviceLimits {
        maxImageDimension1D: u32,
        maxImageDimension2D: u32,
        maxImageDimension3D: u32,
        maxImageDimensionCube: u32,
        maxImageArrayLayers: u32,
        maxTexelBufferElements: u32,
        maxUniformBufferRange: u32,
        maxStorageBufferRange: u32,
        maxPushConstantsSize: u32,
        maxMemoryAllocationCount: u32,
        maxSamplerAllocationCount: u32,
        bufferImageGranularity: VkDeviceSize,
        sparseAddressSpaceSize: VkDeviceSize,
        maxBoundDescriptorSets: u32,
        maxPerStageDescriptorSamplers: u32,
        maxPerStageDescriptorUniformBuffers: u32,
        maxPerStageDescriptorStorageBuffers: u32,
        maxPerStageDescriptorSampledImages: u32,
        maxPerStageDescriptorStorageImages: u32,
        maxPerStageDescriptorInputAttachments: u32,
        maxPerStageResources: u32,
        maxDescriptorSetSamplers: u32,
        maxDescriptorSetUniformBuffers: u32,
        maxDescriptorSetUniformBuffersDynamic: u32,
        maxDescriptorSetStorageBuffers: u32,
        maxDescriptorSetStorageBuffersDynamic: u32,
        maxDescriptorSetSampledImages: u32,
        maxDescriptorSetStorageImages: u32,
        maxDescriptorSetInputAttachments: u32,
        maxVertexInputAttributes: u32,
        maxVertexInputBindings: u32,
        maxVertexInputAttributeOffset: u32,
        maxVertexInputBindingStride: u32,
        maxVertexOutputComponents: u32,
        maxTessellationGenerationLevel: u32,
        maxTessellationPatchSize: u32,
        maxTessellationControlPerVertexInputComponents: u32,
        maxTessellationControlPerVertexOutputComponents: u32,
        maxTessellationControlPerPatchOutputComponents: u32,
        maxTessellationControlTotalOutputComponents: u32,
        maxTessellationEvaluationInputComponents: u32,
        maxTessellationEvaluationOutputComponents: u32,
        maxGeometryShaderInvocations: u32,
        maxGeometryInputComponents: u32,
        maxGeometryOutputComponents: u32,
        maxGeometryOutputVertices: u32,
        maxGeometryTotalOutputComponents: u32,
        maxFragmentInputComponents: u32,
        maxFragmentOutputAttachments: u32,
        maxFragmentDualSrcAttachments: u32,
        maxFragmentCombinedOutputResources: u32,
        maxComputeSharedMemorySize: u32,
        maxComputeWorkGroupCount: [u32; 3],
        maxComputeWorkGroupInvocations: u32,
        maxComputeWorkGroupSize: [u32; 3],
        subPixelPrecisionBits: u32,
        subTexelPrecisionBits: u32,
        mipmapPrecisionBits: u32,
        maxDrawIndexedIndexValue: u32,
        maxDrawIndirectCount: u32,
        maxSamplerLodBias: f32,
        maxSamplerAnisotropy: f32,
        maxViewports: u32,
        maxViewportDimensions: [u32; 2],
        viewportBoundsRange: [f32; 2],
        viewportSubPixelBits: u32,
        minMemoryMapAlignment: usize,
        minTexelBufferOffsetAlignment: VkDeviceSize,
        minUniformBufferOffsetAlignment: VkDeviceSize,
        minStorageBufferOffsetAlignment: VkDeviceSize,
        minTexelOffset: i32,
        maxTexelOffset: u32,
        minTexelGatherOffset: i32,
        maxTexelGatherOffset: u32,
        minInterpolationOffset: f32,
        maxInterpolationOffset: f32,
        subPixelInterpolationOffsetBits: u32,
        maxFramebufferWidth: u32,
        maxFramebufferHeight: u32,
        maxFramebufferLayers: u32,
        framebufferColorSampleCounts: VkSampleCountFlags,
        framebufferDepthSampleCounts: VkSampleCountFlags,
        framebufferStencilSampleCounts: VkSampleCountFlags,
        framebufferNoAttachmentsSampleCounts: VkSampleCountFlags,
        maxColorAttachments: u32,
        sampledImageColorSampleCounts: VkSampleCountFlags,
        sampledImageIntegerSampleCounts: VkSampleCountFlags,
        sampledImageDepthSampleCounts: VkSampleCountFlags,
        sampledImageStencilSampleCounts: VkSampleCountFlags,
        storageImageSampleCounts: VkSampleCountFlags,
        maxSampleMaskWords: u32,
        timestampComputeAndGraphics: u32,
        timestampPeriod: f32,
        maxClipDistances: u32,
        maxCullDistances: u32,
        maxCombinedClipAndCullDistances: u32,
        discreteQueuePriorities: u32,
        pointSizeRange: [f32; 2],
        lineWidthRange: [f32; 2],
        pointSizeGranularity: f32,
        lineWidthGranularity: f32,
        strictLines: u32,
        standardSampleLocations: u32,
        optimalBufferCopyOffsetAlignment: VkDeviceSize,
        optimalBufferCopyRowPitchAlignment: VkDeviceSize,
        nonCoherentAtomSize: VkDeviceSize,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct VkPhysicalDeviceSparseProperties {
        residency_standard_2D_block_shape: u32,
        residency_standard_2D_multisample_block_shape: u32,
        residency_standard_3D_block_shape: u32,
        residency_aligned_mip_size: u32,
        residency_non_resident_strict: u32,
    }

    #[repr(C)]
    struct VkPhysicalDeviceProperties {
        api_version: u32,
        driver_version: u32,
        vendor_id: u32,
        device_id: u32,
        device_type: u32,
        device_name: [c_char; VK_MAX_PHYSICAL_DEVICE_NAME_SIZE],
        pipeline_cache_uuid: [u8; VK_UUID_SIZE],
        limits: VkPhysicalDeviceLimits,
        sparse_properties: VkPhysicalDeviceSparseProperties,
    }

    #[repr(C)]
    struct VkPhysicalDeviceProperties2 {
        s_type: i32,
        p_next: *mut c_void,
        properties: VkPhysicalDeviceProperties,
    }

    #[repr(C)]
    struct VkPhysicalDevicePCIBusInfoPropertiesEXT {
        s_type: i32,
        p_next: *mut c_void,
        pci_domain: u32,
        pci_bus: u32,
        pci_device: u32,
        pci_function: u32,
    }

    #[repr(C)]
    struct VkMemoryHeap {
        size: VkDeviceSize,
        flags: VkFlags,
    }

    #[repr(C)]
    struct VkMemoryType {
        property_flags: VkFlags,
        heap_index: u32,
    }

    #[repr(C)]
    struct VkPhysicalDeviceMemoryProperties {
        memory_type_count: u32,
        memory_types: [VkMemoryType; VK_MAX_MEMORY_TYPES],
        memory_heap_count: u32,
        memory_heaps: [VkMemoryHeap; VK_MAX_MEMORY_HEAPS],
    }

    #[repr(C)]
    struct VkPhysicalDeviceMemoryBudgetPropertiesEXT {
        s_type: i32,
        p_next: *mut c_void,
        heap_budget: [VkDeviceSize; VK_MAX_MEMORY_HEAPS],
        heap_usage: [VkDeviceSize; VK_MAX_MEMORY_HEAPS],
    }

    #[repr(C)]
    struct VkPhysicalDeviceMemoryProperties2 {
        s_type: i32,
        p_next: *mut c_void,
        memory_properties: VkPhysicalDeviceMemoryProperties,
    }

    type PfnVkCreateInstance = unsafe extern "system" fn(
        *const VkInstanceCreateInfo,
        *const c_void,
        *mut VkInstance,
    ) -> VkResult;
    type PfnVkDestroyInstance = unsafe extern "system" fn(VkInstance, *const c_void);
    type PfnVkEnumeratePhysicalDevices =
        unsafe extern "system" fn(VkInstance, *mut u32, *mut VkPhysicalDevice) -> VkResult;
    type PfnVkGetPhysicalDeviceMemoryProperties =
        unsafe extern "system" fn(VkPhysicalDevice, *mut VkPhysicalDeviceMemoryProperties);
    type PfnVkGetInstanceProcAddr =
        unsafe extern "system" fn(VkInstance, *const c_char) -> *mut c_void;

    // Extension to query PCI.
    type PfnVkGetPhysicalDeviceProperties2KHR =
        unsafe extern "system" fn(VkPhysicalDevice, *mut VkPhysicalDeviceProperties2);
    // Extension to query used VRAM.
    type PfnVkGetPhysicalDeviceMemoryProperties2KHR =
        unsafe extern "system" fn(VkPhysicalDevice, *mut VkPhysicalDeviceMemoryProperties2);

    unsafe fn try_create_instance(
        create_instance: PfnVkCreateInstance,
        extensions: &[*const c_char],
        app_info: &VkApplicationInfo,
    ) -> Option<VkInstance> {
        let mut instance: VkInstance = null_mut();
        let create_info = VkInstanceCreateInfo {
            s_type: 1,
            p_next: null(),
            flags: 0,
            p_application_info: app_info,
            enabled_layer_count: 0,
            pp_enabled_layer_names: null(),
            enabled_extension_count: extensions.len() as _,
            pp_enabled_extension_names: extensions.as_ptr(),
        };
        unsafe {
            let res = create_instance(&create_info, null(), &mut instance);
            if res != VK_SUCCESS {
                sysinfo_debug!("vkCreateInstance failed: {res}");
                None
            } else {
                Some(instance)
            }
        }
    }

    pub(crate) struct Vulkan {
        lib_handle: *mut c_void,
        instance: VkInstance,
        destroy_instance: PfnVkDestroyInstance,
        enumerate_physical_devices: PfnVkEnumeratePhysicalDevices,
        get_physical_device_memory_properties: PfnVkGetPhysicalDeviceMemoryProperties,
        get_physical_device_properties_2_khr: PfnVkGetPhysicalDeviceProperties2KHR,
        get_physical_device_memory_properties_2: Option<PfnVkGetPhysicalDeviceMemoryProperties2KHR>,
    }

    impl Vulkan {
        pub(crate) fn load() -> Option<Self> {
            unsafe {
                let lib_name = c"libvulkan.so.1";
                let lib_handle = libc::dlopen(lib_name.as_ptr(), libc::RTLD_NOW);
                if lib_handle.is_null() {
                    sysinfo_debug!("Failed to load {lib_name:?}");
                    return None;
                }

                match Self::load_symbols(lib_handle) {
                    Some(ret) => Some(ret),
                    None => {
                        libc::dlclose(lib_handle);
                        None
                    }
                }
            }
        }

        unsafe fn load_symbols(lib_handle: *mut c_void) -> Option<Self> {
            unsafe {
                let create_instance =
                    load_sym!(lib_handle, c"vkCreateInstance", PfnVkCreateInstance)?;
                let destroy_instance =
                    load_sym!(lib_handle, c"vkDestroyInstance", PfnVkDestroyInstance)?;
                let enumerate_physical_devices = load_sym!(
                    lib_handle,
                    c"vkEnumeratePhysicalDevices",
                    PfnVkEnumeratePhysicalDevices,
                )?;
                let get_physical_device_memory_properties = load_sym!(
                    lib_handle,
                    c"vkGetPhysicalDeviceMemoryProperties",
                    PfnVkGetPhysicalDeviceMemoryProperties,
                )?;
                let get_instance_proc_addr = load_sym!(
                    lib_handle,
                    c"vkGetInstanceProcAddr",
                    PfnVkGetInstanceProcAddr
                )?;

                let app_info = VkApplicationInfo {
                    s_type: VK_STRUCTURE_TYPE_APPLICATION_INFO,
                    p_next: null(),
                    p_application_name: null(),
                    application_version: 0,
                    p_engine_name: null(),
                    engine_version: 0,
                    api_version: (1 << 22) | (1 << 12), // Vulkan 1.1
                };

                // We need to enable this extension to query the `PCI` and "used memory"
                // information.
                let mut extensions = vec![
                    VK_KHR_GET_PHYSICAL_DEVICE_PROPERTIES_2_EXTENSION_NAME.as_ptr(),
                    VK_EXT_MEMORY_BUDGET_EXTENSION_NAME.as_ptr(),
                ];

                let instance = try_create_instance(create_instance, &extensions, &app_info)
                    .or_else(|| {
                        // We retry without the "used memory" extension.
                        extensions.pop();
                        try_create_instance(create_instance, &extensions, &app_info)
                    })?;

                // We try to get the extension function address...
                let ext_fn_name = c"vkGetPhysicalDeviceProperties2KHR";
                let get_physical_device_properties_2_khr_ptr =
                    get_instance_proc_addr(instance, ext_fn_name.as_ptr());

                if get_physical_device_properties_2_khr_ptr.is_null() {
                    sysinfo_debug!("Extension function {ext_fn_name:?} not available");
                    destroy_instance(instance, null());
                    return None;
                }

                let ext_fn_name = c"vkGetPhysicalDeviceMemoryProperties2KHR";
                let get_physical_device_memory_properties_2 =
                    get_instance_proc_addr(instance, ext_fn_name.as_ptr());
                let get_physical_device_memory_properties_2 =
                    if get_physical_device_memory_properties_2.is_null() {
                        None
                    } else {
                        Some(transmute::<
                            *mut c_void,
                            PfnVkGetPhysicalDeviceMemoryProperties2KHR,
                        >(
                            get_physical_device_memory_properties_2
                        ))
                    };

                Some(Self {
                    lib_handle,
                    instance,
                    destroy_instance,
                    enumerate_physical_devices,
                    get_physical_device_memory_properties,
                    get_physical_device_properties_2_khr: transmute::<
                        *mut c_void,
                        PfnVkGetPhysicalDeviceProperties2KHR,
                    >(
                        get_physical_device_properties_2_khr_ptr,
                    ),
                    get_physical_device_memory_properties_2,
                })
            }
        }

        pub(crate) fn get_vulkan_memory(&self, gpus: &mut [Gpu]) {
            unsafe {
                let mut device_count: u32 = 0;
                (self.enumerate_physical_devices)(self.instance, &mut device_count, null_mut());

                if device_count == 0 {
                    return;
                }
                let mut devices = vec![null_mut(); device_count as usize];
                (self.enumerate_physical_devices)(
                    self.instance,
                    &mut device_count,
                    devices.as_mut_ptr(),
                );

                for device in devices {
                    let mut pci_properties = VkPhysicalDevicePCIBusInfoPropertiesEXT {
                        s_type: VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PCI_BUS_INFO_PROPERTIES_EXT,
                        p_next: null_mut(),
                        pci_domain: 0,
                        pci_bus: 0,
                        pci_device: 0,
                        pci_function: 0,
                    };

                    let mut props2 = VkPhysicalDeviceProperties2 {
                        s_type: VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2,
                        p_next: &mut pci_properties as *mut _ as *mut c_void,
                        properties: std::mem::zeroed(),
                    };

                    (self.get_physical_device_properties_2_khr)(device, &mut props2);

                    // Format the extracted PCI string explicitly to match `NVML`/`sys/drm`
                    // format.
                    let pci = PCI {
                        domain: pci_properties.pci_domain,
                        bus: pci_properties.pci_bus,
                        device: pci_properties.pci_device,
                        function: pci_properties.pci_function,
                    };

                    // We don't want to discover new GPUs, just to add extra info to existing
                    // ones.
                    let Some(gpu) = gpus.iter_mut().find(|gpu| gpu.inner.pci == pci) else {
                        continue;
                    };
                    if gpu.inner.total_memory.is_some() && gpu.inner.used_memory.is_some() {
                        // All information is already here, ignoring it.
                        continue;
                    }

                    if gpu.inner.model.is_none() {
                        gpu.inner.model = convert_to_string(&props2.properties.device_name);
                    }

                    if let Some(get_physical_device_memory_properties_2) =
                        self.get_physical_device_memory_properties_2
                    {
                        let mut budget_properties = VkPhysicalDeviceMemoryBudgetPropertiesEXT {
                            s_type: VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_MEMORY_BUDGET_PROPERTIES_EXT,
                            p_next: null_mut(),
                            heap_budget: [0; VK_MAX_MEMORY_HEAPS],
                            heap_usage: [0; VK_MAX_MEMORY_HEAPS],
                        };
                        let mut mem_props2 = VkPhysicalDeviceMemoryProperties2 {
                            s_type: VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_MEMORY_PROPERTIES_2,
                            p_next: &mut budget_properties as *mut _ as *mut c_void,
                            memory_properties: std::mem::zeroed(),
                        };
                        get_physical_device_memory_properties_2(device, &mut mem_props2);

                        let heap_count = mem_props2.memory_properties.memory_heap_count as usize;

                        let mut total_memory: u64 = 0;
                        let mut used_memory: u64 = 0;
                        let mut found_vram = false;
                        for h in 0..heap_count {
                            let heap = &mem_props2.memory_properties.memory_heaps[h];
                            let is_vram = (heap.flags & VK_MEMORY_HEAP_DEVICE_LOCAL_BIT) != 0;
                            // Fun thing to know, embedded GPUs don't always have their own memory and
                            // instead use the system's. To make things simpler, we voluntarily ignore
                            // all memory that isn't VRAM.
                            if !is_vram {
                                continue;
                            }

                            found_vram = true;
                            total_memory = total_memory.saturating_add(heap.size);
                            used_memory =
                                used_memory.saturating_add(budget_properties.heap_usage[h]);
                        }
                        if found_vram {
                            gpu.inner.total_memory = Some(total_memory);
                            gpu.inner.used_memory = Some(used_memory);
                        } else {
                            gpu.inner.total_memory = None;
                            gpu.inner.used_memory = None;
                        }
                    } else {
                        // When we don't have access to used memory.
                        let mut mem_props =
                            MaybeUninit::<VkPhysicalDeviceMemoryProperties>::uninit();
                        (self.get_physical_device_memory_properties)(
                            device,
                            mem_props.as_mut_ptr(),
                        );
                        let mem_props = mem_props.assume_init();

                        let mut total_memory: u64 = 0;
                        let mut found_total = false;
                        for h in 0..(mem_props.memory_heap_count as usize) {
                            let heap = &mem_props.memory_heaps[h];
                            let is_vram = (heap.flags & VK_MEMORY_HEAP_DEVICE_LOCAL_BIT) != 0;
                            // Fun thing to know, embedded GPUs don't always have their own memory and
                            // instead use the system's. To make things simpler, we voluntarily ignore
                            // all memory that isn't VRAM.
                            if !is_vram {
                                continue;
                            }
                            found_total = true;
                            total_memory =
                                total_memory.saturating_add(mem_props.memory_heaps[h].size);
                        }
                        if found_total {
                            gpu.inner.total_memory = Some(total_memory);
                        } else {
                            gpu.inner.total_memory = None;
                        }
                    }
                }
            }
        }
    }

    impl Drop for Vulkan {
        fn drop(&mut self) {
            unsafe {
                // Cleaning up the vulkan instance and the loaded lib handle.
                (self.destroy_instance)(self.instance, null());
                libc::dlclose(self.lib_handle);
            }
        }
    }
}
