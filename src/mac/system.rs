//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use sys::component::Component;
use sys::disk::Disk;
use sys::ffi;
use sys::network::Networks;
use sys::process::*;
use sys::processor::*;

use {LoadAvg, Pid, ProcessExt, ProcessorExt, RefreshKind, SystemExt, User};

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::mem;
use std::sync::Arc;

use libc::{self, c_int, c_void, size_t, sysconf, _SC_PAGESIZE};

use rayon::prelude::*;

/// Structs containing system's information.
pub struct System {
    process_list: HashMap<Pid, Process>,
    mem_total: u64,
    mem_free: u64,
    swap_total: u64,
    swap_free: u64,
    global_processor: Processor,
    processors: Vec<Processor>,
    page_size_kb: u64,
    components: Vec<Component>,
    connection: Option<ffi::io_connect_t>,
    disks: Vec<Disk>,
    networks: Networks,
    port: ffi::mach_port_t,
    users: Vec<User>,
    boot_time: u64,
}

impl Drop for System {
    fn drop(&mut self) {
        if let Some(conn) = self.connection {
            unsafe {
                ffi::IOServiceClose(conn);
            }
        }
    }
}

pub(crate) struct Wrap<'a>(pub UnsafeCell<&'a mut HashMap<Pid, Process>>);

unsafe impl<'a> Send for Wrap<'a> {}
unsafe impl<'a> Sync for Wrap<'a> {}

impl System {
    fn clear_procs(&mut self) {
        let mut to_delete = Vec::new();

        for (pid, mut proc_) in &mut self.process_list {
            if !has_been_updated(&mut proc_) {
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
    let mut len = ::std::mem::size_of::<libc::timeval>();
    let mut mib: [libc::c_int; 2] = [libc::CTL_KERN, libc::KERN_BOOTTIME];
    if unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            2,
            &mut boot_time as *mut libc::timeval as *mut _,
            &mut len,
            ::std::ptr::null_mut(),
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
            swap_total: 0,
            swap_free: 0,
            global_processor,
            processors,
            page_size_kb: unsafe { sysconf(_SC_PAGESIZE) as u64 / 1_000 },
            components: Vec::with_capacity(2),
            connection: get_io_service_connection(),
            disks: Vec::with_capacity(1),
            networks: Networks::new(),
            port,
            users: Vec::new(),
            boot_time: boot_time(),
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
                ffi::CTL_VM,
                ffi::VM_SWAPUSAGE,
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
                    ffi::CTL_HW,
                    ffi::HW_MEMSIZE,
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
                // self.mem_free = u64::from(stat.free_count) * self.page_size_kb;
                self.mem_free = self.mem_total
                    - (u64::from(stat.active_count)
                        + u64::from(stat.inactive_count)
                        + u64::from(stat.wire_count)
                        + u64::from(stat.speculative_count)
                        - u64::from(stat.purgeable_count))
                        * self.page_size_kb;
            }
        }
    }

    fn refresh_components_list(&mut self) {
        if let Some(con) = self.connection {
            self.components.clear();
            // getting CPU critical temperature
            let critical_temp = crate::mac::component::get_temperature(
                con,
                &['T' as i8, 'C' as i8, '0' as i8, 'D' as i8, 0],
            );

            for (id, v) in crate::mac::component::COMPONENTS_TEMPERATURE_IDS.iter() {
                if let Some(c) = Component::new((*id).to_owned(), None, critical_temp, v, con) {
                    self.components.push(c);
                }
            }
        }
    }

    fn refresh_cpu(&mut self) {
        // get processor values
        let mut num_cpu_u = 0u32;
        let mut cpu_info: *mut i32 = ::std::ptr::null_mut();
        let mut num_cpu_info = 0u32;

        let mut pourcent = 0f32;

        unsafe {
            if ffi::host_processor_info(
                self.port,
                ffi::PROCESSOR_CPU_LOAD_INFO,
                &mut num_cpu_u as *mut u32,
                &mut cpu_info as *mut *mut i32,
                &mut num_cpu_info as *mut u32,
            ) == ffi::KERN_SUCCESS
            {
                let proc_data = Arc::new(ProcessorData::new(cpu_info, num_cpu_info));
                for (i, proc_) in self.processors.iter_mut().enumerate() {
                    let old_proc_data = &*proc_.get_data();
                    let in_use =
                        (*cpu_info.offset(
                            (ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_USER as isize,
                        ) - *old_proc_data.cpu_info.offset(
                            (ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_USER as isize,
                        )) + (*cpu_info.offset(
                            (ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_SYSTEM as isize,
                        ) - *old_proc_data.cpu_info.offset(
                            (ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_SYSTEM as isize,
                        )) + (*cpu_info.offset(
                            (ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_NICE as isize,
                        ) - *old_proc_data.cpu_info.offset(
                            (ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_NICE as isize,
                        ));
                    let total = in_use
                        + (*cpu_info.offset(
                            (ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_IDLE as isize,
                        ) - *old_proc_data.cpu_info.offset(
                            (ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_IDLE as isize,
                        ));
                    proc_.update(in_use as f32 / total as f32 * 100., Arc::clone(&proc_data));
                    pourcent += proc_.get_cpu_usage();
                }
            }
        }
        self.global_processor
            .set_cpu_usage(pourcent / self.processors.len() as f32);
    }

    fn refresh_processes(&mut self) {
        let count = unsafe { ffi::proc_listallpids(::std::ptr::null_mut(), 0) };
        if count < 1 {
            return;
        }
        if let Some(pids) = get_proc_list() {
            let arg_max = get_arg_max();
            let entries: Vec<Process> = {
                let wrap = &Wrap(UnsafeCell::new(&mut self.process_list));
                pids.par_iter()
                    .flat_map(|pid| match update_process(wrap, *pid, arg_max as size_t) {
                        Ok(x) => x,
                        Err(_) => None,
                    })
                    .collect()
            };
            entries.into_iter().for_each(|entry| {
                self.process_list.insert(entry.pid(), entry);
            });
            self.clear_procs();
        }
    }

    fn refresh_process(&mut self, pid: Pid) -> bool {
        let arg_max = get_arg_max();
        match {
            let wrap = Wrap(UnsafeCell::new(&mut self.process_list));
            update_process(&wrap, pid, arg_max as size_t)
        } {
            Ok(Some(p)) => {
                self.process_list.insert(p.pid(), p);
                true
            }
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn refresh_disks_list(&mut self) {
        self.disks = crate::mac::disk::get_disks();
    }

    fn refresh_users_list(&mut self) {
        self.users = crate::mac::users::get_users_list();
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
        let loads = vec![0f64; 3];
        unsafe {
            ffi::getloadavg(loads.as_ptr() as *const f64, 3);
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
}

impl Default for System {
    fn default() -> System {
        System::new()
    }
}

// code from https://github.com/Chris911/iStats
fn get_io_service_connection() -> Option<ffi::io_connect_t> {
    let mut master_port: ffi::mach_port_t = 0;
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
            ::std::ptr::null_mut(),
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
        ::std::ptr::null_mut(),
        0,
    ) == 0
}
