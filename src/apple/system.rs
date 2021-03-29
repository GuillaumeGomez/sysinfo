//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use crate::sys::component::Component;
use crate::sys::disk::*;
use crate::sys::ffi;
use crate::sys::network::Networks;
use crate::sys::process::*;
use crate::sys::processor::*;
#[cfg(target_os = "macos")]
use core_foundation_sys::base::{kCFAllocatorDefault, CFRelease};

use crate::{LoadAvg, Pid, ProcessorExt, RefreshKind, SystemExt, User};

#[cfg(all(target_os = "macos", not(feature = "apple-app-store")))]
use crate::ProcessExt;

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::mem;
use std::sync::Arc;

#[cfg(all(target_os = "macos", not(feature = "apple-app-store")))]
use libc::size_t;

use libc::{self, c_char, c_int, c_void, natural_t, sysconf, _SC_PAGESIZE};

unsafe fn free_cpu_load_info(cpu_load: &mut libc::processor_cpu_load_info_t) {
    if !cpu_load.is_null() {
        libc::munmap(*cpu_load as _, ffi::vm_page_size);
        *cpu_load = std::ptr::null_mut();
    }
}

struct SystemTimeInfo {
    timebase_to_ns: f64,
    clock_per_sec: f64,
    old_cpu_load: libc::processor_cpu_load_info_t,
    old_cpu_count: natural_t,
}

unsafe impl Send for SystemTimeInfo {}
unsafe impl Sync for SystemTimeInfo {}

impl SystemTimeInfo {
    #[allow(deprecated)] // Everything related to mach_timebase_info_data_t
    fn new(port: libc::mach_port_t) -> Option<Self> {
        let clock_ticks_per_sec = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };

        // FIXME: Maybe check errno here? Problem is that if errno is not 0 before this call,
        //        we will get an error which isn't related...
        // if let Some(er) = std::io::Error::last_os_error().raw_os_error() {
        //     if err != 0 {
        //         println!("==> {:?}", er);
        //         sysinfo_debug!("Failed to get _SC_CLK_TCK value, using old CPU tick measure system");
        //         return None;
        //     }
        // }

        let mut info = libc::mach_timebase_info_data_t { numer: 0, denom: 0 };
        if unsafe { libc::mach_timebase_info(&mut info) } != ffi::KERN_SUCCESS {
            sysinfo_debug!("mach_timebase_info failed, using default value of 1");
            info.numer = 1;
            info.denom = 1;
        }

        let mut old_cpu_load = std::ptr::null_mut();
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

    fn update_ticks(
        port: libc::mach_port_t,
        cpu_load: &mut libc::processor_cpu_load_info_t,
    ) -> Option<natural_t> {
        let mut info_size = std::mem::size_of::<libc::processor_cpu_load_info_t>() as _;
        let mut cpu_count = 0;

        unsafe {
            free_cpu_load_info(cpu_load);
        }

        if unsafe {
            ffi::host_processor_info(
                port,
                libc::PROCESSOR_CPU_LOAD_INFO,
                &mut cpu_count,
                cpu_load as *mut _ as *mut _,
                &mut info_size,
            )
        } != 0
        {
            sysinfo_debug!("host_processor_info failed, not updating CPU ticks usage...");
            None
        } else if cpu_count < 1 || cpu_load.is_null() {
            None
        } else {
            Some(cpu_count)
        }
    }

    fn get_time_interval(&mut self, port: libc::mach_port_t) -> f64 {
        let mut total = 0;
        let mut new_cpu_load = std::ptr::null_mut();

        let new_cpu_count = match Self::update_ticks(port, &mut new_cpu_load) {
            Some(c) => c,
            None => return 0.,
        };
        let cpu_count = std::cmp::min(self.old_cpu_count, new_cpu_count);
        for i in 0..cpu_count {
            let new_load: &libc::processor_cpu_load_info = unsafe { &*new_cpu_load.offset(i as _) };
            let old_load: &libc::processor_cpu_load_info =
                unsafe { &*self.old_cpu_load.offset(i as _) };
            for (new, old) in new_load.cpu_ticks.iter().zip(old_load.cpu_ticks.iter()) {
                if new > old {
                    total += new - old;
                }
            }
        }

        unsafe {
            free_cpu_load_info(&mut self.old_cpu_load);
        }
        self.old_cpu_load = new_cpu_load;
        self.old_cpu_count = new_cpu_count;

        // Now we convert the ticks to nanoseconds:
        total as f64 / self.timebase_to_ns * self.clock_per_sec / cpu_count as f64
    }
}

