// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::{self/*, sleep*/, JoinHandle};
//use std::time::Duration;

use ProcessorExt;
use windows::tools::KeyHandler;

use winapi::shared::minwindef::{FALSE, ULONG};
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::pdh::{
    PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_FMT_LARGE, PDH_HCOUNTER, PDH_HQUERY, PdhAddCounterW,
    PdhCollectQueryData, PdhCollectQueryDataEx, PdhGetFormattedCounterValue, PdhOpenQueryA,
};
use winapi::um::synchapi::{CreateEventA, WaitForSingleObject};
use winapi::um::winbase::{INFINITE, WAIT_OBJECT_0};
use winapi::um::winnt::HANDLE;

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
                println!("error: {:?} {} {:?}", status, ERROR_SUCCESS, self.query);
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
                                if PdhGetFormattedCounterValue(x.counter,
                                                               PDH_FMT_DOUBLE,
                                                               &mut counter_type,
                                                               &mut display_value) == ERROR_SUCCESS as i32 {
                                    *value = *display_value.u.doubleValue() as f32 / 100f32;
                                }
                            }
                            CounterValue::Integer(ref mut value) => {
                                if PdhGetFormattedCounterValue(x.counter,
                                                               PDH_FMT_LARGE,
                                                               &mut counter_type,
                                                               &mut display_value) == ERROR_SUCCESS as i32 {
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

pub struct Query {
    internal: Arc<InternalQuery>,
    thread: Option<JoinHandle<()>>,
}

impl Query {
    pub fn new() -> Option<Query> {
        let mut query = ::std::ptr::null_mut();
        unsafe {
            if PdhOpenQueryA(::std::ptr::null_mut(), 0, &mut query) == ERROR_SUCCESS as i32 {
                let event = CreateEventA(::std::ptr::null_mut(), FALSE, FALSE,
                                         b"some_ev\0".as_ptr() as *const i8);
                if event.is_null() {
                    None
                } else {
                    let q = Arc::new(InternalQuery {
                        query: query,
                        event: event,
                        data: Mutex::new(HashMap::new()),
                    });
                    Some(
                        Query {
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
            let ret = PdhAddCounterW(self.internal.query,
                                     getter.as_ptr(),
                                     0,
                                     &mut counter);
            if ret == ERROR_SUCCESS as i32 {
                self.internal.data.lock()
                                  .expect("couldn't add counter...")
                                  .insert(name.clone(),
                                          match value {
                                              CounterValue::Float(v) => Counter::new_f32(counter, v, getter),
                                              CounterValue::Integer(v) => Counter::new_u64(counter, v, getter),
                                          });
            } else {
                eprintln!("failed to add counter '{}': {:x}...", name, ret);
                return false;
            }
        }
        true
    }

    pub fn start(&mut self) {
        let internal = Arc::clone(&self.internal);
        self.thread = Some(
            thread::spawn(move || {
                loop {
                    internal.record();
                }
            }));
    }
}

/// Struct containing a processor information.
pub struct Processor {
    name: String,
    cpu_usage: f32,
    key_idle: Option<KeyHandler>,
    key_used: Option<KeyHandler>,
}

impl ProcessorExt for Processor {
    fn get_cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    fn get_name(&self) -> &str {
        &self.name
    }
}

impl Processor {
    fn new_with_values(name: &str) -> Processor {
        Processor {
            name: name.to_owned(),
            cpu_usage: 0f32,
            key_idle: None,
            key_used: None,
        }
    }
}

pub fn create_processor(name: &str) -> Processor {
    Processor::new_with_values(name)
}

pub fn set_cpu_usage(p: &mut Processor, value: f32) {
    p.cpu_usage = value;
}

pub fn get_key_idle(p: &mut Processor) -> &mut Option<KeyHandler> {
    &mut p.key_idle
}

pub fn get_key_used(p: &mut Processor) -> &mut Option<KeyHandler> {
    &mut p.key_used
}
