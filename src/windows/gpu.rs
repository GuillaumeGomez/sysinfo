// Take a look at the license at the top of the repository in the LICENSE file.

use std::collections::HashMap;
use std::mem::{MaybeUninit, zeroed};

use windows::Wdk::Graphics::Direct3D::{
    D3DKMT_ADAPTERADDRESS, D3DKMT_CLOSEADAPTER, D3DKMT_OPENADAPTERFROMLUID,
    D3DKMT_QUERYADAPTERINFO, D3DKMTCloseAdapter, D3DKMTOpenAdapterFromLuid, D3DKMTQueryAdapterInfo,
    KMTQAITYPE_ADAPTERADDRESS,
};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_PRESENT, GUID_DEVCLASS_DISPLAY, HDEVINFO, SETUP_DI_REGISTRY_PROPERTY, SP_DEVINFO_DATA,
    SPDRP_ADDRESS, SPDRP_BUSNUMBER, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
    SetupDiGetClassDevsW, SetupDiGetDeviceRegistryPropertyW,
};
use windows::Win32::Foundation::{ERROR_SUCCESS, LUID};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, DXGI_QUERY_VIDEO_MEMORY_INFO,
    IDXGIAdapter3, IDXGIFactory6,
};
use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PdhAddEnglishCounterW,
    PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    PdhRemoveCounter,
};
use windows::core::{Interface, PCWSTR};

use crate::{Gpu, PCI};

pub(crate) struct GpuInner {
    total_memory: Option<u64>,
    used_memory: Option<u64>,
    usage: Option<f32>,
    model: Option<String>,
    vendor: Option<String>,
    pci: PCI,
    pub(crate) updated: bool,
    luid: LUID,
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
        self.total_memory
    }
    pub(crate) fn used_memory(&self) -> Option<u64> {
        self.used_memory
    }
}

pub(crate) struct GpusInner {
    pub(crate) gpus: Vec<Gpu>,
    query: Option<PDH_HQUERY>,
    gpu_query: Option<PDH_HCOUNTER>,
}

impl Drop for GpusInner {
    fn drop(&mut self) {
        if let Some(gpu_query) = self.gpu_query
            && !gpu_query.is_invalid()
        {
            unsafe {
                PdhRemoveCounter(gpu_query);
            }
        }
        if let Some(query) = self.query
            && !query.is_invalid()
        {
            unsafe {
                PdhCloseQuery(query);
            }
        }
    }
}

