// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{Cpu, CpuRefreshKind, Error as SError, LoadAvg};

use std::collections::HashMap;
use std::io::Error;
use std::mem;

use windows::Win32::Foundation::{CloseHandle, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, HANDLE};
use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PERF_DETAIL_NOVICE,
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhEnumObjectsW,
    PdhGetFormattedCounterValue, PdhOpenQueryW, PdhRemoveCounter,
};
use windows::Win32::System::Power::{
    CallNtPowerInformation, PROCESSOR_POWER_INFORMATION, ProcessorInformation,
};
use windows::Win32::System::SystemInformation::{
    self, GetLogicalProcessorInformationEx, GetSystemInfo, RelationAll, RelationProcessorCore,
    SYSTEM_INFO, SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
};
use windows::core::PCWSTR;

pub(crate) fn get_load_average() -> Result<LoadAvg, crate::Error> {
    Err(crate::Error::Unsupported)
}

struct Counter {
    /// Number of users for this counter.
    count: usize,
    /// Counter key.
    key: PDH_HCOUNTER,
}

unsafe impl Send for Counter {}
unsafe impl Sync for Counter {}

impl Drop for Counter {
    fn drop(&mut self) {
        unsafe {
            PdhRemoveCounter(self.key);
        }
    }
}

struct InternalQuery {
    query: PDH_HQUERY,
    event: HANDLE,
    /// The key is the "counter query", the value is the `(number of users, counter)`.
    data: HashMap<String, Counter>,
}

unsafe impl Send for InternalQuery {}
unsafe impl Sync for InternalQuery {}

impl Drop for InternalQuery {
    fn drop(&mut self) {
        unsafe {
            // We remove all counters because closing the rest.
            self.data.clear();

            if !self.event.is_invalid() {
                let _err = CloseHandle(self.event);
            }

            if !self.query.is_invalid() {
                PdhCloseQuery(self.query);
            }
        }
    }
}

pub(crate) struct Query {
    internal: InternalQuery,
}

impl Query {
    pub fn new(force_reload: bool) -> Option<Query> {
        let mut query = PDH_HQUERY::default();
        unsafe {
            if force_reload {
                PdhEnumObjectsW(
                    PCWSTR::null(),
                    PCWSTR::null(),
                    None,
                    &mut 0,
                    PERF_DETAIL_NOVICE,
                    true,
                );
            }
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) == ERROR_SUCCESS.0 {
                let q = InternalQuery {
                    query,
                    event: HANDLE::default(),
                    data: HashMap::new(),
                };
                Some(Query { internal: q })
            } else {
                sysinfo_debug!("Query::new: PdhOpenQueryW failed");
                None
            }
        }
    }

    #[allow(clippy::ptr_arg)]
    pub fn get(&self, name: &str) -> Option<f32> {
        if let Some(counter) = self.internal.data.get(name) {
            unsafe {
                let mut display_value = mem::MaybeUninit::<PDH_FMT_COUNTERVALUE>::uninit();

                return if PdhGetFormattedCounterValue(
                    counter.key,
                    PDH_FMT_DOUBLE,
                    None,
                    display_value.as_mut_ptr(),
                ) == ERROR_SUCCESS.0
                {
                    let display_value = display_value.assume_init();
                    Some(display_value.Anonymous.doubleValue as f32)
                } else {
                    sysinfo_debug!("Query::get: PdhGetFormattedCounterValue failed");
                    Some(0.)
                };
            }
        }
        None
    }

    #[cfg(feature = "gpu")]
    pub fn get_array(&self, name: &str) -> Option<Vec<f32>> {
        use windows::Win32::System::Performance::{
            PDH_FMT_COUNTERVALUE_ITEM_W, PdhGetFormattedCounterArrayW,
        };

        let counter = self.internal.data.get(name)?;

        let mut buffer_size = 0u32;
        let mut item_count = 0u32;

        unsafe {
            PdhGetFormattedCounterArrayW(
                counter.key,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                None,
            );

            if buffer_size == 0 || item_count == 0 {
                return None;
            }

            let mut buffer = vec![0u8; buffer_size as usize];

            let status = PdhGetFormattedCounterArrayW(
                counter.key,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                Some(buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W),
            );

            if status != ERROR_SUCCESS.0 {
                return None;
            }
            let items = std::slice::from_raw_parts(
                buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
                item_count as usize,
            );
            let mut output = Vec::with_capacity(items.len());
            for item in items {
                output.push(item.FmtValue.Anonymous.doubleValue as f32)
            }
            Some(output)
        }
    }

    #[allow(clippy::ptr_arg)]
    pub fn add_english_counter(&mut self, name: &str) -> bool {
        if let Some(counter) = self.internal.data.get_mut(name) {
            counter.count += 1;
            return true;
        }
        unsafe {
            let mut key = PDH_HCOUNTER::default();
            let counter_key = name
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect::<Vec<_>>();
            let ret = PdhAddEnglishCounterW(
                self.internal.query,
                PCWSTR::from_raw(counter_key.as_ptr()),
                0,
                &mut key,
            );
            if ret == ERROR_SUCCESS.0 {
                self.internal
                    .data
                    .insert(name.to_owned(), Counter { count: 1, key });
            } else {
                sysinfo_debug!(
                    "Query::add_english_counter: failed to add counter {name:?}: {ret:x}...",
                );
                return false;
            }
        }
        true
    }

    #[cfg(feature = "gpu")]
    pub fn remove_counter(&mut self, name: &str) {
        let mut need_removal = false;
        if let Some(counter) = self.internal.data.get_mut(name) {
            counter.count -= 1;
            need_removal = counter.count < 1;
        }
        if need_removal {
            self.internal.data.remove(name);
        }
    }

    pub fn refresh(&self) {
        unsafe {
            if PdhCollectQueryData(self.internal.query) != ERROR_SUCCESS.0 {
                sysinfo_debug!("failed to refresh CPU data");
            }
        }
    }
}

