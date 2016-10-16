// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use sys::ffi;
use sys::component::Component;
use sys::processor::*;
use sys::process::*;
use std::collections::HashMap;
use libc::{self, c_void, c_int, size_t, stat, c_char, sysconf, _SC_PAGESIZE};
use std::rc::Rc;
use sys::processor;

pub struct System {
    process_list: HashMap<usize, Process>,
    mem_total: u64,
    mem_free: u64,
    swap_total: u64,
    swap_free: u64,
    processors: Vec<Processor>,
    page_size_kb: u64,
    temperatures: Vec<Component>,
    connection: Option<ffi::io_connect_t>,
}

impl Drop for System {
    fn drop(&mut self) {
        if let Some(conn) = self.connection {
            unsafe { ffi::IOServiceClose(conn); }
        }
    }
}

// code from https://github.com/Chris911/iStats
fn get_io_service_connection() -> Option<ffi::io_connect_t> {
    let mut master_port: ffi::mach_port_t = 0;
    let mut iterator: ffi::io_iterator_t = 0;

    unsafe {
        ffi::IOMasterPort(ffi::MACH_PORT_NULL, &mut master_port);

        let matching_dictionary = ffi::IOServiceMatching(b"AppleSMC\0".as_ptr() as *const i8);
        let result = ffi::IOServiceGetMatchingServices(master_port, matching_dictionary,
                                                       &mut iterator);
        if result != ffi::KIO_RETURN_SUCCESS {
            //println!("Error: IOServiceGetMatchingServices() = {}", result);
            return None;
        }

        let device = ffi::IOIteratorNext(iterator);
        ffi::IOObjectRelease(iterator);
        if device == 0 {
            //println!("Error: no SMC found");
            return None;
        }

        let mut conn = 0;
        let result = ffi::IOServiceOpen(device, ffi::mach_task_self(), 0, &mut conn);
        ffi::IOObjectRelease(device);
        if result != ffi::KIO_RETURN_SUCCESS {
            //println!("Error: IOServiceOpen() = {}", result);
            return None;
        }

        Some(conn)
    }
}

unsafe fn strtoul(s: *mut c_char, size: c_int, base: c_int) -> u32 {
    let mut total = 0u32;

    for i in 0..size {
        if base == 16 {
            total += (*s.offset(i as isize) as u32) << ((size - 1 - i) as u32) * 8;
        } else {
            total += (*s.offset(i as isize) << (size - 1 - i) * 8) as u32;
        }
    }
    return total;
}

unsafe fn ultostr(s: *mut c_char, val: u32) {
    *s = 0;
    ffi::sprintf(s, b"%c%c%c%c\0".as_ptr() as *const i8, val >> 24, val >> 16, val >> 8, val);
}

unsafe fn perform_call(conn: ffi::io_connect_t, index: c_int, input_structure: *mut ffi::KeyData_t,
                       output_structure: *mut ffi::KeyData_t) -> i32 {
    let mut structure_output_size = ::std::mem::size_of::<ffi::KeyData_t>();

    ffi::IOConnectCallStructMethod(conn, index as u32,
                                   input_structure, ::std::mem::size_of::<ffi::KeyData_t>(),
                                   output_structure, &mut structure_output_size)
}

unsafe fn read_key(con: ffi::io_connect_t, key: *mut c_char) -> Result<ffi::Val_t, i32> {
    let mut input_structure: ffi::KeyData_t = ::std::mem::zeroed::<ffi::KeyData_t>();
    let mut output_structure: ffi::KeyData_t = ::std::mem::zeroed::<ffi::KeyData_t>();
    let mut val: ffi::Val_t = ::std::mem::zeroed::<ffi::Val_t>();

    input_structure.key = strtoul(key, 4, 16);
    input_structure.data8 = ffi::SMC_CMD_READ_KEYINFO;

    let result = perform_call(con, ffi::KERNEL_INDEX_SMC, &mut input_structure,
                              &mut output_structure);
    if result != ffi::KIO_RETURN_SUCCESS {
        return Err(result);
    }

    val.data_size = output_structure.key_info.data_size;
    ultostr(val.data_type.as_mut_ptr(), output_structure.key_info.data_type);
    input_structure.key_info.data_size = val.data_size;
    input_structure.data8 = ffi::SMC_CMD_READ_BYTES;

    let result = perform_call(con, ffi::KERNEL_INDEX_SMC, &mut input_structure,
                              &mut output_structure);
    if result != ffi::KIO_RETURN_SUCCESS {
        Err(result)
    } else {
        ffi::memcpy(val.bytes.as_mut_ptr() as *mut c_void,
                    output_structure.bytes.as_mut_ptr() as *mut c_void,
                    ::std::mem::size_of::<[u8; 32]>());
        Ok(val)
    }
}

