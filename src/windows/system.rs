// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

use sys::component::Component;
use sys::processor::*;
use sys::process::*;
use std::collections::HashMap;
use std::mem::{size_of, zeroed};

use libc::c_char;

use kernel32;
use winapi;
use winapi::minwindef::{DWORD, MAX_PATH};
use winapi::winnt::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use winapi::psapi::LIST_MODULES_ALL;
use winapi::sysinfoapi::{MEMORYSTATUSEX, SYSTEM_INFO};

pub struct System {
    process_list: HashMap<usize, Process>,
    mem_total: u64,
    mem_free: u64,
    swap_total: u64,
    swap_free: u64,
    processors: Vec<Processor>,
    temperatures: Vec<Component>,
}

fn init_processors() -> Vec<Processor> {
    unsafe {
        let mut sys_info: SYSTEM_INFO = zeroed();
        let mut ret = Vec::new();
        kernel32::GetSystemInfo(&mut sys_info);
        for nb in 0..sys_info.dwNumberOfProcessors {
            ret.push(create_processor(&format!("CPU {}", nb + 1)));
        }
        ret.insert(0, create_processor("Total CPU"));
        ret
    }
}

impl System {
    pub fn new() -> System {
        let mut s = System {
            process_list: HashMap::new(),
            mem_total: 0,
            mem_free: 0,
            swap_total: 0,
            swap_free: 0,
            processors: init_processors(),
            temperatures: Component::get_components(),
        };
        s.refresh_all();
        s
    }

    pub fn refresh_system(&mut self) {
        unsafe {
            let mut mem_info: MEMORYSTATUSEX = zeroed();
            mem_info.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
            kernel32::GlobalMemoryStatusEx(&mut mem_info);
            self.mem_total = auto_cast!(mem_info.ullTotalPhys, u64);
            self.mem_free = auto_cast!(mem_info.ullAvailPhys, u64);
            self.swap_total = auto_cast!(mem_info.ullTotalPageFile - mem_info.ullTotalPhys, u64);
            self.mem_free = self.mem_total - auto_cast!(mem_info.ullAvailPageFile, u64);
        }
    }

    pub fn refresh_process(&mut self) {
        let mut process_ids: [DWORD; 1024] = [0; 1024];
        let mut cb_needed = 0;

        unsafe {
            let size = ::std::mem::size_of::<DWORD>() * process_ids.len();
            if kernel32::K32EnumProcesses(process_ids.as_mut_ptr(),
                                          size as DWORD,
                                          &mut cb_needed) == 0 {
                return
            }
            let nb_processes = cb_needed / ::std::mem::size_of::<DWORD>() as DWORD;

            for i in 0..nb_processes as usize {
                let pid = process_ids[i];
                if pid == 0 {
                    continue
                }
                let options = PROCESS_QUERY_INFORMATION | PROCESS_VM_READ;
                let process_handler = kernel32::OpenProcess(options, winapi::FALSE, pid);
                if process_handler.is_null() {
                    continue
                }
                if let Some(ref mut entry) = self.process_list.get_mut(&(pid as usize)) {
                    update_proc_info(entry);
                    continue
                }
                let mut p = Process::new(process_handler, pid, 0); // TODO: should be start time, not 0
                let mut h_mod = ::std::ptr::null_mut();
                let mut process_name = [0 as u8; MAX_PATH];

                if kernel32::K32EnumProcessModulesEx(process_handler,
                                           &mut h_mod,
                                           ::std::mem::size_of::<DWORD>() as DWORD,
                                           &mut cb_needed,
                                           LIST_MODULES_ALL) != 0 {
                    kernel32::K32GetModuleBaseNameA(process_handler,
                                                    h_mod,
                                                    process_name.as_mut_ptr() as *mut c_char,
                                                    MAX_PATH as DWORD);
                }
                p.name = String::from_utf8_unchecked(process_name.to_vec());
                update_proc_info(&mut p);
                self.process_list.insert(pid as usize, p);
            }
        }
        self.clear_procs();
    }

    fn clear_procs(&mut self) {
        if self.processors.len() > 0 {
            let mut to_delete = Vec::new();

            for (pid, proc_) in self.process_list.iter_mut() {
                if has_been_updated(&proc_) == false {
                    to_delete.push(*pid);
                } else {
                    compute_cpu_usage(proc_, self.processors.len() as u64 - 1);
                }
            }
            for pid in to_delete {
                self.process_list.remove(&pid);
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

/*fn get_page_size() -> u64 {
    let mut system_info = unsafe { ::std::mem::zeroed() };
    unsafe { kernel32::GetSystemInfo(&mut system_info); }
    system_info.dwPageSize as u64
}*/