pub(crate) struct CpusWrapper {
    pub(crate) global: CpuUsage,
    cpus: Vec<Cpu>,
}

impl CpusWrapper {
    pub fn new() -> Self {
        Self {
            global: CpuUsage {
                percent: 0f32,
                key_used: None,
            },
            cpus: Vec::new(),
        }
    }

    pub fn global_cpu_usage(&self) -> f32 {
        self.global.percent
    }

    pub fn cpus(&self) -> &[Cpu] {
        &self.cpus
    }

    pub(crate) fn init_if_needed(&mut self, refresh_kind: CpuRefreshKind) {
        if self.cpus.is_empty() {
            self.cpus = init_cpus(refresh_kind);
        }
    }

    pub fn len(&mut self) -> usize {
        self.init_if_needed(CpuRefreshKind::nothing());
        self.cpus.len()
    }

    pub fn iter_mut(&mut self, refresh_kind: CpuRefreshKind) -> impl Iterator<Item = &mut Cpu> {
        self.init_if_needed(refresh_kind);
        self.cpus.iter_mut()
    }

    pub fn get_frequencies(&mut self) {
        let frequencies = get_frequencies(self.cpus.len());

        for (cpu, frequency) in self.cpus.iter_mut().zip(frequencies) {
            cpu.inner.set_frequency(frequency);
        }
    }
}

pub(crate) struct CpuUsage {
    percent: f32,
    pub(crate) key_used: Option<KeyHandler>,
}

impl CpuUsage {
    pub(crate) fn set_cpu_usage(&mut self, value: f32) {
        self.percent = value;
    }
}

pub(crate) struct CpuInner {
    name: String,
    vendor_id: String,
    usage: CpuUsage,
    brand: String,
    frequency: u64,
}