impl GpusInner {
    pub(crate) fn new() -> Result<Self, crate::Error> {
        unsafe {
            let mut ret = GpusInner {
                gpus: Vec::new(),
                query: None,
                gpu_query: None,
            };
            let mut query = PDH_HQUERY::default();
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) == ERROR_SUCCESS.0 {
                ret.gpu_query =
                    get_english_counter(query, r"\GPU Engine(*)\Utilization Percentage");
                ret.query = Some(query);
            }
            Ok(ret)
        }
    }

    pub(crate) fn refresh(&mut self) {
        unsafe {
            let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory6>() else {
                return;
            };

            let mut pcis = None;
            let mut index = 0;
            while let Ok(adapter) = factory.EnumAdapters1(index) {
                index += 1;

                let Ok(desc) = adapter.GetDesc1() else {
                    continue;
                };
                let gpu = match self
                    .gpus
                    .iter_mut()
                    .find(|gpu| gpu.inner.luid == desc.AdapterLuid)
                {
                    Some(gpu) => {
                        gpu.inner.updated = true;
                        &mut gpu.inner
                    }
                    None => {
                        if pcis.is_none() {
                            pcis = get_all_pcis();
                        }
                        if let Some(ref pcis) = pcis
                            && let Some(addr) = LUIDAdapter::new(desc.AdapterLuid)
                                .and_then(|adapter| adapter.query())
                            // Why not using the `addr` info instead of iterating through PCIs?
                            // Because it might return a "fake" GPU, and PCIs allow us to filter.
                            && let Some(pci) = pcis.iter().find(|pci| {
                                pci.bus == addr.BusNumber
                                    && pci.device == addr.DeviceNumber
                                    && pci.function == addr.FunctionNumber
                            })
                        {
                            let gpu = GpuInner {
                                pci: pci.clone(),
                                vendor: crate::utils::gpu_vendor_name(desc.VendorId)
                                    .map(|v| v.to_owned()),
                                model: Some(utf16_to_string(&desc.Description)),
                                total_memory: None,
                                used_memory: None,
                                usage: None,
                                updated: true,
                                luid: desc.AdapterLuid,
                            };
                            self.gpus.push(Gpu { inner: gpu });
                            &mut self.gpus.last_mut().unwrap().inner
                        } else {
                            continue;
                        }
                    }
                };

                gpu.total_memory = Some(desc.DedicatedVideoMemory as u64);

                if let Ok(adapter3) = adapter.cast::<IDXGIAdapter3>() {
                    let mut mem = MaybeUninit::<DXGI_QUERY_VIDEO_MEMORY_INFO>::uninit();
                    if adapter3
                        .QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, mem.as_mut_ptr())
                        .is_ok()
                    {
                        let mem = mem.assume_init();
                        gpu.used_memory = Some(mem.CurrentUsage);
                    }
                }
            }

            // We now query the % usage of each GPU (if any).
            if let Some(query) = self.query
                && let Some(gpu_query) = self.gpu_query
                && self.gpus.iter().any(|gpu| gpu.inner.updated)
            {
                let util = query_gpu_utilization(query, gpu_query);
                for gpu in self.gpus.iter_mut().filter(|gpu| gpu.inner.updated) {
                    gpu.inner.usage = util
                        .get(&(gpu.inner.luid.HighPart, gpu.inner.luid.LowPart))
                        .copied();
                }
            }
        }
    }
}

unsafe fn get_english_counter(query: PDH_HQUERY, name: &str) -> Option<PDH_HCOUNTER> {
    let key = name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();

    let mut counter = PDH_HCOUNTER::default();
    unsafe {
        let ret = PdhAddEnglishCounterW(query, PCWSTR::from_raw(key.as_ptr()), 0, &mut counter);
        if ret == ERROR_SUCCESS.0 && !counter.is_invalid() {
            Some(counter)
        } else {
            sysinfo_debug!("`PdhAddEnglishCounterW` failed to add counter `{name}`: {ret:x}");
            None
        }
    }
}

struct ClassDevs(HDEVINFO);

impl ClassDevs {
    unsafe fn new() -> Option<Self> {
        unsafe {
            if let Ok(ret) =
                SetupDiGetClassDevsW(Some(&GUID_DEVCLASS_DISPLAY), None, None, DIGCF_PRESENT)
                && !ret.is_invalid()
            {
                Some(Self(ret))
            } else {
                None
            }
        }
    }
}

