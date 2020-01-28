//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use std::collections::HashMap;
use std::mem;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use windows::tools::KeyHandler;
use ProcessorExt;

use ntapi::ntpoapi::PROCESSOR_POWER_INFORMATION;

use winapi::shared::minwindef::{FALSE, ULONG};
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::handleapi::CloseHandle;
use winapi::um::pdh::{
    PdhAddCounterW, PdhCloseQuery, PdhCollectQueryData, PdhCollectQueryDataEx,
    PdhGetFormattedCounterValue, PdhOpenQueryA, PdhRemoveCounter, PDH_FMT_COUNTERVALUE,
    PDH_FMT_DOUBLE, PDH_FMT_LARGE, PDH_HCOUNTER, PDH_HQUERY,
};
use winapi::um::powerbase::CallNtPowerInformation;
use winapi::um::synchapi::{CreateEventA, WaitForSingleObject};
use winapi::um::sysinfoapi::SYSTEM_INFO;
use winapi::um::winbase::{INFINITE, WAIT_OBJECT_0};
use winapi::um::winnt::{HANDLE, ProcessorInformation};

#[derive(Debug)]
pub enum CounterValue {
    Float(f32),
    Integer(u64),
}

impl CounterValue {
    pub fn get_f32(&self) -> f32 {
        match *self {
            CounterValue::Float(v) => v,
            _ => panic!("not a float"),
        }
    }

