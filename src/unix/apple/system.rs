// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::cpu::*;
#[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
use crate::sys::process::*;
use crate::sys::utils::{get_sys_value, get_sys_value_by_name};

use crate::{Cpu, CpuRefreshKind, LoadAvg, Pid, Process, ProcessRefreshKind};

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::mem;
#[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
use std::time::SystemTime;

#[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
use libc::size_t;

use libc::{
    c_int, c_void, host_statistics64, mach_port_t, sysconf, sysctl, timeval, vm_statistics64,
    _SC_PAGESIZE,
};

pub(crate) struct SystemInner {
    process_list: HashMap<Pid, Process>,
    mem_total: u64,
    mem_free: u64,
    mem_used: u64,
    mem_available: u64,
    swap_total: u64,
    swap_free: u64,
    page_size_b: u64,
    port: mach_port_t,
    boot_time: u64,
    #[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
    clock_info: Option<crate::sys::macos::system::SystemTimeInfo>,
    cpus: CpusWrapper,
}

pub(crate) struct Wrap<'a>(pub UnsafeCell<&'a mut HashMap<Pid, Process>>);

unsafe impl<'a> Send for Wrap<'a> {}
unsafe impl<'a> Sync for Wrap<'a> {}

fn boot_time() -> u64 {
    let mut boot_time = timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let mut len = std::mem::size_of::<timeval>();
    let mut mib: [c_int; 2] = [libc::CTL_KERN, libc::KERN_BOOTTIME];

    unsafe {
        if sysctl(
            mib.as_mut_ptr(),
            mib.len() as _,
            &mut boot_time as *mut timeval as *mut _,
            &mut len,
            std::ptr::null_mut(),
            0,
        ) < 0
        {
            0
        } else {
            boot_time.tv_sec as _
        }
    }
}

#[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
fn get_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|n| n.as_secs())
        .unwrap_or(0)
}

impl SystemInner {
    pub(crate) fn new() -> Self {
        unsafe {
            let port = libc::mach_host_self();

            Self {
                process_list: HashMap::with_capacity(200),
                mem_total: 0,
                mem_free: 0,
                mem_available: 0,
                mem_used: 0,
                swap_total: 0,
                swap_free: 0,
                page_size_b: sysconf(_SC_PAGESIZE) as _,
                port,
                boot_time: boot_time(),
                #[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
                clock_info: crate::sys::macos::system::SystemTimeInfo::new(port),
                cpus: CpusWrapper::new(),
            }
        }
    }

    pub(crate) fn refresh_memory(&mut self) {
        let mut mib = [0, 0];

        unsafe {
            // get system values
            // get swap info
            let mut xs: libc::xsw_usage = mem::zeroed::<libc::xsw_usage>();
            if get_sys_value(
                libc::CTL_VM as _,
                libc::VM_SWAPUSAGE as _,
                mem::size_of::<libc::xsw_usage>(),
                &mut xs as *mut _ as *mut c_void,
                &mut mib,
            ) {
                self.swap_total = xs.xsu_total;
                self.swap_free = xs.xsu_avail;
            }
            // get ram info
            if self.mem_total < 1 {
                get_sys_value(
                    libc::CTL_HW as _,
                    libc::HW_MEMSIZE as _,
                    mem::size_of::<u64>(),
                    &mut self.mem_total as *mut u64 as *mut c_void,
                    &mut mib,
                );
            }
            let mut count: u32 = libc::HOST_VM_INFO64_COUNT as _;
            let mut stat = mem::zeroed::<vm_statistics64>();
            if host_statistics64(
                self.port,
                libc::HOST_VM_INFO64,
                &mut stat as *mut vm_statistics64 as *mut _,
                &mut count,
            ) == libc::KERN_SUCCESS
            {
                // From the apple documentation:
                //
                // /*
                //  * NB: speculative pages are already accounted for in "free_count",
                //  * so "speculative_count" is the number of "free" pages that are
                //  * used to hold data that was read speculatively from disk but
                //  * haven't actually been used by anyone so far.
                //  */
                self.mem_available = u64::from(stat.free_count)
                    .saturating_add(u64::from(stat.inactive_count))
                    .saturating_add(u64::from(stat.purgeable_count))
                    .saturating_sub(u64::from(stat.compressor_page_count))
                    .saturating_mul(self.page_size_b);
                self.mem_used = u64::from(stat.active_count)
                    .saturating_add(u64::from(stat.wire_count))
                    .saturating_add(u64::from(stat.compressor_page_count))
                    .saturating_add(u64::from(stat.speculative_count))
                    .saturating_mul(self.page_size_b);
                self.mem_free = u64::from(stat.free_count)
                    .saturating_sub(u64::from(stat.speculative_count))
                    .saturating_mul(self.page_size_b);
            }
        }
    }

