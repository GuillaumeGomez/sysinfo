//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

use sys::component::{self, Component};
use sys::disk::Disk;
use sys::processor::*;

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::mem::{size_of, zeroed};

use DiskExt;
use Pid;
use ProcessExt;
use SystemExt;

use windows::network::{self, NetworkData};
use windows::processor::CounterValue;
use windows::tools::*;
use windows::process::{
    Process, compute_cpu_usage, get_handle, get_parent_process_id, update_proc_info,
};

use winapi::shared::minwindef::{DWORD, FALSE};
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::minwinbase::STILL_ACTIVE;
use winapi::um::pdh::PdhEnumObjectItemsW;
use winapi::um::processthreadsapi::GetExitCodeProcess;
use winapi::um::psapi::K32EnumProcesses;
use winapi::um::sysinfoapi::{
    GlobalMemoryStatusEx, MEMORYSTATUSEX,
};
use winapi::um::winnt::HANDLE;

use rayon::prelude::*;

const PROCESS_LEN: usize = 10192;

/// Struct containing system's information.
pub struct System {
    process_list: HashMap<usize, Process>,
    mem_total: u64,
    mem_free: u64,
    swap_total: u64,
    swap_free: u64,
    processors: Vec<Processor>,
    temperatures: Vec<Component>,
    disks: Vec<Disk>,
    query: Option<Query>,
    network: NetworkData,
    uptime: u64,
}

