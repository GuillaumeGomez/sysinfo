// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::system::get_sys_value;

use crate::{CpuRefreshKind, ProcessorExt};

use libc::{c_char, host_processor_info, mach_task_self};
use std::mem;
use std::ops::Deref;
use std::sync::Arc;

pub(crate) struct UnsafePtr<T>(*mut T);

unsafe impl<T> Send for UnsafePtr<T> {}
unsafe impl<T> Sync for UnsafePtr<T> {}

impl<T> Deref for UnsafePtr<T> {
    type Target = *mut T;

    fn deref(&self) -> &*mut T {
        &self.0
    }
}

pub(crate) struct ProcessorData {
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
            let prev_cpu_info_size = std::mem::size_of::<i32>() as u32 * self.num_cpu_info;
            unsafe {
                libc::vm_deallocate(
                    mach_task_self(),
                    self.cpu_info.0 as _,
                    prev_cpu_info_size as _,
                );
            }
            self.cpu_info.0 = std::ptr::null_mut();
        }
    }
}

#[doc = include_str!("../../md_doc/processor.md")]
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

    pub(crate) fn data(&self) -> Arc<ProcessorData> {
        Arc::clone(&self.processor_data)
    }

    pub(crate) fn set_frequency(&mut self, frequency: u64) {
        self.frequency = frequency;
    }
}

impl ProcessorExt for Processor {
    fn cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    fn name(&self) -> &str {
        &self.name
    }

    /// Returns the processor frequency in MHz.
    fn frequency(&self) -> u64 {
        self.frequency
    }

    fn vendor_id(&self) -> &str {
        &self.vendor_id
    }

    fn brand(&self) -> &str {
        &self.brand
    }
}

pub(crate) fn get_cpu_frequency() -> u64 {
    let mut speed: u64 = 0;
    let mut len = std::mem::size_of::<u64>();
    unsafe {
        libc::sysctlbyname(
            b"hw.cpufrequency\0".as_ptr() as *const c_char,
            &mut speed as *mut _ as _,
            &mut len,
            std::ptr::null_mut(),
            0,
        );
        speed / 1_000_000
    }
}

#[inline]
fn get_in_use(cpu_info: *mut i32, offset: isize) -> i32 {
    unsafe {
        *cpu_info.offset(offset + libc::CPU_STATE_USER as isize)
            + *cpu_info.offset(offset + libc::CPU_STATE_SYSTEM as isize)
            + *cpu_info.offset(offset + libc::CPU_STATE_NICE as isize)
    }
}

#[inline]
fn get_idle(cpu_info: *mut i32, offset: isize) -> i32 {
    unsafe { *cpu_info.offset(offset + libc::CPU_STATE_IDLE as isize) }
}

pub(crate) fn compute_processor_usage(proc_: &Processor, cpu_info: *mut i32, offset: isize) -> f32 {
    let old_cpu_info = proc_.data().cpu_info.0;
    let in_use;
    let total;

    // In case we are initializing processors, there is no "old value" yet.
    if old_cpu_info == cpu_info {
        in_use = get_in_use(cpu_info, offset);
        total = in_use + get_idle(cpu_info, offset);
    } else {
        in_use = get_in_use(cpu_info, offset) - get_in_use(old_cpu_info, offset);
        total = in_use + (get_idle(cpu_info, offset) - get_idle(old_cpu_info, offset));
    }
    in_use as f32 / total as f32 * 100.
}

pub(crate) fn update_processor_usage<F: FnOnce(Arc<ProcessorData>, *mut i32) -> (f32, usize)>(
    port: libc::mach_port_t,
    global_processor: &mut Processor,
    f: F,
) {
    let mut num_cpu_u = 0u32;
    let mut cpu_info: *mut i32 = std::ptr::null_mut();
    let mut num_cpu_info = 0u32;

    let mut total_cpu_usage = 0f32;

    unsafe {
        if host_processor_info(
            port,
            libc::PROCESSOR_CPU_LOAD_INFO,
            &mut num_cpu_u as *mut u32,
            &mut cpu_info as *mut *mut i32,
            &mut num_cpu_info as *mut u32,
        ) == libc::KERN_SUCCESS
        {
            let (total_percentage, len) = f(
                Arc::new(ProcessorData::new(cpu_info, num_cpu_info)),
                cpu_info,
            );
            total_cpu_usage = total_percentage / len as f32;
        }
        global_processor.set_cpu_usage(total_cpu_usage);
    }
}