impl Drop for SystemTimeInfo {
    fn drop(&mut self) {
        unsafe {
            free_cpu_load_info(&mut self.old_cpu_load);
        }
    }
}

/// Structs containing system's information.
pub struct System {
    process_list: HashMap<Pid, Process>,
    mem_total: u64,
    mem_free: u64,
    mem_available: u64,
    swap_total: u64,
    swap_free: u64,
    global_processor: Processor,
    processors: Vec<Processor>,
    page_size_kb: u64,
    components: Vec<Component>,
    // Used to get CPU information, not supported on iOS.
    #[cfg(target_os = "macos")]
    connection: Option<ffi::io_connect_t>,
    disks: Vec<Disk>,
    networks: Networks,
    port: libc::mach_port_t,
    users: Vec<User>,
    boot_time: u64,
    // Used to get disk information, to be more specific, it's needed by the
    // DADiskCreateFromVolumePath function. Not supported on iOS.
    #[cfg(target_os = "macos")]
    session: ffi::SessionWrap,
    clock_info: Option<SystemTimeInfo>,
}

impl Drop for System {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(conn) = self.connection {
            unsafe {
                ffi::IOServiceClose(conn);
            }
        }

        #[cfg(target_os = "macos")]
        if !self.session.0.is_null() {
            unsafe {
                CFRelease(self.session.0 as _);
            }
        }
    }
}

pub(crate) struct Wrap<'a>(pub UnsafeCell<&'a mut HashMap<Pid, Process>>);

unsafe impl<'a> Send for Wrap<'a> {}
unsafe impl<'a> Sync for Wrap<'a> {}

#[cfg(all(target_os = "macos", not(feature = "apple-app-store")))]
impl System {
    fn clear_procs(&mut self) {
        use crate::sys::macos::process;

        let mut to_delete = Vec::new();

        for (pid, mut proc_) in &mut self.process_list {
            if !process::has_been_updated(&mut proc_) {
                to_delete.push(*pid);
            }
        }
        for pid in to_delete {
            self.process_list.remove(&pid);
        }
    }
}

fn boot_time() -> u64 {
    let mut boot_time = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let mut len = std::mem::size_of::<libc::timeval>();
    let mut mib: [libc::c_int; 2] = [libc::CTL_KERN, libc::KERN_BOOTTIME];
    if unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            2,
            &mut boot_time as *mut libc::timeval as *mut _,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    } < 0
    {
        0
    } else {
        boot_time.tv_sec as _
    }
}

impl SystemExt for System {
    fn new_with_specifics(refreshes: RefreshKind) -> System {
        let port = unsafe { ffi::mach_host_self() };
        let (global_processor, processors) = init_processors(port);

        let mut s = System {
            process_list: HashMap::with_capacity(200),
            mem_total: 0,
            mem_free: 0,
            mem_available: 0,
            swap_total: 0,
            swap_free: 0,
            global_processor,
            processors,
            page_size_kb: unsafe { sysconf(_SC_PAGESIZE) as u64 / 1_000 },
            components: Vec::with_capacity(2),
            #[cfg(target_os = "macos")]
            connection: get_io_service_connection(),
            disks: Vec::with_capacity(1),
            networks: Networks::new(),
            port,
            users: Vec::new(),
            boot_time: boot_time(),
            #[cfg(target_os = "macos")]
            session: ffi::SessionWrap(::std::ptr::null_mut()),
            clock_info: SystemTimeInfo::new(port),
        };
        s.refresh_specifics(refreshes);
        s
    }

