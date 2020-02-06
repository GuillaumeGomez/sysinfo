//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use libc::c_char;
use std::ops::Deref;
use std::sync::Arc;
use sys::ffi;

use ProcessorExt;

pub struct UnsafePtr<T>(*mut T);

unsafe impl<T> Send for UnsafePtr<T> {}
unsafe impl<T> Sync for UnsafePtr<T> {}

impl<T> Deref for UnsafePtr<T> {
    type Target = *mut T;

    fn deref(&self) -> &*mut T {
        &self.0
    }
}

pub struct ProcessorData {
    pub cpu_info: UnsafePtr<i32>,
    pub num_cpu_info: u32,
}

impl ProcessorData {
    pub fn new(cpu_info: *mut i32, num_cpu_info: u32) -> ProcessorData {
        ProcessorData {
            cpu_info: UnsafePtr(cpu_info),
            num_cpu_info,
        }
    }
}

impl Drop for ProcessorData {
    fn drop(&mut self) {
        if !self.cpu_info.0.is_null() {
            let prev_cpu_info_size = ::std::mem::size_of::<i32>() as u32 * self.num_cpu_info;
            unsafe {
                ffi::vm_deallocate(ffi::mach_task_self(), self.cpu_info.0, prev_cpu_info_size);
            }
            self.cpu_info.0 = ::std::ptr::null_mut();
        }
    }
}

/// Struct containing a processor information.
pub struct Processor {
    name: String,
    cpu_usage: f32,
    processor_data: Arc<ProcessorData>,
    frequency: u64,
    vendor_id: String,
    brand: String,
}

impl Processor {
    pub(crate) fn new(
        name: String,
        processor_data: Arc<ProcessorData>,
        frequency: u64,
        vendor_id: String,
        brand: String,
    ) -> Processor {
        Processor {
            name,
            cpu_usage: 0f32,
            processor_data,
            frequency,
            vendor_id,
            brand,
        }
    }

    pub(crate) fn set_cpu_usage(&mut self, cpu_usage: f32) {
        self.cpu_usage = cpu_usage;
    }

    pub(crate) fn update(&mut self, cpu_usage: f32, processor_data: Arc<ProcessorData>) {
        self.cpu_usage = cpu_usage;
        self.processor_data = processor_data;
    }

    pub(crate) fn get_data(&self) -> Arc<ProcessorData> {
        Arc::clone(&self.processor_data)
    }
}

impl ProcessorExt for Processor {
    fn get_cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    /// Returns the processor frequency in MHz.
    fn get_frequency(&self) -> u64 {
        self.frequency
    }

    fn get_vendor_id(&self) -> &str {
        &self.vendor_id
    }

    fn get_brand(&self) -> &str {
        &self.brand
    }
}

pub fn get_cpu_frequency() -> u64 {
    let mut speed: u64 = 0;
    let mut len = std::mem::size_of::<u64>();
    unsafe {
        ffi::sysctlbyname(
            b"hw.cpufrequency\0".as_ptr() as *const c_char,
            &mut speed as *mut _ as _,
            &mut len,
            std::ptr::null_mut(),
            0,
        );
    }
    speed / 1_000_000
}

fn get_sysctl_str(s: &[u8]) -> String {
    let mut len = 0;

    unsafe {
        ffi::sysctlbyname(
            s.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            &mut len,
            std::ptr::null_mut(),
            0,
        );
    }
    if len < 1 {
        return String::new();
    }
    let mut buf = Vec::with_capacity(len);
    unsafe {
        ffi::sysctlbyname(
            s.as_ptr() as *const c_char,
            buf.as_mut_ptr() as _,
            &mut len,
            std::ptr::null_mut(),
            0,
        );
    }
    if len > 0 {
        unsafe {
            buf.set_len(len);
        }
        String::from_utf8(buf).unwrap_or_else(|_| String::new())
    } else {
        String::new()
    }
}

pub fn get_vendor_id_and_brand() -> (String, String) {
    (
        get_sysctl_str(b"machdep.cpu.brand_string\0"),
        get_sysctl_str(b"machdep.cpu.vendor\0"),
    )
}