    pub(crate) fn cgroup_limits(&self) -> Option<crate::CGroupLimits> {
        None
    }

    pub(crate) fn refresh_cpu_specifics(&mut self, refresh_kind: CpuRefreshKind) {
        self.cpus.refresh(refresh_kind, self.port);
    }

    #[cfg(any(target_os = "ios", feature = "apple-sandbox"))]
    pub(crate) fn refresh_processes_specifics(&mut self, _refresh_kind: ProcessRefreshKind) {}

    #[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
    pub(crate) fn refresh_processes_specifics(&mut self, refresh_kind: ProcessRefreshKind) {
        use crate::utils::into_iter;

        unsafe {
            let count = libc::proc_listallpids(::std::ptr::null_mut(), 0);
            if count < 1 {
                return;
            }
        }
        if let Some(pids) = get_proc_list() {
            let now = get_now();
            let arg_max = get_arg_max();
            let port = self.port;
            let time_interval = self.clock_info.as_mut().map(|c| c.get_time_interval(port));
            let entries: Vec<Process> = {
                let wrap = &Wrap(UnsafeCell::new(&mut self.process_list));

                #[cfg(feature = "multithread")]
                use rayon::iter::ParallelIterator;

                into_iter(pids)
                    .flat_map(|pid| {
                        match update_process(
                            wrap,
                            pid,
                            arg_max as size_t,
                            time_interval,
                            now,
                            refresh_kind,
                            false,
                        ) {
                            Ok(x) => x,
                            _ => None,
                        }
                    })
                    .collect()
            };
            entries.into_iter().for_each(|entry| {
                self.process_list.insert(entry.pid(), entry);
            });
            self.process_list
                .retain(|_, proc_| std::mem::replace(&mut proc_.inner.updated, false));
        }
    }

    #[cfg(any(target_os = "ios", feature = "apple-sandbox"))]
    pub(crate) fn refresh_process_specifics(
        &mut self,
        _pid: Pid,
        _refresh_kind: ProcessRefreshKind,
    ) -> bool {
        false
    }

    #[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
    pub(crate) fn refresh_process_specifics(
        &mut self,
        pid: Pid,
        refresh_kind: ProcessRefreshKind,
    ) -> bool {
        let mut time_interval = None;
        let arg_max = get_arg_max();
        let now = get_now();

        if refresh_kind.cpu() {
            let port = self.port;
            time_interval = self.clock_info.as_mut().map(|c| c.get_time_interval(port));
        }
        match {
            let wrap = Wrap(UnsafeCell::new(&mut self.process_list));
            update_process(
                &wrap,
                pid,
                arg_max as size_t,
                time_interval,
                now,
                refresh_kind,
                true,
            )
        } {
            Ok(Some(p)) => {
                self.process_list.insert(p.pid(), p);
                true
            }
            Ok(_) => true,
            Err(_) => false,
        }
    }

    // COMMON PART
    //
    // Need to be moved into a "common" file to avoid duplication.

    pub(crate) fn processes(&self) -> &HashMap<Pid, Process> {
        &self.process_list
    }

    pub(crate) fn process(&self, pid: Pid) -> Option<&Process> {
        self.process_list.get(&pid)
    }

    pub(crate) fn global_cpu_info(&self) -> &Cpu {
        &self.cpus.global_cpu
    }

    pub(crate) fn cpus(&self) -> &[Cpu] {
        &self.cpus.cpus
    }

    pub(crate) fn physical_core_count(&self) -> Option<usize> {
        physical_core_count()
    }

    pub(crate) fn total_memory(&self) -> u64 {
        self.mem_total
    }

    pub(crate) fn free_memory(&self) -> u64 {
        self.mem_free
    }

    pub(crate) fn available_memory(&self) -> u64 {
        self.mem_available
    }

    pub(crate) fn used_memory(&self) -> u64 {
        self.mem_used
    }

    pub(crate) fn total_swap(&self) -> u64 {
        self.swap_total
    }

    pub(crate) fn free_swap(&self) -> u64 {
        self.swap_free
    }

    // TODO: need to be checked
    pub(crate) fn used_swap(&self) -> u64 {
        self.swap_total - self.swap_free
    }

    pub(crate) fn uptime(&self) -> u64 {
        unsafe {
            let csec = libc::time(::std::ptr::null_mut());

            libc::difftime(csec, self.boot_time as _) as u64
        }
    }

    pub(crate) fn load_average(&self) -> LoadAvg {
        let mut loads = vec![0f64; 3];

        unsafe {
            libc::getloadavg(loads.as_mut_ptr(), 3);
            LoadAvg {
                one: loads[0],
                five: loads[1],
                fifteen: loads[2],
            }
        }
    }

    pub(crate) fn boot_time(&self) -> u64 {
        self.boot_time
    }