    pub fn get_u64(&self) -> u64 {
        match *self {
            CounterValue::Integer(v) => v,
            _ => panic!("not an integer"),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct Counter {
    counter: PDH_HCOUNTER,
    value: CounterValue,
    getter: Vec<u16>,
}

impl Counter {
    fn new_f32(counter: PDH_HCOUNTER, value: f32, getter: Vec<u16>) -> Counter {
        Counter {
            counter: counter,
            value: CounterValue::Float(value),
            getter: getter,
        }
    }

    fn new_u64(counter: PDH_HCOUNTER, value: u64, getter: Vec<u16>) -> Counter {
        Counter {
            counter: counter,
            value: CounterValue::Integer(value),
            getter: getter,
        }
    }
}

struct InternalQuery {
    query: PDH_HQUERY,
    event: HANDLE,
    data: Mutex<HashMap<String, Counter>>,
}

unsafe impl Send for InternalQuery {}
unsafe impl Sync for InternalQuery {}

impl InternalQuery {
    pub fn record(&self) -> bool {
        unsafe {
            let status = PdhCollectQueryData(self.query);
            if status != ERROR_SUCCESS as i32 {
                eprintln!("PdhCollectQueryData error: {:x} {:?}", status, self.query);
                return false;
            }
            if PdhCollectQueryDataEx(self.query, 1, self.event) != ERROR_SUCCESS as i32 {
                return false;
            }
            if WaitForSingleObject(self.event, INFINITE) == WAIT_OBJECT_0 {
                if let Ok(ref mut data) = self.data.lock() {
                    let mut counter_type: ULONG = 0;
                    let mut display_value: PDH_FMT_COUNTERVALUE = ::std::mem::zeroed();
                    for (_, x) in data.iter_mut() {
                        match x.value {
                            CounterValue::Float(ref mut value) => {
                                if PdhGetFormattedCounterValue(
                                    x.counter,
                                    PDH_FMT_DOUBLE,
                                    &mut counter_type,
                                    &mut display_value,
                                ) == ERROR_SUCCESS as i32
                                {
                                    *value = *display_value.u.doubleValue() as f32 / 100f32;
                                }
                            }
                            CounterValue::Integer(ref mut value) => {
                                if PdhGetFormattedCounterValue(
                                    x.counter,
                                    PDH_FMT_LARGE,
                                    &mut counter_type,
                                    &mut display_value,
                                ) == ERROR_SUCCESS as i32
                                {
                                    *value = *display_value.u.largeValue() as u64;
                                }
                            }
                        }
                    }
                }
                true
            } else {
                false
            }
        }
    }
}

impl Drop for InternalQuery {
    fn drop(&mut self) {
        unsafe {
            if let Ok(ref data) = self.data.lock() {
                for (_, counter) in data.iter() {
                    PdhRemoveCounter(counter.counter);
                }
            }

            if !self.event.is_null() {
                CloseHandle(self.event);
            }

            if !self.query.is_null() {
                PdhCloseQuery(self.query);
            }
        }
    }
}

pub struct Query {
    internal: Arc<InternalQuery>,
    thread: Option<JoinHandle<()>>,
}

impl Query {
    pub fn new() -> Option<Query> {
        let mut query = ::std::ptr::null_mut();
        unsafe {
            if PdhOpenQueryA(::std::ptr::null_mut(), 0, &mut query) == ERROR_SUCCESS as i32 {
                let event = CreateEventA(
                    ::std::ptr::null_mut(),
                    FALSE,
                    FALSE,
                    b"some_ev\0".as_ptr() as *const i8,
                );
                if event.is_null() {
                    PdhCloseQuery(query);
                    None
                } else {
                    let q = Arc::new(InternalQuery {
                        query: query,
                        event: event,
                        data: Mutex::new(HashMap::new()),
                    });
                    Some(Query {
                        internal: q,
                        thread: None,
                    })
                }
            } else {
                None
            }
        }
    }

    pub fn get(&self, name: &String) -> Option<f32> {
        if let Ok(data) = self.internal.data.lock() {
            if let Some(ref counter) = data.get(name) {
                return Some(counter.value.get_f32());
            }
        }
        None
    }

    pub fn get_u64(&self, name: &String) -> Option<u64> {
        if let Ok(data) = self.internal.data.lock() {
            if let Some(ref counter) = data.get(name) {
                return Some(counter.value.get_u64());
            }
        }
        None
    }

    pub fn add_counter(&mut self, name: &String, getter: Vec<u16>, value: CounterValue) -> bool {
        if let Ok(data) = self.internal.data.lock() {
            if data.contains_key(name) {
                return false;
            }
        }
        unsafe {
            let mut counter: PDH_HCOUNTER = ::std::mem::zeroed();
            let ret = PdhAddCounterW(self.internal.query, getter.as_ptr(), 0, &mut counter);
            if ret == ERROR_SUCCESS as i32 {
                self.internal
                    .data
                    .lock()
                    .expect("couldn't add counter...")
                    .insert(
                        name.clone(),
                        match value {
                            CounterValue::Float(v) => Counter::new_f32(counter, v, getter),
                            CounterValue::Integer(v) => Counter::new_u64(counter, v, getter),
                        },
                    );
            } else {
                eprintln!("failed to add counter '{}': {:x}...", name, ret);
                return false;
            }
        }
        true
    }

    pub fn start(&mut self) {
        let internal = Arc::clone(&self.internal);
        self.thread = Some(thread::spawn(move || loop {
            internal.record();
        }));
    }
}

/// Struct containing a processor information.
pub struct Processor {
    name: String,
    cpu_usage: f32,
    key_idle: Option<KeyHandler>,
    key_used: Option<KeyHandler>,
    vendor_id: String,
    frequency: u64,
}

impl ProcessorExt for Processor {
    fn get_cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_frequency(&self) -> u64 {
        self.frequency
    }

    fn get_vendor_id(&self) -> &str {
        &self.vendor_id
    }
}

impl Processor {
    pub(crate) fn new_with_values(name: &str, vendor_id: String, frequency: u64) -> Processor {
        Processor {
            name: name.to_owned(),
            cpu_usage: 0f32,
            key_idle: None,
            key_used: None,
            vendor_id,
            frequency,
        }
    }

    pub(crate) fn set_cpu_usage(&mut self, value: f32) {
       self.cpu_usage = value;
    }
}

fn get_vendor_id_not_great(info: &SYSTEM_INFO) -> String {
    use winapi::um::winnt;
    // https://docs.microsoft.com/fr-fr/windows/win32/api/sysinfoapi/ns-sysinfoapi-system_info
    match unsafe { info.u.s() }.wProcessorArchitecture {
        winnt::PROCESSOR_ARCHITECTURE_INTEL => "Intel x86",
        winnt::PROCESSOR_ARCHITECTURE_MIPS => "MIPS",
        winnt::PROCESSOR_ARCHITECTURE_ALPHA => "RISC Alpha",
        winnt::PROCESSOR_ARCHITECTURE_PPC => "PPC",
        winnt::PROCESSOR_ARCHITECTURE_SHX => "SHX",
        winnt::PROCESSOR_ARCHITECTURE_ARM => "ARM",
        winnt::PROCESSOR_ARCHITECTURE_IA64 => "Intel Itanium-based x64",
        winnt::PROCESSOR_ARCHITECTURE_ALPHA64 => "RISC Alpha x64",
        winnt::PROCESSOR_ARCHITECTURE_MSIL => "MSIL",
        winnt::PROCESSOR_ARCHITECTURE_AMD64 => "(Intel or AMD) x64",
        winnt::PROCESSOR_ARCHITECTURE_IA32_ON_WIN64 => "Intel Itanium-based x86",
        winnt::PROCESSOR_ARCHITECTURE_NEUTRAL => "unknown",
        winnt::PROCESSOR_ARCHITECTURE_ARM64 => "ARM x64",
        winnt::PROCESSOR_ARCHITECTURE_ARM32_ON_WIN64 => "ARM",
        winnt::PROCESSOR_ARCHITECTURE_IA32_ON_ARM64 => "Intel Itanium-based x86",
        _ => "unknown"
    }.to_owned()
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub fn get_vendor_id(info: &SYSTEM_INFO) -> String {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::__cpuid;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::__cpuid;

    fn add_u32(v: &mut Vec<u8>, i: u32) {
        let i = &i as *const u32 as *const u8;
        unsafe {
            v.push(*i);
            v.push(*i.offset(1));
            v.push(*i.offset(2));
            v.push(*i.offset(3));
        }
    }

    // First, we try to get the complete name.
    let res = unsafe { __cpuid(0x80000000) };
    let n_ex_ids = res.eax;
    if n_ex_ids >= 0x80000004 {
        let mut extdata = Vec::with_capacity(5);

        for i in 0x80000000..=n_ex_ids {
            extdata.push(unsafe { __cpuid(i) });
        }

        let mut out = Vec::with_capacity(4 * 4 * 3); // 4 * u32 * nb_entries
        for i in 2..5 {
            add_u32(&mut out, extdata[i].eax);
            add_u32(&mut out, extdata[i].ebx);
            add_u32(&mut out, extdata[i].ecx);
            add_u32(&mut out, extdata[i].edx);
        }
        let mut pos = 0;
        for e in out.iter() {
            if *e == 0 {
                break;
            }
            pos += 1;
        }
        match ::std::str::from_utf8(&out[..pos]) {
            Ok(s) => return s.to_owned(),
            _ => {},
        }
    }

    // Failed to get full name, let's retry for the short version!
    let res = unsafe { __cpuid(0) };
    let mut x = Vec::with_capacity(16); // 3 * u32
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
    match ::std::str::from_utf8(&x[..pos]) {
        Ok(s) => s.to_owned(),
        Err(_) => get_vendor_id_not_great(info),
    }
}

#[cfg(all(not(target_arch = "x86_64"), not(target_arch = "x86")))]
pub fn get_vendor_id(info: &SYSTEM_INFO) -> String {
    get_vendor_id_not_great(info)
}

pub fn get_key_idle(p: &mut Processor) -> &mut Option<KeyHandler> {
    &mut p.key_idle
}

pub fn get_key_used(p: &mut Processor) -> &mut Option<KeyHandler> {
    &mut p.key_used
}

// From https://stackoverflow.com/a/43813138:
//
// If your PC has 64 or fewer logical processors installed, the above code will work fine. However,
// if your PC has more than 64 logical processors installed, use GetActiveProcessorCount() or
// GetLogicalProcessorInformation() to determine the total number of logical processors installed.
pub fn get_frequencies(nb_processors: usize) -> Vec<u64> {
   let size = nb_processors * mem::size_of::<PROCESSOR_POWER_INFORMATION>();
   let mut infos: Vec<PROCESSOR_POWER_INFORMATION> = Vec::with_capacity(nb_processors);

    if unsafe { CallNtPowerInformation(ProcessorInformation, ::std::ptr::null_mut(), 0, infos.as_mut_ptr() as _, size as _) } == 0 {
        unsafe { infos.set_len(nb_processors); }
        // infos.Number
        infos.into_iter().map(|i| i.CurrentMhz as u64).collect::<Vec<_>>()
    } else {
        vec![0; nb_processors]
    }
}