impl CpuInner {
    pub(crate) fn cpu_usage(&self) -> f32 {
        self.usage.percent
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn frequency(&self) -> u64 {
        self.frequency
    }

    pub(crate) fn vendor_id(&self) -> &str {
        &self.vendor_id
    }

    pub(crate) fn brand(&self) -> &str {
        &self.brand
    }

    pub(crate) fn new_with_values(
        name: String,
        vendor_id: String,
        brand: String,
        frequency: u64,
    ) -> Self {
        Self {
            name,
            usage: CpuUsage {
                percent: 0f32,
                key_used: None,
            },
            vendor_id,
            brand,
            frequency,
        }
    }

    pub(crate) fn set_cpu_usage(&mut self, value: f32) {
        self.usage.set_cpu_usage(value);
    }

    pub(crate) fn set_frequency(&mut self, value: u64) {
        self.frequency = value;
    }
}

fn get_vendor_id_not_great(info: &SYSTEM_INFO) -> String {
    // https://docs.microsoft.com/fr-fr/windows/win32/api/sysinfoapi/ns-sysinfoapi-system_info
    unsafe {
        match info.Anonymous.Anonymous.wProcessorArchitecture {
            SystemInformation::PROCESSOR_ARCHITECTURE_INTEL => "Intel x86",
            SystemInformation::PROCESSOR_ARCHITECTURE_MIPS => "MIPS",
            SystemInformation::PROCESSOR_ARCHITECTURE_ALPHA => "RISC Alpha",
            SystemInformation::PROCESSOR_ARCHITECTURE_PPC => "PPC",
            SystemInformation::PROCESSOR_ARCHITECTURE_SHX => "SHX",
            SystemInformation::PROCESSOR_ARCHITECTURE_ARM => "ARM",
            SystemInformation::PROCESSOR_ARCHITECTURE_IA64 => "Intel Itanium-based x64",
            SystemInformation::PROCESSOR_ARCHITECTURE_ALPHA64 => "RISC Alpha x64",
            SystemInformation::PROCESSOR_ARCHITECTURE_MSIL => "MSIL",
            SystemInformation::PROCESSOR_ARCHITECTURE_AMD64 => "(Intel or AMD) x64",
            SystemInformation::PROCESSOR_ARCHITECTURE_IA32_ON_WIN64 => "Intel Itanium-based x86",
            SystemInformation::PROCESSOR_ARCHITECTURE_NEUTRAL => "unknown",
            SystemInformation::PROCESSOR_ARCHITECTURE_ARM64 => "ARM x64",
            SystemInformation::PROCESSOR_ARCHITECTURE_ARM32_ON_WIN64 => "ARM",
            SystemInformation::PROCESSOR_ARCHITECTURE_IA32_ON_ARM64 => "Intel Itanium-based x86",
            _ => "unknown",
        }
        .to_owned()
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub(crate) fn get_vendor_id_and_brand(info: &SYSTEM_INFO) -> (String, String) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::__cpuid;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::__cpuid;

    unsafe fn add_u32(v: &mut Vec<u8>, i: u32) {
        let i = &i as *const u32 as *const u8;
        unsafe {
            v.push(*i);
            v.push(*i.offset(1));
            v.push(*i.offset(2));
            v.push(*i.offset(3));
        }
    }

    unsafe {
        // First, we try to get the complete name.
        let res = __cpuid(0x80000000);
        let n_ex_ids = res.eax;
        let brand = if n_ex_ids >= 0x80000004 {
            let mut extdata = Vec::with_capacity(5);

            for i in 0x80000000..=n_ex_ids {
                extdata.push(__cpuid(i));
            }

            // 4 * u32 * nb_entries
            let mut out = Vec::with_capacity(4 * std::mem::size_of::<u32>() * 3);
            for data in extdata.iter().take(5).skip(2) {
                add_u32(&mut out, data.eax);
                add_u32(&mut out, data.ebx);
                add_u32(&mut out, data.ecx);
                add_u32(&mut out, data.edx);
            }
            let mut pos = 0;
            for e in out.iter() {
                if *e == 0 {
                    break;
                }
                pos += 1;
            }
            match std::str::from_utf8(&out[..pos]) {
                Ok(s) => s.to_owned(),
                _ => String::new(),
            }
        } else {
            String::new()
        };

        // Failed to get full name, let's retry for the short version!
        let res = __cpuid(0);
        let mut x = Vec::with_capacity(3 * std::mem::size_of::<u32>());
        add_u32(&mut x, res.ebx);
        add_u32(&mut x, res.edx);
        add_u32(&mut x, res.ecx);
        let mut pos = 0;
        for e in x.iter() {
            if *e == 0 {
                break;
            }
            pos += 1;
        }
        let vendor_id = match std::str::from_utf8(&x[..pos]) {
            Ok(s) => s.to_owned(),
            Err(_) => get_vendor_id_not_great(info),
        };
        (vendor_id, brand)
    }
}

#[cfg(all(not(target_arch = "x86_64"), not(target_arch = "x86")))]
pub(crate) fn get_vendor_id_and_brand(info: &SYSTEM_INFO) -> (String, String) {
    use crate::sys::system::get_reg_string_value;
    use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;

    const HKEY_PROCESSOR: &str = r"HARDWARE\DESCRIPTION\System\CentralProcessor\0";

    let vendor_id = get_reg_string_value(HKEY_LOCAL_MACHINE, HKEY_PROCESSOR, "VendorIdentifier")
        .unwrap_or_else(|| get_vendor_id_not_great(info));
    let brand = get_reg_string_value(HKEY_LOCAL_MACHINE, HKEY_PROCESSOR, "ProcessorNameString")
        .unwrap_or_default();
    (vendor_id, brand)
}

#[inline]
pub(crate) fn get_key_used(p: &mut Cpu) -> &mut Option<KeyHandler> {
    &mut p.inner.usage.key_used
}

// From https://stackoverflow.com/a/43813138:
//
// If your PC has 64 or fewer logical cpus installed, the above code will work fine. However,
// if your PC has more than 64 logical cpus installed, use GetActiveCpuCount() or
// GetLogicalCpuInformation() to determine the total number of logical cpus installed.
pub(crate) fn get_frequencies(nb_cpus: usize) -> Vec<u64> {
    let size = nb_cpus * mem::size_of::<PROCESSOR_POWER_INFORMATION>();
    let mut infos: Vec<PROCESSOR_POWER_INFORMATION> = Vec::with_capacity(nb_cpus);

    unsafe {
        if CallNtPowerInformation(
            ProcessorInformation,
            None,
            0,
            Some(infos.as_mut_ptr() as _),
            size as _,
        )
        .is_ok()
        {
            infos.set_len(nb_cpus);
            // infos.Number
            return infos
                .into_iter()
                .map(|i| i.CurrentMhz as u64)
                .collect::<Vec<_>>();
        }
    }
    sysinfo_debug!("get_frequencies: CallNtPowerInformation failed");
    vec![0; nb_cpus]
}

pub(crate) fn get_physical_core_count() -> Result<usize, SError> {
    // We cannot use the number of cpus here to pre calculate the buf size.
    // `GetLogicalCpuInformationEx` with `RelationProcessorCore` passed to it not only returns
    // the logical cores but also numa nodes.
    //
    // GetLogicalCpuInformationEx: https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex

    let mut needed_size = 0;
    unsafe {
        // This function call will always return an error as it only returns "success" when it
        // has written at least one item in the buffer (which it cannot do here).
        let _err = GetLogicalProcessorInformationEx(RelationAll, None, &mut needed_size);

        let mut buf: Vec<u8> = Vec::with_capacity(needed_size as _);

        loop {
            // Needs to be updated for `Vec::reserve` to actually add additional capacity if
            // `GetLogicalProcessorInformationEx` fails because the buffer isn't big enough.
            buf.set_len(needed_size as _);

            if GetLogicalProcessorInformationEx(
                RelationAll,
                Some(buf.as_mut_ptr().cast()),
                &mut needed_size,
            )
            .is_ok()
            {
                break;
            } else {
                let e = Error::last_os_error();
                // For some reasons, the function might return a size not big enough...
                match e.raw_os_error() {
                    Some(value) if value == ERROR_INSUFFICIENT_BUFFER.0 as i32 => {}
                    _ => {
                        sysinfo_debug!(
                            "get_physical_core_count: GetLogicalCpuInformationEx failed"
                        );
                        return Err(SError::Other(
                            "failed to retrieve physical core count".into(),
                        ));
                    }
                }
            }
            let reserve = if needed_size as usize > buf.capacity() {
                needed_size as usize - buf.capacity()
            } else {
                1
            };
            needed_size = match needed_size.checked_add(reserve as _) {
                Some(new_size) => new_size,
                None => {
                    sysinfo_debug!(
                        "get_physical_core_count: buffer size is too big ({} + {})",
                        needed_size,
                        reserve,
                    );
                    return Err(SError::Other(
                        "failed to retrieve physical core count".into(),
                    ));
                }
            };
            buf.reserve(reserve);
        }

        buf.set_len(needed_size as _);

        let mut i = 0;
        let raw_buf = buf.as_ptr();
        let mut count = 0;
        while i < buf.len() {
            let p = &*(raw_buf.add(i) as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX);
            i += p.Size as usize;
            if p.Relationship == RelationProcessorCore {
                // Only count the physical cores.
                count += 1;
            }
        }
        Ok(count)
    }
}

fn init_cpus(refresh_kind: CpuRefreshKind) -> Vec<Cpu> {
    unsafe {
        let mut sys_info = SYSTEM_INFO::default();
        GetSystemInfo(&mut sys_info);
        let (vendor_id, brand) = get_vendor_id_and_brand(&sys_info);
        let nb_cpus = sys_info.dwNumberOfProcessors as usize;
        let frequencies = if refresh_kind.frequency() {
            get_frequencies(nb_cpus)
        } else {
            vec![0; nb_cpus]
        };
        let mut ret = Vec::with_capacity(nb_cpus + 1);
        for (nb, frequency) in frequencies.iter().enumerate() {
            ret.push(Cpu {
                inner: CpuInner::new_with_values(
                    format!("CPU {}", nb + 1),
                    vendor_id.clone(),
                    brand.clone(),
                    *frequency,
                ),
            });
        }
        ret
    }
}

pub(crate) struct KeyHandler {
    pub unique_id: String,
}

impl KeyHandler {
    pub fn new(unique_id: String) -> Self {
        Self { unique_id }
    }
}