impl System {
    fn clear_procs(&mut self) {
        if self.processors.len() > 0 {
            let mut to_delete = Vec::new();

            for (pid, proc_) in self.process_list.iter_mut() {
                if !is_proc_running(get_handle(proc_)) {
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
}

struct Wrap(Process);

unsafe impl Send for Wrap {}

struct WrapSystem<'a>(UnsafeCell<&'a mut System>);
 impl<'a> WrapSystem<'a> {
    fn get(&self) -> &'a mut System {
        unsafe { *(self.0.get()) }
    }
}
 unsafe impl<'a> Send for WrapSystem<'a> {}
unsafe impl<'a> Sync for WrapSystem<'a> {}

impl SystemExt for System {
    #[allow(non_snake_case)]
    fn new() -> System {
        let mut s = System {
            process_list: HashMap::new(),
            mem_total: 0,
            mem_free: 0,
            swap_total: 0,
            swap_free: 0,
            processors: init_processors(),
            temperatures: component::get_components(),
            disks: unsafe { get_disks() },
            query: Query::new(),
            network: network::new(),
            uptime: get_uptime(),
        };
        // TODO: in case a translation fails, it might be nice to log it somewhere...
        if let Some(ref mut query) = s.query {
            let x = unsafe { load_symbols() };
            if let Some(processor_trans) = get_translation(&"Processor".to_owned(), &x) {
                let idle_time_trans = get_translation(&"% Idle Time".to_owned(), &x);
                let proc_time_trans = get_translation(&"% Processor Time".to_owned(), &x);
                if let Some(ref proc_time_trans) = proc_time_trans {
                    add_counter(format!("\\{}(_Total)\\{}", processor_trans, proc_time_trans),
                                query,
                                get_key_used(&mut s.processors[0]),
                                "tot_0".to_owned(),
                                CounterValue::Float(0.));
                }
                if let Some(ref idle_time_trans) = idle_time_trans {
                    add_counter(format!("\\{}(_Total)\\{}", processor_trans, idle_time_trans),
                                query,
                                get_key_idle(&mut s.processors[0]),
                                "tot_1".to_owned(),
                                CounterValue::Float(0.));
                }
                for (pos, proc_) in s.processors.iter_mut().skip(1).enumerate() {
                    if let Some(ref proc_time_trans) = proc_time_trans {
                        add_counter(format!("\\{}({})\\{}", processor_trans, pos, proc_time_trans),
                                    query,
                                    get_key_used(proc_),
                                    format!("{}_0", pos),
                                    CounterValue::Float(0.));
                    }
                    if let Some(ref idle_time_trans) = idle_time_trans {
                        add_counter(format!("\\{}({})\\{}", processor_trans, pos, idle_time_trans),
                                    query,
                                    get_key_idle(proc_),
                                    format!("{}_1", pos),
                                    CounterValue::Float(0.));
                    }
                }
            }

            if let Some(network_trans) = get_translation(&"Network Interface".to_owned(), &x) {
                let network_in_trans = get_translation(&"Bytes Received/Sec".to_owned(), &x);
                let network_out_trans = get_translation(&"Bytes Sent/sec".to_owned(), &x);

                const PERF_DETAIL_WIZARD: DWORD = 400;
                const PDH_MORE_DATA: DWORD = 0x800007D2;

                let mut network_trans_utf16: Vec<u16> = network_trans.encode_utf16().collect();
                network_trans_utf16.push(0);
                let mut dwCounterListSize: DWORD = 0;
                let mut dwInstanceListSize: DWORD = 0;
                let status = unsafe {
                    PdhEnumObjectItemsW(::std::ptr::null(),
                                        ::std::ptr::null(),
                                        network_trans_utf16.as_ptr(),
                                        ::std::ptr::null_mut(),
                                        &mut dwCounterListSize,
                                        ::std::ptr::null_mut(),
                                        &mut dwInstanceListSize,
                                        PERF_DETAIL_WIZARD,
                                        0)
                };
                if status != PDH_MORE_DATA as i32 {
                    panic!("got invalid status: {:x}", status);
                }
                let mut pwsCounterListBuffer: Vec<u16> = Vec::with_capacity(dwCounterListSize as usize);
                let mut pwsInstanceListBuffer: Vec<u16> = Vec::with_capacity(dwInstanceListSize as usize);
                unsafe {
                    pwsCounterListBuffer.set_len(dwCounterListSize as usize);
                    pwsInstanceListBuffer.set_len(dwInstanceListSize as usize);
                }
                let status = unsafe {
                    PdhEnumObjectItemsW(::std::ptr::null(),
                                        ::std::ptr::null(),
                                        network_trans_utf16.as_ptr(),
                                        pwsCounterListBuffer.as_mut_ptr(),
                                        &mut dwCounterListSize,
                                        pwsInstanceListBuffer.as_mut_ptr(),
                                        &mut dwInstanceListSize,
                                        PERF_DETAIL_WIZARD,
                                        0)
                };
                if status != ERROR_SUCCESS as i32 {
                    panic!("got invalid status: {:x}", status);
                }

                for (pos, x) in pwsInstanceListBuffer.split(|x| *x == 0)
                                                     .filter(|x| x.len() > 0)
                                                     .enumerate() {
                    let net_interface = String::from_utf16(x).expect("invalid utf16");
                    if let Some(ref network_in_trans) = network_in_trans {
                        let mut key_in = None;
                        add_counter(format!("\\{}({})\\{}",
                                            network_trans, net_interface, network_in_trans),
                                    query,
                                    &mut key_in,
                                    format!("net{}_in", pos),
                                    CounterValue::Integer(0));
                        if key_in.is_some() {
                            network::get_keys_in(&mut s.network).push(key_in.unwrap());
                        }
                    }
                    if let Some(ref network_out_trans) = network_out_trans {
                        let mut key_out = None;
                        add_counter(format!("\\{}({})\\{}",
                                            network_trans, net_interface, network_out_trans),
                                    query,
                                    &mut key_out,
                                    format!("net{}_out", pos),
                                    CounterValue::Integer(0));
                        if key_out.is_some() {
                            network::get_keys_out(&mut s.network).push(key_out.unwrap());
                        }
                    }
                }
            }
            query.start();
        }
        s.refresh_all();
        s
    }

    fn refresh_system(&mut self) {
        unsafe {
            let mut mem_info: MEMORYSTATUSEX = zeroed();
            mem_info.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
            GlobalMemoryStatusEx(&mut mem_info);
            self.mem_total = auto_cast!(mem_info.ullTotalPhys, u64);
            self.mem_free = auto_cast!(mem_info.ullAvailPhys, u64);
            //self.swap_total = auto_cast!(mem_info.ullTotalPageFile - mem_info.ullTotalPhys, u64);
            //self.swap_free = auto_cast!(mem_info.ullAvailPageFile, u64);
        }
        self.uptime = get_uptime();
        if let Some(ref mut query) = self.query {
            for p in self.processors.iter_mut() {
                let mut idle_time = None;
                if let &mut Some(ref key_idle) = get_key_idle(p) {
                    idle_time = Some(query.get(&key_idle.unique_id).expect("key disappeared"));
                }
                if let Some(idle_time) = idle_time {
                    set_cpu_usage(p, 1. - idle_time);
                }
            }
        }
    }

    fn refresh_network(&mut self) {
        network::refresh(&mut self.network, &self.query);
    }

    fn refresh_process(&mut self, pid: Pid) -> bool {
        if refresh_existing_process(self, pid, true) == false {
            self.process_list.remove(&pid);
            false
        } else {
            true
        }
    }

    fn refresh_processes(&mut self) {
        // I think that 10192 as length will be enough to get all processes at once...
        let mut process_ids: Vec<DWORD> = Vec::with_capacity(PROCESS_LEN);
        let mut cb_needed = 0;

        unsafe { process_ids.set_len(PROCESS_LEN); }
        let size = ::std::mem::size_of::<DWORD>() * process_ids.len();
        unsafe {
            if K32EnumProcesses(process_ids.as_mut_ptr(),
                                size as DWORD,
                                &mut cb_needed) == 0 {
                return
            }
        }
        let nb_processes = cb_needed / ::std::mem::size_of::<DWORD>() as DWORD;
        unsafe { process_ids.set_len(nb_processes as usize); }

        {
            let this = WrapSystem(UnsafeCell::new(self));

            process_ids.par_iter()
                       .filter_map(|pid| {
                           let pid = *pid as usize;
                           if !refresh_existing_process(this.get(), pid, false) {
                               let ppid = unsafe { get_parent_process_id(pid) };
                               let mut p = Process::new(pid, ppid, 0);
                               update_proc_info(&mut p);
                               Some(Wrap(p))
                           } else {
                               None
                           }
                       })
                       .collect::<Vec<_>>()
        }.into_iter()
         .for_each(|p| {
             self.process_list.insert(p.0.pid(), p.0);
         });
        self.clear_procs();
    }

    fn refresh_disks(&mut self) {
        for disk in &mut self.disks {
            disk.update();
        }
    }

    fn refresh_disk_list(&mut self) {
        self.disks = unsafe { get_disks() };
    }

    fn get_process_list(&self) -> &HashMap<Pid, Process> {
        &self.process_list
    }

    fn get_process(&self, pid: Pid) -> Option<&Process> {
        self.process_list.get(&(pid as usize))
    }

    fn get_processor_list(&self) -> &[Processor] {
        &self.processors[..]
    }

    fn get_total_memory(&self) -> u64 {
        self.mem_total >> 10
    }

    fn get_free_memory(&self) -> u64 {
        self.mem_free >> 10
    }

    fn get_used_memory(&self) -> u64 {
        (self.mem_total - self.mem_free) >> 10
    }

    fn get_total_swap(&self) -> u64 {
        self.swap_total >> 10
    }

    fn get_free_swap(&self) -> u64 {
        self.swap_free >> 10
    }

    fn get_used_swap(&self) -> u64 {
        (self.swap_total - self.swap_free) >> 10
    }

    fn get_components_list(&self) -> &[Component] {
        &self.temperatures[..]
    }

    fn get_disks(&self) -> &[Disk] {
        &self.disks[..]
    }

    fn get_network(&self) -> &NetworkData {
        &self.network
    }

    fn get_uptime(&self) -> u64 {
        self.uptime
    }
}

fn is_proc_running(handle: HANDLE) -> bool {
    let mut exit_code = 0;
    let ret = unsafe { GetExitCodeProcess(handle, &mut exit_code) };
    !(ret == FALSE || exit_code != STILL_ACTIVE)
}

fn refresh_existing_process(s: &mut System, pid: Pid, compute_cpu: bool) -> bool {
    if let Some(ref mut entry) = s.process_list.get_mut(&(pid as usize)) {
        if !is_proc_running(get_handle(entry)) {
            return false;
        }
        update_proc_info(entry);
        if compute_cpu {
            compute_cpu_usage(entry, s.processors.len() as u64 - 1);
        }
        true
    } else {
        false
    }
}