    fn refresh_memory(&mut self) {
        let mut mib = [0, 0];

        unsafe {
            // get system values
            // get swap info
            let mut xs: ffi::xsw_usage = mem::zeroed::<ffi::xsw_usage>();
            if get_sys_value(
                libc::CTL_VM as _,
                libc::VM_SWAPUSAGE as _,
                mem::size_of::<ffi::xsw_usage>(),
                &mut xs as *mut ffi::xsw_usage as *mut c_void,
                &mut mib,
            ) {
                self.swap_total = xs.xsu_total / 1_000;
                self.swap_free = xs.xsu_avail / 1_000;
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
                self.mem_total /= 1_000;
            }
            let count: u32 = ffi::HOST_VM_INFO64_COUNT;
            let mut stat = mem::zeroed::<ffi::vm_statistics64>();
            if ffi::host_statistics64(
                self.port,
                ffi::HOST_VM_INFO64,
                &mut stat as *mut ffi::vm_statistics64 as *mut c_void,
                &count,
            ) == ffi::KERN_SUCCESS
            {
                // From the apple documentation:
                //
                // /*
                //  * NB: speculative pages are already accounted for in "free_count",
                //  * so "speculative_count" is the number of "free" pages that are
                //  * used to hold data that was read speculatively from disk but
                //  * haven't actually been used by anyone so far.
                //  */
                self.mem_available = self.mem_total
                    - (u64::from(stat.active_count)
                        + u64::from(stat.inactive_count)
                        + u64::from(stat.wire_count)
                        + u64::from(stat.speculative_count)
                        - u64::from(stat.purgeable_count))
                        * self.page_size_kb;
                self.mem_free = u64::from(stat.free_count) * self.page_size_kb;
            }
        }
    }

    #[cfg(target_os = "ios")]
    fn refresh_components_list(&mut self) {}

    #[cfg(target_os = "macos")]
    fn refresh_components_list(&mut self) {
        if let Some(con) = self.connection {
            self.components.clear();
            // getting CPU critical temperature
            let critical_temp = crate::apple::component::get_temperature(
                con,
                &['T' as i8, 'C' as i8, '0' as i8, 'D' as i8, 0],
            );

            for (id, v) in crate::apple::component::COMPONENTS_TEMPERATURE_IDS.iter() {
                if let Some(c) = Component::new((*id).to_owned(), None, critical_temp, v, con) {
                    self.components.push(c);
                }
            }
        }
    }

    fn refresh_cpu(&mut self) {
        // get processor values
        let mut num_cpu_u = 0u32;
        let mut cpu_info: *mut i32 = std::ptr::null_mut();
        let mut num_cpu_info = 0u32;

        let mut pourcent = 0f32;

        unsafe {
            if ffi::host_processor_info(
                self.port,
                libc::PROCESSOR_CPU_LOAD_INFO,
                &mut num_cpu_u as *mut u32,
                &mut cpu_info as *mut *mut i32,
                &mut num_cpu_info as *mut u32,
            ) == ffi::KERN_SUCCESS
            {
                let proc_data = Arc::new(ProcessorData::new(cpu_info, num_cpu_info));
                let mut add = 0;
                for proc_ in self.processors.iter_mut() {
                    let old_proc_data = &*proc_.get_data();
                    let in_use = (*cpu_info.offset(add as isize + libc::CPU_STATE_USER as isize)
                        - *old_proc_data
                            .cpu_info
                            .offset(add as isize + libc::CPU_STATE_USER as isize))
                        + (*cpu_info.offset(add as isize + libc::CPU_STATE_SYSTEM as isize)
                            - *old_proc_data
                                .cpu_info
                                .offset(add as isize + libc::CPU_STATE_SYSTEM as isize))
                        + (*cpu_info.offset(add as isize + libc::CPU_STATE_NICE as isize)
                            - *old_proc_data
                                .cpu_info
                                .offset(add as isize + libc::CPU_STATE_NICE as isize));
                    let total = in_use
                        + (*cpu_info.offset(add as isize + libc::CPU_STATE_IDLE as isize)
                            - *old_proc_data
                                .cpu_info
                                .offset(add as isize + libc::CPU_STATE_IDLE as isize));
                    proc_.update(in_use as f32 / total as f32 * 100., Arc::clone(&proc_data));
                    pourcent += proc_.get_cpu_usage();

                    add += libc::CPU_STATE_MAX;
                }
            }
        }
        self.global_processor
            .set_cpu_usage(pourcent / self.processors.len() as f32);
    }

    #[cfg(any(target_os = "ios", feature = "apple-app-store"))]
    fn refresh_processes(&mut self) {}