pub(crate) fn init_processors(
    port: libc::mach_port_t,
    processors: &mut Vec<Processor>,
    global_processor: &mut Processor,
    refresh_kind: CpuRefreshKind,
) {
    let mut num_cpu = 0;
    let mut mib = [0, 0];

    let (vendor_id, brand) = get_vendor_id_and_brand();
    let frequency = if refresh_kind.frequency() {
        get_cpu_frequency()
    } else {
        0
    };

    unsafe {
        if !get_sys_value(
            libc::CTL_HW as _,
            libc::HW_NCPU as _,
            mem::size_of::<u32>(),
            &mut num_cpu as *mut _ as *mut _,
            &mut mib,
        ) {
            num_cpu = 1;
        }
    }
    update_processor_usage(port, global_processor, |proc_data, cpu_info| {
        let mut percentage = 0f32;
        let mut offset = 0;
        for i in 0..num_cpu {
            let mut p = Processor::new(
                format!("{}", i + 1),
                Arc::clone(&proc_data),
                frequency,
                vendor_id.clone(),
                brand.clone(),
            );
            if refresh_kind.cpu_usage() {
                let cpu_usage = compute_processor_usage(&p, cpu_info, offset);
                p.set_cpu_usage(cpu_usage);
                percentage += p.cpu_usage();
            }
            processors.push(p);

            offset += libc::CPU_STATE_MAX as isize;
        }
        (percentage, processors.len())
    });

    // We didn't set them above to avoid cloning them unnecessarily.
    global_processor.brand = brand;
    global_processor.vendor_id = vendor_id;
    global_processor.frequency = frequency;
}

fn get_sysctl_str(s: &[u8]) -> String {
    let mut len = 0;

    unsafe {
        libc::sysctlbyname(
            s.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            &mut len,
            std::ptr::null_mut(),
            0,
        );
        if len < 1 {
            return String::new();
        }

        let mut buf = Vec::with_capacity(len);
        libc::sysctlbyname(
            s.as_ptr() as *const c_char,
            buf.as_mut_ptr() as _,
            &mut len,
            std::ptr::null_mut(),
            0,
        );
        if len > 0 {
            buf.set_len(len);
            while buf.last() == Some(&b'\0') {
                buf.pop();
            }
            String::from_utf8(buf).unwrap_or_else(|_| String::new())
        } else {
            String::new()
        }
    }
}

pub(crate) fn get_vendor_id_and_brand() -> (String, String) {
    // On apple M1, `sysctl machdep.cpu.vendor` returns "", so fallback to "Apple" if the result
    // is empty.
    let mut vendor = get_sysctl_str(b"machdep.cpu.vendor\0");
    if vendor.is_empty() {
        vendor = "Apple".to_string();
    }

    (vendor, get_sysctl_str(b"machdep.cpu.brand_string\0"))
}

#[cfg(test)]
mod test {
    use crate::*;
    use std::process::Command;

    #[test]
    fn check_vendor_and_brand() {
        let child = Command::new("sysctl")
            .arg("-a")
            .output()
            .expect("Failed to start command...");

        assert!(child.status.success());
        let stdout = String::from_utf8(child.stdout).expect("Not valid UTF8");

        let sys = System::new_with_specifics(
            crate::RefreshKind::new().with_cpu(CpuRefreshKind::new().with_cpu_usage()),
        );
        let processors = sys.processors();
        assert!(!processors.is_empty(), "no processor found");
        if let Some(line) = stdout.lines().find(|l| l.contains("machdep.cpu.vendor")) {
            let sysctl_value = line.split(":").skip(1).next().unwrap();
            assert_eq!(processors[0].vendor_id(), sysctl_value.trim());
        }
        if let Some(line) = stdout
            .lines()
            .find(|l| l.contains("machdep.cpu.brand_string"))
        {
            let sysctl_value = line.split(":").skip(1).next().unwrap();
            assert_eq!(processors[0].brand(), sysctl_value.trim());
        }
    }
}