impl Drop for ClassDevs {
    fn drop(&mut self) {
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}

fn utf16_to_string(buf: &[u16]) -> String {
    unsafe {
        let buf = std::slice::from_raw_parts(buf.as_ptr(), buf.len());
        let len = buf.iter().position(|x| *x == 0).unwrap_or(buf.len());

        String::from_utf16_lossy(&buf[..len])
    }
}

unsafe fn get_u32_property(
    h_dev_info: HDEVINFO,
    dev_info_data: &SP_DEVINFO_DATA,
    key: SETUP_DI_REGISTRY_PROPERTY,
) -> Option<u32> {
    let mut required = std::mem::size_of::<u32>() as u32;
    let mut value = [0u8; std::mem::size_of::<u32>()];

    unsafe {
        if SetupDiGetDeviceRegistryPropertyW(
            h_dev_info,
            dev_info_data,
            key,
            None,
            Some(&mut value),
            Some(&mut required),
        )
        .is_ok()
        {
            Some(u32::from_ne_bytes(value))
        } else {
            None
        }
    }
}

unsafe fn get_all_pcis() -> Option<Vec<PCI>> {
    unsafe {
        let dev_info_set = ClassDevs::new()?;

        let mut dev_info_data: SP_DEVINFO_DATA = zeroed();
        dev_info_data.cbSize = size_of::<SP_DEVINFO_DATA>() as _;
        let mut index = 0;
        let mut pcis = Vec::new();

        while SetupDiEnumDeviceInfo(dev_info_set.0, index, &mut dev_info_data).is_ok() {
            index += 1;

            let bus =
                get_u32_property(dev_info_set.0, &dev_info_data, SPDRP_BUSNUMBER).unwrap_or(0);
            let address =
                get_u32_property(dev_info_set.0, &dev_info_data, SPDRP_ADDRESS).unwrap_or(0);
            // High 16 bits = Device number, Low 16 bits = Function number
            let device = (address >> 16) & 0xFFFF;
            let function = address & 0xFFFF;

            pcis.push(PCI {
                domain: 0,
                bus,
                device,
                function,
            });
        }

        Some(pcis)
    }
}

unsafe fn query_gpu_utilization(
    query: PDH_HQUERY,
    gpu_query: PDH_HCOUNTER,
) -> HashMap<(i32, u32), f32> {
    let mut result = HashMap::new();
    let mut buffer_size = 0u32;
    let mut item_count = 0u32;

    unsafe {
        PdhCollectQueryData(query);

        PdhGetFormattedCounterArrayW(
            gpu_query,
            PDH_FMT_DOUBLE,
            &mut buffer_size,
            &mut item_count,
            None,
        );

        let mut buffer = vec![0u8; buffer_size as usize];

        let status = PdhGetFormattedCounterArrayW(
            gpu_query,
            PDH_FMT_DOUBLE,
            &mut buffer_size,
            &mut item_count,
            Some(buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W),
        );

        if status == ERROR_SUCCESS.0 {
            let items = std::slice::from_raw_parts(
                buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
                item_count as usize,
            );

            for item in items {
                let name = PCWSTR(item.szName.0).to_string().unwrap_or_default();

                let value = item.FmtValue.Anonymous.doubleValue as f32;

                if let Some(pos) = name.find("luid_0x") {
                    let s = &name[pos + 7..];

                    let mut parts = s.split('_');

                    let high_hex = parts.next().unwrap_or("0");
                    let low_hex = parts
                        .next()
                        .and_then(|s| s.strip_prefix("0x"))
                        .unwrap_or("0");

                    if let (Ok(high), Ok(low)) = (
                        u32::from_str_radix(high_hex, 16),
                        u32::from_str_radix(low_hex, 16),
                    ) {
                        *result.entry((high as i32, low)).or_insert(0.0) += value;
                    }
                }
            }
        }
    }

    result
}

struct LUIDAdapter(D3DKMT_OPENADAPTERFROMLUID);

impl LUIDAdapter {
    unsafe fn new(luid: LUID) -> Option<Self> {
        let mut adapter = D3DKMT_OPENADAPTERFROMLUID {
            AdapterLuid: luid,
            hAdapter: 0,
        };

        unsafe {
            if D3DKMTOpenAdapterFromLuid(&mut adapter).is_ok() {
                Some(Self(adapter))
            } else {
                None
            }
        }
    }

    unsafe fn query(self) -> Option<D3DKMT_ADAPTERADDRESS> {
        let mut address = D3DKMT_ADAPTERADDRESS::default();

        let mut query = D3DKMT_QUERYADAPTERINFO {
            hAdapter: self.0.hAdapter,
            Type: KMTQAITYPE_ADAPTERADDRESS,
            pPrivateDriverData: &mut address as *mut _ as _,
            PrivateDriverDataSize: size_of::<D3DKMT_ADAPTERADDRESS>() as u32,
        };

        unsafe {
            if D3DKMTQueryAdapterInfo(&mut query).is_ok() {
                Some(address)
            } else {
                None
            }
        }
    }
}

impl Drop for LUIDAdapter {
    fn drop(&mut self) {
        unsafe {
            let _ = D3DKMTCloseAdapter(&D3DKMT_CLOSEADAPTER {
                hAdapter: self.0.hAdapter,
            });
        }
    }
}