    #[cfg(all(target_os = "macos", not(feature = "apple-app-store")))]
    fn refresh_processes(&mut self) {
        use crate::utils::into_iter;

        let count = unsafe { ffi::proc_listallpids(::std::ptr::null_mut(), 0) };
        if count < 1 {
            return;
        }
        if let Some(pids) = get_proc_list() {
            let arg_max = get_arg_max();
            let port = self.port;
            let time_interval = self.clock_info.as_mut().map(|c| c.get_time_interval(port));
            let entries: Vec<Process> = {
                let wrap = &Wrap(UnsafeCell::new(&mut self.process_list));

                #[cfg(feature = "multithread")]
                use rayon::iter::ParallelIterator;

                into_iter(pids)
                    .flat_map(|pid| {
                        match update_process(wrap, pid, arg_max as size_t, time_interval) {
                            Ok(x) => x,
                            Err(_) => None,
                        }
                    })
                    .collect()
            };
            entries.into_iter().for_each(|entry| {
                self.process_list.insert(entry.pid(), entry);
            });
            self.clear_procs();
        }
    }

    #[cfg(any(target_os = "ios", feature = "apple-app-store"))]
    fn refresh_process(&mut self, _: Pid) -> bool {
        false
    }

    #[cfg(all(target_os = "macos", not(feature = "apple-app-store")))]
    fn refresh_process(&mut self, pid: Pid) -> bool {
        let arg_max = get_arg_max();
        let port = self.port;
        let time_interval = self.clock_info.as_mut().map(|c| c.get_time_interval(port));
        match {
            let wrap = Wrap(UnsafeCell::new(&mut self.process_list));
            update_process(&wrap, pid, arg_max as size_t, time_interval)
        } {
            Ok(Some(p)) => {
                self.process_list.insert(p.pid(), p);
                true
            }
            Ok(_) => true,
            Err(_) => false,
        }
    }

    #[cfg(target_os = "ios")]
    fn refresh_disks_list(&mut self) {}

    #[cfg(target_os = "macos")]
    fn refresh_disks_list(&mut self) {
        if self.session.0.is_null() {
            self.session.0 = unsafe { ffi::DASessionCreate(kCFAllocatorDefault as _) };
        }
        self.disks = get_disks(self.session.0);
    }

    fn refresh_users_list(&mut self) {
        self.users = crate::apple::users::get_users_list();
    }

    // COMMON PART
    //
    // Need to be moved into a "common" file to avoid duplication.

    fn get_processes(&self) -> &HashMap<Pid, Process> {
        &self.process_list
    }

    fn get_process(&self, pid: Pid) -> Option<&Process> {
        self.process_list.get(&pid)
    }

    fn get_global_processor_info(&self) -> &Processor {
        &self.global_processor
    }

    fn get_processors(&self) -> &[Processor] {
        &self.processors
    }

    fn get_physical_core_count(&self) -> Option<usize> {
        let mut physical_core_count = 0;

        if unsafe {
            get_sys_value_by_name(
                b"hw.physicalcpu\0",
                &mut mem::size_of::<u32>(),
                &mut physical_core_count as *mut usize as *mut c_void,
            )
        } {
            Some(physical_core_count)
        } else {
            None
        }
    }

    fn get_networks(&self) -> &Networks {
        &self.networks
    }

    fn get_networks_mut(&mut self) -> &mut Networks {
        &mut self.networks
    }

    fn get_total_memory(&self) -> u64 {
        self.mem_total
    }

    fn get_free_memory(&self) -> u64 {
        self.mem_free
    }

    fn get_available_memory(&self) -> u64 {
        self.mem_available
    }

    fn get_used_memory(&self) -> u64 {
        self.mem_total - self.mem_free
    }

    fn get_total_swap(&self) -> u64 {
        self.swap_total
    }

    fn get_free_swap(&self) -> u64 {
        self.swap_free
    }

    // need to be checked
    fn get_used_swap(&self) -> u64 {
        self.swap_total - self.swap_free
    }

    fn get_components(&self) -> &[Component] {
        &self.components
    }

    fn get_components_mut(&mut self) -> &mut [Component] {
        &mut self.components
    }

    fn get_disks(&self) -> &[Disk] {
        &self.disks
    }

    fn get_disks_mut(&mut self) -> &mut [Disk] {
        &mut self.disks
    }

    fn get_uptime(&self) -> u64 {
        let csec = unsafe { libc::time(::std::ptr::null_mut()) };

        unsafe { libc::difftime(csec, self.boot_time as _) as u64 }
    }

    fn get_load_average(&self) -> LoadAvg {
        let mut loads = vec![0f64; 3];
        unsafe {
            libc::getloadavg(loads.as_mut_ptr(), 3);
        }
        LoadAvg {
            one: loads[0],
            five: loads[1],
            fifteen: loads[2],
        }
    }

