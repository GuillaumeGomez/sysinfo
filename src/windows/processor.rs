// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use winapi::shared::minwindef::{FALSE, ULONG};
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::pdh::{
    PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PdhAddCounterA,
    PdhCollectQueryData, PdhCollectQueryDataEx, PdhGetFormattedCounterValue, PdhOpenQueryA,
};
use winapi::um::synchapi::{CreateEventA, WaitForSingleObject};
use winapi::um::winbase::{INFINITE, WAIT_OBJECT_0};
use winapi::um::winnt::HANDLE;

struct InternalQuery {
    query: PDH_HQUERY,
    event: HANDLE,
    data: Mutex<HashMap<String, (PDH_HCOUNTER, f32)>>,
}

unsafe impl Send for InternalQuery {}
unsafe impl Sync for InternalQuery {}

impl InternalQuery {
    pub fn record(&mut self) -> bool {
        unsafe {
            if PdhCollectQueryData(self.query) != ERROR_SUCCESS as i32 {
                return false;
            }
            if PdhCollectQueryDataEx(self.query, 1, self.event) != ERROR_SUCCESS as i32 {
                return false;
            }
            if WaitForSingleObject(self.event, INFINITE) == WAIT_OBJECT_0 {
                if let Ok(ref mut data) = self.data.lock() {
                    let mut counter_type: ULONG = 0;
                    let mut display_value: PDH_FMT_COUNTERVALUE = ::std::mem::zeroed();
                    for (name, x) in data.iter_mut() {
                        if PdhGetFormattedCounterValue(x.0, PDH_FMT_DOUBLE,
                                                            &mut counter_type,
                                                            &mut display_value) == ERROR_SUCCESS as i32 {
                            x.1 = *display_value.u.doubleValue() as f32;
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
                    let mut q = Arc::new(InternalQuery {
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
        if let Some(&(_, v)) = self.internal.data.lock().unwrap().get(name) {
            Some(v)
        } else {
            None
        }
    }

    pub fn add_counter(&mut self, name: &String, getter: &[u8]) {
        if self.internal.data.lock().unwrap().contains_key(name) {
            return;
        }
        let mut counter = ::std::ptr::null_mut();
        unsafe {
            if PdhAddCounterA(self.internal.query,
                              getter.as_ptr() as *const i8,
                              0,
                              &mut counter) == ERROR_SUCCESS as i32 {
                self.internal.data.lock().unwrap().insert(name.clone(), (counter, 0f32));
            }
        }
    }

    pub fn start(&mut self) {
        let mut q_clone = self.internal.clone();
        self.thread = Some(
            thread::spawn(move || {
                loop {
                    q_clone.record();
                }
            }));
    }
}

/// Struct containing a processor information.
pub struct Processor {
    name: String,
    cpu_usage: f32,
}

impl Processor {
    #[allow(dead_code)]
    fn new() -> Processor {
        Processor {
            name: String::new(),
            cpu_usage: 0f32,
        }
    }

    fn new_with_values(name: &str) -> Processor {
        Processor {
            name: name.to_owned(),
            cpu_usage: 0f32,
        }
    }

    /// Returns this processor's usage.
    pub fn get_cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    /// Returns this processor's name.
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

pub fn create_processor(name: &str) -> Processor {
    Processor::new_with_values(name)
}

pub fn set_cpu_usage(p: &mut Processor, value: f32) {
    p.cpu_usage = value;
}