unsafe fn get_temperature(con: ffi::io_connect_t, key: *mut c_char) -> f32 {
    if let Ok(val) = read_key(con, key) {
        if val.data_size > 0 &&
           ffi::strcmp(val.data_type.as_ptr(), b"sp78\0".as_ptr() as *const i8) == 0 {
            // convert fp78 value to temperature
            let x = (val.bytes[0] as i32 * 256 + val.bytes[1] as i32) >> 2;
            return x as f32 / 64f32;
        }
    }
    return 0f32;
}

unsafe fn get_unchecked_str(cp: *mut u8, start: *mut u8) -> String {
    let len = cp as usize - start as usize;
    let part = Vec::from_raw_parts(start, len, len);
    let tmp = String::from_utf8_unchecked(part.clone());
    ::std::mem::forget(part);
    tmp
}

impl System {
    pub fn new() -> System {
        let mut s = System {
            process_list: HashMap::new(),
            mem_total: 0,
            mem_free: 0,
            swap_total: 0,
            swap_free: 0,
            processors: Vec::new(),
            page_size_kb: unsafe { sysconf(_SC_PAGESIZE) as u64 / 1024 },
            temperatures: Vec::new(),
            connection: get_io_service_connection(),
        };
        s.refresh_all();
        s
    }

    pub fn refresh_system(&mut self) {
        unsafe fn get_sys_value(high: u32, low: u32, mut len: usize, value: *mut c_void) -> bool {
            let mut mib = [high as i32, low as i32];
            ffi::sysctl(mib.as_mut_ptr(), 2, value, &mut len as *mut usize,
                        ::std::ptr::null_mut(), 0) == 0
        }

        unsafe {
            // get system values
            if self.mem_total < 1 {
                get_sys_value(ffi::CTL_HW, ffi::HW_MEMSIZE, ::std::mem::size_of::<u64>(),
                              &mut self.mem_total as *mut u64 as *mut c_void);
                self.mem_total /= 1024;
            }
            let count: u32 = ffi::HOST_VM_INFO64_COUNT;
            let mut stat = ::std::mem::zeroed::<ffi::vm_statistics64>();
            if ffi::host_statistics64(ffi::mach_host_self(), ffi::HOST_VM_INFO64,
                                      &mut stat as *mut ffi::vm_statistics64 as *mut c_void,
                                      &count as *const u32) == ffi::KERN_SUCCESS {
                self.mem_free = (stat.free_count + stat.inactive_count
                    + stat.speculative_count) as u64 * self.page_size_kb;
            }

            if let Some(con) = self.connection {
                if self.temperatures.len() < 1 {
                    // getting CPU critical temperature
                    let mut v = vec!('T' as i8, 'C' as i8, '0' as i8, 'D' as i8, 0);
                    let tmp = get_temperature(con, v.as_mut_ptr());
                    let critical_temp = if tmp > 0f32 {
                        Some(tmp)
                    } else {
                        None
                    };
                    // getting CPU temperature
                    v[3] = 'P' as i8;
                    let temp = get_temperature(con, v.as_mut_ptr() as *mut i8);
                    if temp > 0f32 {
                        self.temperatures.push(Component::new("CPU".to_owned(),
                                                              None, critical_temp));
                    }
                    // getting GPU temperature
                    v[1] = 'G' as i8;
                    let temp = get_temperature(con, v.as_mut_ptr() as *mut i8);
                    if temp > 0f32 {
                        self.temperatures.push(Component::new("GPU".to_owned(),
                                                              None, critical_temp));
                    }
                    // getting battery temperature
                    v[1] = 'B' as i8;
                    v[3] = 'T' as i8;
                    let temp = get_temperature(con, v.as_mut_ptr() as *mut i8);
                    if temp > 0f32 {
                        self.temperatures.push(Component::new("Battery".to_owned(),
                                                              None, critical_temp));
                    }
                } else {
                    let mut v = vec!('T' as i8, 'C' as i8, '0' as i8, 'P' as i8, 0);
                    for comp in self.temperatures.iter_mut() {
                        match &*comp.label {
                            "CPU" => {
                                v[1] = 'C' as i8;
                                v[3] = 'P' as i8;
                            }
                            "GPU" => {
                                v[1] = 'G' as i8;
                                v[3] = 'P' as i8;
                            }
                            _ => {
                                v[1] = 'B' as i8;
                                v[3] = 'T' as i8;
                            }
                        };
                        let temp = get_temperature(con, v.as_mut_ptr() as *mut i8);
                        ::sys::component::update_component(comp, temp);
                    }
                }
            }

            // get processor values
            let mut num_cpu_u = 0u32;
            let mut cpu_info: *mut i32 = ::std::ptr::null_mut();
            let mut num_cpu_info = 0u32;

            if self.processors.len() == 0 {
                let mut num_cpu = 0;

                if get_sys_value(ffi::CTL_HW, ffi::HW_NCPU, ::std::mem::size_of::<u32>(),
                                 &mut num_cpu as *mut usize as *mut c_void) == false {
                    num_cpu = 1;
                }

                self.processors.push(
                    processor::create_proc("0".to_owned(),
                                           Rc::new(ProcessorData::new(::std::ptr::null_mut(), 0))));
                if ffi::host_processor_info(ffi::mach_host_self(), ffi::PROCESSOR_CPU_LOAD_INFO,
                                       &mut num_cpu_u as *mut u32,
                                       &mut cpu_info as *mut *mut i32,
                                       &mut num_cpu_info as *mut u32) == ffi::KERN_SUCCESS {
                    let proc_data = Rc::new(ProcessorData::new(cpu_info, num_cpu_info));
                    for i in 0..num_cpu {
                        let mut p = processor::create_proc(format!("{}", i + 1), proc_data.clone());
                        let in_use = *cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_USER as isize)
                            + *cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_SYSTEM as isize)
                            + *cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_NICE as isize);
                        let total = in_use + *cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_IDLE as isize);
                        processor::set_cpu_proc(&mut p, in_use as f32 / total as f32);
                        self.processors.push(p);
                    }
                }
            } else if ffi::host_processor_info(ffi::mach_host_self(), ffi::PROCESSOR_CPU_LOAD_INFO,
                                               &mut num_cpu_u as *mut u32,
                                               &mut cpu_info as *mut *mut i32,
                                               &mut num_cpu_info as *mut u32) == ffi::KERN_SUCCESS {
                let mut pourcent = 0f32;
                let proc_data = Rc::new(ProcessorData::new(cpu_info, num_cpu_info));
                for (i, proc_) in self.processors.iter_mut().skip(1).enumerate() {
                    let old_proc_data = &*processor::get_processor_data(proc_);
                    let in_use = (*cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_USER as isize)
                            - *old_proc_data.cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_USER as isize))
                        + (*cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_SYSTEM as isize)
                            - *old_proc_data.cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_SYSTEM as isize))
                        + (*cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_NICE as isize)
                            - *old_proc_data.cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_NICE as isize));
                    let total = in_use + (*cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_IDLE as isize)
                        - *old_proc_data.cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_IDLE as isize));
                    processor::update_proc(proc_, in_use as f32 / total as f32, proc_data.clone());
                    pourcent += proc_.get_cpu_usage();
                }
                if self.processors.len() > 1 {
                    let len = self.processors.len() - 1;
                    if let Some(p) = self.processors.get_mut(0) {
                        processor::set_cpu_usage(p, pourcent / len as f32);
                    }
                }
            }
        }
    }

    pub fn refresh_process(&mut self) {
        let count = unsafe { ffi::proc_listallpids(::std::ptr::null_mut(), 0) };
        if count < 1 {
            return;
        }
        let mut pids: Vec<libc::pid_t> = Vec::with_capacity(count as usize);
        unsafe { pids.set_len(count as usize); }
        let count = count * ::std::mem::size_of::<libc::pid_t>() as i32;
        let x = unsafe { ffi::proc_listallpids(pids.as_mut_ptr() as *mut c_void, count) };

        if x < 1 || x as usize > pids.len() {
            return;
        } else if pids.len() > x as usize {
            unsafe { pids.set_len(x as usize); }
        }

        let taskallinfo_size = ::std::mem::size_of::<ffi::proc_taskallinfo>() as i32;
        let taskinfo_size = ::std::mem::size_of::<ffi::proc_taskinfo>() as i32;
        let threadinfo_size = ::std::mem::size_of::<ffi::proc_threadinfo>() as i32;

        let mut mib: [c_int; 3] = [ffi::CTL_KERN, ffi::KERN_ARGMAX, 0];
        let mut argmax = 0;
        let mut size = ::std::mem::size_of::<c_int>();
        unsafe {
            while ffi::sysctl(mib.as_mut_ptr(), 2, (&mut argmax) as *mut i32 as *mut c_void,
                              &mut size, ::std::ptr::null_mut(), 0) == -1 {}
        }
        let mut proc_args = Vec::with_capacity(argmax as usize);

        for pid in pids {
            unsafe {
                let mut thread_info = ::std::mem::zeroed::<ffi::proc_threadinfo>();
                let mut user_time = 0;
                let mut system_time = 0;
                if ffi::proc_pidinfo(pid,
                                     ffi::PROC_PIDTHREADINFO,
                                     0,
                                     &mut thread_info as *mut ffi::proc_threadinfo as *mut c_void,
                                     threadinfo_size) != 0 {
                    user_time = thread_info.pth_user_time;
                    system_time = thread_info.pth_system_time;
                }
                if let Some(ref mut p) = self.process_list.get_mut(&(pid as usize)) {
                    let mut task_info = ::std::mem::zeroed::<ffi::proc_taskinfo>();
                    if ffi::proc_pidinfo(pid,
                                         ffi::PROC_PIDTASKINFO,
                                         0,
                                         &mut task_info as *mut ffi::proc_taskinfo as *mut c_void,
                                         taskinfo_size) != taskinfo_size {
                        continue
                    }
                    let task_time = user_time + system_time
                        + task_info.pti_total_user + task_info.pti_total_system;
                    let time = ffi::mach_absolute_time();
                    compute_cpu_usage(p, time, task_time);

                    p.memory = task_info.pti_resident_size / 1024;
                    continue
                }

                let mut task_info = ::std::mem::zeroed::<ffi::proc_taskallinfo>();
                if ffi::proc_pidinfo(pid,
                                     ffi::PROC_PIDTASKALLINFO,
                                     0,
                                     &mut task_info as *mut ffi::proc_taskallinfo as *mut c_void,
                                     taskallinfo_size as i32) != taskallinfo_size as i32 {
                    continue
                }

                let mut p = Process::new(pid as i64, task_info.pbsd.pbi_start_tvsec);
                p.memory = task_info.ptinfo.pti_resident_size / 1024;

                let ptr = proc_args.as_mut_slice().as_mut_ptr();
                mib[0] = ffi::CTL_KERN;
                mib[1] = ffi::KERN_PROCARGS2;
                mib[2] = pid as c_int;
                size = argmax as size_t;
                /*
                * /---------------\ 0x00000000
                * | ::::::::::::: |
                * |---------------| <-- Beginning of data returned by sysctl() is here.
                * | argc          |
                * |---------------|
                * | exec_path     |
                * |---------------|
                * | 0             |
                * |---------------|
                * | arg[0]        |
                * |---------------|
                * | 0             |
                * |---------------|
                * | arg[n]        |
                * |---------------|
                * | 0             |
                * |---------------|
                * | env[0]        |
                * |---------------|
                * | 0             |
                * |---------------|
                * | env[n]        |
                * |---------------|
                * | ::::::::::::: |
                * |---------------| <-- Top of stack.
                * :               :
                * :               :
                * \---------------/ 0xffffffff
                */
                if ffi::sysctl(mib.as_mut_ptr(), 3, ptr as *mut c_void,
                               &mut size, ::std::ptr::null_mut(), 0) != -1 {
                    let mut n_args: c_int = 0;
                    ffi::memcpy((&mut n_args) as *mut c_int as *mut c_void, ptr as *const c_void, ::std::mem::size_of::<c_int>());
                    let mut cp = ptr.offset(::std::mem::size_of::<c_int>() as isize);
                    let mut start = cp;
                    if cp < ptr.offset(size as isize) {
                        while cp < ptr.offset(size as isize) && *cp != 0 {
                            cp = cp.offset(1);
                        }
                        p.exe = get_unchecked_str(cp, start);
                        if let Some(l) = p.exe.split("/").last() {
                            p.name = l.to_owned();
                        }
                        while cp < ptr.offset(size as isize) && *cp == 0 {
                            cp = cp.offset(1);
                        }
                        start = cp;
                        let mut c = 0;
                        while c < n_args && cp < ptr.offset(size as isize) {
                            if *cp == 0 {
                                c += 1;
                                if c < n_args {
                                    *cp = ' ' as u8;
                                }
                            }
                            cp = cp.offset(1);
                        }
                        p.cmd = get_unchecked_str(cp.offset(-1), start);
                        start = cp;
                        while cp < ptr.offset(size as isize) {
                            if *cp == 0 {
                                if cp == start {
                                    break;
                                }
                                p.environ.push(get_unchecked_str(cp, start));
                                start = cp.offset(1);
                            }
                            cp = cp.offset(1);
                        }
                    }
                } else {
                    // we don't have enough priviledges to get access to these info
                    continue;
                }
                self.process_list.insert(pid as usize, p);
            }
        }
    }

    // COMMON PART
    //
    // Need to be moved into a "common" file to avoid duplication.

    pub fn refresh_all(&mut self) {
        self.refresh_system();
        self.refresh_process();
    }

    pub fn get_process_list<'a>(&'a self) -> &'a HashMap<usize, Process> {
        &self.process_list
    }

    /// Return the process corresponding to the given pid or None if no such process exists.
    pub fn get_process(&self, pid: i64) -> Option<&Process> {
        self.process_list.get(&(pid as usize))
    }

    /// Return a list of process starting with the given name.
    pub fn get_process_by_name(&self, name: &str) -> Vec<&Process> {
        let mut ret = vec!();
        for val in self.process_list.values() {
            if val.name.starts_with(name) {
                ret.push(val);
            }
        }
        ret
    }

    /// The first process in the array is the "main" process
    pub fn get_processor_list<'a>(&'a self) -> &'a [Processor] {
        &self.processors[..]
    }

    pub fn get_total_memory(&self) -> u64 {
        self.mem_total
    }

    pub fn get_free_memory(&self) -> u64 {
        self.mem_free
    }

    pub fn get_used_memory(&self) -> u64 {
        self.mem_total - self.mem_free
    }

    pub fn get_total_swap(&self) -> u64 {
        self.swap_total
    }

    pub fn get_free_swap(&self) -> u64 {
        self.swap_free
    }

    // need to be checked
    pub fn get_used_swap(&self) -> u64 {
        self.swap_total - self.swap_free
    }

    pub fn get_components_list<'a>(&'a self) -> &'a [Component] {
        &self.temperatures[..]
    }
}
