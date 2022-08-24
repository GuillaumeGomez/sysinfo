// Take a look at the license at the top of the repository in the LICENSE file.

#[allow(deprecated)]
use libc::{mach_timebase_info, mach_timebase_info_data_t};

use libc::{
    host_processor_info, mach_port_t, munmap, natural_t, processor_cpu_load_info,
    processor_cpu_load_info_t, sysconf, vm_page_size, PROCESSOR_CPU_LOAD_INFO, _SC_CLK_TCK,
};
use std::ptr::null_mut;

unsafe fn free_cpu_load_info(cpu_load: &mut processor_cpu_load_info_t) {
    if !cpu_load.is_null() {
        munmap(*cpu_load as _, vm_page_size);
        *cpu_load = null_mut();
    }
}

pub(crate) struct SystemTimeInfo {
    timebase_to_ns: f64,
    clock_per_sec: f64,
    old_cpu_load: processor_cpu_load_info_t,
    old_cpu_count: natural_t,
}

unsafe impl Send for SystemTimeInfo {}
unsafe impl Sync for SystemTimeInfo {}

impl SystemTimeInfo {
    #[allow(deprecated)] // Everything related to mach_timebase_info_data_t
    pub fn new(port: mach_port_t) -> Option<Self> {
        unsafe {
            let clock_ticks_per_sec = sysconf(_SC_CLK_TCK);

            // FIXME: Maybe check errno here? Problem is that if errno is not 0 before this call,
            //        we will get an error which isn't related...
            // if let Some(er) = std::io::Error::last_os_error().raw_os_error() {
            //     if err != 0 {
            //         println!("==> {:?}", er);
            //         sysinfo_debug!("Failed to get _SC_CLK_TCK value, using old CPU tick measure system");
            //         return None;
            //     }
            // }

            let mut info = mach_timebase_info_data_t { numer: 0, denom: 0 };
            if mach_timebase_info(&mut info) != libc::KERN_SUCCESS {
                sysinfo_debug!("mach_timebase_info failed, using default value of 1");
                info.numer = 1;
                info.denom = 1;
            }

            let mut old_cpu_load = null_mut();
            let old_cpu_count = match Self::update_ticks(port, &mut old_cpu_load) {
                Some(c) => c,
                None => {
                    sysinfo_debug!("host_processor_info failed, using old CPU tick measure system");
                    return None;
                }
            };

            let nano_per_seconds = 1_000_000_000.;
            sysinfo_debug!("");
            Some(Self {
                timebase_to_ns: info.numer as f64 / info.denom as f64,
                clock_per_sec: nano_per_seconds / clock_ticks_per_sec as f64,
                old_cpu_load,
                old_cpu_count,
            })
        }
    }

    fn update_ticks(
        port: mach_port_t,
        cpu_load: &mut processor_cpu_load_info_t,
    ) -> Option<natural_t> {
        let mut info_size = std::mem::size_of::<processor_cpu_load_info_t>() as _;
        let mut cpu_count = 0;

        unsafe {
            free_cpu_load_info(cpu_load);

            if host_processor_info(
                port,
                PROCESSOR_CPU_LOAD_INFO,
                &mut cpu_count,
                cpu_load as *mut _ as *mut _,
                &mut info_size,
            ) != 0
            {
                sysinfo_debug!("host_processor_info failed, not updating CPU ticks usage...");
                None
            } else if cpu_count < 1 || cpu_load.is_null() {
                None
            } else {
                Some(cpu_count)
            }
        }
    }

    pub fn get_time_interval(&mut self, port: mach_port_t) -> f64 {
        let mut total = 0;
        let mut new_cpu_load = null_mut();

        let new_cpu_count = match Self::update_ticks(port, &mut new_cpu_load) {
            Some(c) => c,
            None => return 0.,
        };
        let cpu_count = std::cmp::min(self.old_cpu_count, new_cpu_count);
        unsafe {
            for i in 0..cpu_count {
                let new_load: &processor_cpu_load_info = &*new_cpu_load.offset(i as _);
                let old_load: &processor_cpu_load_info = &*self.old_cpu_load.offset(i as _);
                for (new, old) in new_load.cpu_ticks.iter().zip(old_load.cpu_ticks.iter()) {
                    if new > old {
                        total += new - old;
                    }
                }
            }

            free_cpu_load_info(&mut self.old_cpu_load);
            self.old_cpu_load = new_cpu_load;
            self.old_cpu_count = new_cpu_count;

            // Now we convert the ticks to nanoseconds:
            total as f64 / self.timebase_to_ns * self.clock_per_sec / cpu_count as f64
        }
    }
}

impl Drop for SystemTimeInfo {
    fn drop(&mut self) {
        unsafe {
            free_cpu_load_info(&mut self.old_cpu_load);
        }
    }
}