    pub(crate) fn name(&self) -> Option<String> {
        get_system_info(libc::KERN_OSTYPE, Some("Darwin"))
    }

    pub(crate) fn long_os_version(&self) -> Option<String> {
        #[cfg(target_os = "macos")]
        let friendly_name = match self.os_version().unwrap_or_default() {
            f_n if f_n.starts_with("14.0") => "Sonoma",
            f_n if f_n.starts_with("10.16")
                | f_n.starts_with("11.0")
                | f_n.starts_with("11.1")
                | f_n.starts_with("11.2") =>
            {
                "Big Sur"
            }
            f_n if f_n.starts_with("10.15") => "Catalina",
            f_n if f_n.starts_with("10.14") => "Mojave",
            f_n if f_n.starts_with("10.13") => "High Sierra",
            f_n if f_n.starts_with("10.12") => "Sierra",
            f_n if f_n.starts_with("10.11") => "El Capitan",
            f_n if f_n.starts_with("10.10") => "Yosemite",
            f_n if f_n.starts_with("10.9") => "Mavericks",
            f_n if f_n.starts_with("10.8") => "Mountain Lion",
            f_n if f_n.starts_with("10.7") => "Lion",
            f_n if f_n.starts_with("10.6") => "Snow Leopard",
            f_n if f_n.starts_with("10.5") => "Leopard",
            f_n if f_n.starts_with("10.4") => "Tiger",
            f_n if f_n.starts_with("10.3") => "Panther",
            f_n if f_n.starts_with("10.2") => "Jaguar",
            f_n if f_n.starts_with("10.1") => "Puma",
            f_n if f_n.starts_with("10.0") => "Cheetah",
            _ => "",
        };

        #[cfg(target_os = "macos")]
        let long_name = Some(format!(
            "MacOS {} {}",
            self.os_version().unwrap_or_default(),
            friendly_name
        ));

        #[cfg(target_os = "ios")]
        let long_name = Some(format!("iOS {}", self.os_version().unwrap_or_default()));

        long_name
    }

    pub(crate) fn host_name(&self) -> Option<String> {
        get_system_info(libc::KERN_HOSTNAME, None)
    }

    pub(crate) fn kernel_version(&self) -> Option<String> {
        get_system_info(libc::KERN_OSRELEASE, None)
    }

    pub(crate) fn os_version(&self) -> Option<String> {
        unsafe {
            // get the size for the buffer first
            let mut size = 0;
            if get_sys_value_by_name(b"kern.osproductversion\0", &mut size, std::ptr::null_mut())
                && size > 0
            {
                // now create a buffer with the size and get the real value
                let mut buf = vec![0_u8; size as _];

                if get_sys_value_by_name(
                    b"kern.osproductversion\0",
                    &mut size,
                    buf.as_mut_ptr() as *mut c_void,
                ) {
                    if let Some(pos) = buf.iter().position(|x| *x == 0) {
                        // Shrink buffer to terminate the null bytes
                        buf.resize(pos, 0);
                    }

                    String::from_utf8(buf).ok()
                } else {
                    // getting the system value failed
                    None
                }
            } else {
                // getting the system value failed, or did not return a buffer size
                None
            }
        }
    }

    pub(crate) fn distribution_id(&self) -> String {
        std::env::consts::OS.to_owned()
    }
}

#[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
fn get_arg_max() -> usize {
    let mut mib = [libc::CTL_KERN, libc::KERN_ARGMAX];
    let mut arg_max = 0i32;
    let mut size = mem::size_of::<c_int>();
    unsafe {
        if sysctl(
            mib.as_mut_ptr(),
            mib.len() as _,
            (&mut arg_max) as *mut i32 as *mut c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        ) == -1
        {
            4096 // We default to this value
        } else {
            arg_max as usize
        }
    }
}

fn get_system_info(value: c_int, default: Option<&str>) -> Option<String> {
    let mut mib: [c_int; 2] = [libc::CTL_KERN, value];
    let mut size = 0;

    unsafe {
        // Call first to get size
        sysctl(
            mib.as_mut_ptr(),
            mib.len() as _,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        );

        // exit early if we did not update the size
        if size == 0 {
            default.map(|s| s.to_owned())
        } else {
            // set the buffer to the correct size
            let mut buf = vec![0_u8; size as _];

            if sysctl(
                mib.as_mut_ptr(),
                mib.len() as _,
                buf.as_mut_ptr() as _,
                &mut size,
                std::ptr::null_mut(),
                0,
            ) == -1
            {
                // If command fails return default
                default.map(|s| s.to_owned())
            } else {
                if let Some(pos) = buf.iter().position(|x| *x == 0) {
                    // Shrink buffer to terminate the null bytes
                    buf.resize(pos, 0);
                }

                String::from_utf8(buf).ok()
            }
        }
    }
}
