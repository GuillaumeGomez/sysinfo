// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use std::ops::Deref;
use std::sync::Arc;
use sys::ffi;

use ::ProcessorExt;

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
}

impl Processor {
    fn new(name: String, processor_data: Arc<ProcessorData>) -> Processor {
        Processor {
            name,
            cpu_usage: 0f32,
            processor_data,
        }
    }
}

impl ProcessorExt for Processor {
    fn get_cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    fn get_name(&self) -> &str {
        &self.name
    }
}

pub fn set_cpu_usage(p: &mut Processor, usage: f32) {
    p.cpu_usage = usage;
}

pub fn create_proc(name: String, processor_data: Arc<ProcessorData>) -> Processor {
    Processor::new(name, processor_data)
}

pub fn update_proc(p: &mut Processor, cpu_usage: f32, processor_data: Arc<ProcessorData>) {
    p.cpu_usage = cpu_usage;
    p.processor_data = processor_data;
}

pub fn set_cpu_proc(p: &mut Processor, cpu_usage: f32) {
    p.cpu_usage = cpu_usage;
}

pub fn get_processor_data(p: &Processor) -> Arc<ProcessorData> {
    Arc::clone(&p.processor_data)
}