    fn get_users(&self) -> &[User] {
        &self.users
    }

    fn get_boot_time(&self) -> u64 {
        self.boot_time
    }

    fn get_name(&self) -> Option<String> {
        get_system_info(libc::KERN_OSTYPE, Some("Darwin"))
    }

    fn get_long_os_version(&self) -> Option<String> {
        #[cfg(target_os = "macos")]
        let friendly_name = match self.get_os_version().unwrap_or_default() {
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
            self.get_os_version().unwrap_or_default(),
            friendly_name
        ));

        #[cfg(target_os = "ios")]
        let long_name = Some(format!("iOS {}", self.get_os_version().unwrap_or_default()));

        long_name
    }

    fn get_host_name(&self) -> Option<String> {
        get_system_info(libc::KERN_HOSTNAME, None)
    }

    fn get_kernel_version(&self) -> Option<String> {
        get_system_info(libc::KERN_OSRELEASE, None)
    }

    fn get_os_version(&self) -> Option<String> {
        unsafe {
            // get the size for the buffer first
            let mut size = 0;
            if get_sys_value_by_name(b"kern.osproductversion\0", &mut size, std::ptr::null_mut())
                && size > 0
            {
                // now create a buffer with the size and get the real value
                let mut buf = vec![0_u8; size as usize];

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
}

impl Default for System {
    fn default() -> System {
        System::new()
    }
}

// code from https://github.com/Chris911/iStats
// Not supported on iOS
#[cfg(target_os = "macos")]
fn get_io_service_connection() -> Option<ffi::io_connect_t> {
    let mut master_port: libc::mach_port_t = 0;
    let mut iterator: ffi::io_iterator_t = 0;

    unsafe {
        ffi::IOMasterPort(ffi::MACH_PORT_NULL, &mut master_port);

        let matching_dictionary = ffi::IOServiceMatching(b"AppleSMC\0".as_ptr() as *const i8);
        let result =
            ffi::IOServiceGetMatchingServices(master_port, matching_dictionary, &mut iterator);
        if result != ffi::KIO_RETURN_SUCCESS {
            sysinfo_debug!("Error: IOServiceGetMatchingServices() = {}", result);
            return None;
        }

        let device = ffi::IOIteratorNext(iterator);
        ffi::IOObjectRelease(iterator);
        if device == 0 {
            sysinfo_debug!("Error: no SMC found");
            return None;
        }

        let mut conn = 0;
        let result = ffi::IOServiceOpen(device, ffi::mach_task_self(), 0, &mut conn);
        ffi::IOObjectRelease(device);
        if result != ffi::KIO_RETURN_SUCCESS {
            sysinfo_debug!("Error: IOServiceOpen() = {}", result);
            return None;
        }

        Some(conn)
    }
}

#[cfg(all(target_os = "macos", not(feature = "apple-app-store")))]
fn get_arg_max() -> usize {
    let mut mib: [c_int; 3] = [libc::CTL_KERN, libc::KERN_ARGMAX, 0];
    let mut arg_max = 0i32;
    let mut size = mem::size_of::<c_int>();
    unsafe {
        if libc::sysctl(
            mib.as_mut_ptr(),
            2,
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

pub(crate) unsafe fn get_sys_value(
    high: u32,
    low: u32,
    mut len: usize,
    value: *mut libc::c_void,
    mib: &mut [i32; 2],
) -> bool {
    mib[0] = high as i32;
    mib[1] = low as i32;
    libc::sysctl(
        mib.as_mut_ptr(),
        2,
        value,
        &mut len as *mut usize,
        std::ptr::null_mut(),
        0,
    ) == 0
}

unsafe fn get_sys_value_by_name(name: &[u8], len: &mut usize, value: *mut libc::c_void) -> bool {
    libc::sysctlbyname(
        name.as_ptr() as *const c_char,
        value,
        len,
        std::ptr::null_mut(),
        0,
    ) == 0
}

fn get_system_info(value: c_int, default: Option<&str>) -> Option<String> {
    let mut mib: [c_int; 2] = [libc::CTL_KERN, value];
    let mut size = 0;

    // Call first to get size
    unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            2,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };

    // exit early if we did not update the size
    if size == 0 {
        default.map(|s| s.to_owned())
    } else {
        // set the buffer to the correct size
        let mut buf = vec![0_u8; size as usize];

        if unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                2,
                buf.as_mut_ptr() as _,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        } == -1
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
