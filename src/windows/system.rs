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
use RefreshKind;
use SystemExt;

use windows::network::{self, NetworkData};
use windows::process::{
    compute_cpu_usage, get_handle, get_system_computation_time, update_proc_info, Process,
};
use windows::processor::CounterValue;
use windows::tools::*;

use ntapi::ntexapi::{
    NtQuerySystemInformation, SystemProcessInformation, SYSTEM_PROCESS_INFORMATION,
};
use winapi::shared::minwindef::{DWORD, FALSE};
use winapi::shared::ntdef::{PVOID, ULONG};
use winapi::shared::ntstatus::STATUS_INFO_LENGTH_MISMATCH;
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::minwinbase::STILL_ACTIVE;
use winapi::um::pdh::PdhEnumObjectItemsW;
use winapi::um::processthreadsapi::GetExitCodeProcess;
use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use winapi::um::winnt::HANDLE;

use rayon::prelude::*;

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

// Useful for parallel iterations.
struct Wrap<T>(T);

unsafe impl<T> Send for Wrap<T> {}
unsafe impl<T> Sync for Wrap<T> {}

impl SystemExt for System {
    #[allow(non_snake_case)]
    fn new_with_specifics(refreshes: RefreshKind) -> System {
        let mut s = System {
            process_list: HashMap::with_capacity(500),
            mem_total: 0,
            mem_free: 0,
            swap_total: 0,
            swap_free: 0,
            processors: init_processors(),
            temperatures: component::get_components(),
            disks: Vec::with_capacity(2),
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
                    add_counter(
                        format!("\\{}(_Total)\\{}", processor_trans, proc_time_trans),
                        query,
                        get_key_used(&mut s.processors[0]),
                        "tot_0".to_owned(),
                        CounterValue::Float(0.),
                    );
                }
                if let Some(ref idle_time_trans) = idle_time_trans {
                    add_counter(
                        format!("\\{}(_Total)\\{}", processor_trans, idle_time_trans),
                        query,
                        get_key_idle(&mut s.processors[0]),
                        "tot_1".to_owned(),
                        CounterValue::Float(0.),
                    );
                }
                for (pos, proc_) in s.processors.iter_mut().skip(1).enumerate() {
                    if let Some(ref proc_time_trans) = proc_time_trans {
                        add_counter(
                            format!("\\{}({})\\{}", processor_trans, pos, proc_time_trans),
                            query,
                            get_key_used(proc_),
                            format!("{}_0", pos),
                            CounterValue::Float(0.),
                        );
                    }
                    if let Some(ref idle_time_trans) = idle_time_trans {
                        add_counter(
                            format!("\\{}({})\\{}", processor_trans, pos, idle_time_trans),
                            query,
                            get_key_idle(proc_),
                            format!("{}_1", pos),
                            CounterValue::Float(0.),
                        );
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
                    PdhEnumObjectItemsW(
                        ::std::ptr::null(),
                        ::std::ptr::null(),
                        network_trans_utf16.as_ptr(),
                        ::std::ptr::null_mut(),
                        &mut dwCounterListSize,
                        ::std::ptr::null_mut(),
                        &mut dwInstanceListSize,
                        PERF_DETAIL_WIZARD,
                        0,
                    )
                };
                if status != PDH_MORE_DATA as i32 {
                    eprintln!("PdhEnumObjectItems invalid status: {:x}", status);
                } else {
                    let mut pwsCounterListBuffer: Vec<u16> =
                        Vec::with_capacity(dwCounterListSize as usize);
                    let mut pwsInstanceListBuffer: Vec<u16> =
                        Vec::with_capacity(dwInstanceListSize as usize);
                    unsafe {
                        pwsCounterListBuffer.set_len(dwCounterListSize as usize);
                        pwsInstanceListBuffer.set_len(dwInstanceListSize as usize);
                    }
                    let status = unsafe {
                        PdhEnumObjectItemsW(
                            ::std::ptr::null(),
                            ::std::ptr::null(),
                            network_trans_utf16.as_ptr(),
                            pwsCounterListBuffer.as_mut_ptr(),
                            &mut dwCounterListSize,
                            pwsInstanceListBuffer.as_mut_ptr(),
                            &mut dwInstanceListSize,
                            PERF_DETAIL_WIZARD,
                            0,
                        )
                    };
                    if status != ERROR_SUCCESS as i32 {
                        eprintln!("PdhEnumObjectItems invalid status: {:x}", status);
                    } else {
                        for (pos, x) in pwsInstanceListBuffer
                            .split(|x| *x == 0)
                            .filter(|x| x.len() > 0)
                            .enumerate()
                        {
                            let net_interface = String::from_utf16(x).expect("invalid utf16");
                            if let Some(ref network_in_trans) = network_in_trans {
                                let mut key_in = None;
                                add_counter(
                                    format!(
                                        "\\{}({})\\{}",
                                        network_trans, net_interface, network_in_trans
                                    ),
                                    query,
                                    &mut key_in,
                                    format!("net{}_in", pos),
                                    CounterValue::Integer(0),
                                );
                                if key_in.is_some() {
                                    network::get_keys_in(&mut s.network).push(key_in.unwrap());
                                }
                            }
                            if let Some(ref network_out_trans) = network_out_trans {
                                let mut key_out = None;
                                add_counter(
                                    format!(
                                        "\\{}({})\\{}",
                                        network_trans, net_interface, network_out_trans
                                    ),
                                    query,
                                    &mut key_out,
                                    format!("net{}_out", pos),
                                    CounterValue::Integer(0),
                                );
                                if key_out.is_some() {
                                    network::get_keys_out(&mut s.network).push(key_out.unwrap());
                                }
                            }
                        }
                    }
                }
            }
            query.start();
        }
        s.refresh_specifics(refreshes);
        s
    }

    fn refresh_cpu(&mut self) {
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

    fn refresh_memory(&mut self) {
        self.uptime = get_uptime();
        unsafe {
            let mut mem_info: MEMORYSTATUSEX = zeroed();
            mem_info.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
            GlobalMemoryStatusEx(&mut mem_info);
            self.mem_total = auto_cast!(mem_info.ullTotalPhys, u64);
            self.mem_free = auto_cast!(mem_info.ullAvailPhys, u64);
            //self.swap_total = auto_cast!(mem_info.ullTotalPageFile - mem_info.ullTotalPhys, u64);
            //self.swap_free = auto_cast!(mem_info.ullAvailPageFile, u64);
        }
    }

    fn refresh_temperatures(&mut self) {
        // does nothing for the moment...
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
        // Windows 10 notebook requires at least 512kb of memory to make it in one go
        let mut buffer_size: usize = 512 * 1024;

        loop {
            let mut process_information: Vec<u8> = Vec::with_capacity(buffer_size);

            let mut cb_needed = 0;
            let ntstatus = unsafe {
                process_information.set_len(buffer_size);
                NtQuerySystemInformation(
                    SystemProcessInformation,
                    process_information.as_mut_ptr() as PVOID,
                    buffer_size as ULONG,
                    &mut cb_needed,
                )
            };

            if ntstatus != STATUS_INFO_LENGTH_MISMATCH {
                if ntstatus < 0 {
                    eprintln!(
                        "Couldn't get process infos: NtQuerySystemInformation returned {}",
                        ntstatus
                    );
                }

                // Parse the data block to get process information
                let mut process_ids = Vec::with_capacity(500);
                let mut process_information_offset = 0;
                loop {
                    let p = unsafe {
                        process_information
                            .as_ptr()
                            .offset(process_information_offset)
                            as *const SYSTEM_PROCESS_INFORMATION
                    };
                    let pi = unsafe { &*p };

                    process_ids.push(Wrap(p));

                    if pi.NextEntryOffset == 0 {
                        break;
                    }

                    process_information_offset += pi.NextEntryOffset as isize;
                }
                let nb_processors = self.processors.len() as u64;
                let process_list = Wrap(UnsafeCell::new(&mut self.process_list));
                let system_time = get_system_computation_time();
                // TODO: instead of using parallel iterator only here, would be better to be able
                //       to run it over `process_information` directly!
                let processes = process_ids
                    .into_par_iter()
                    .filter_map(|pi| unsafe {
                        let pi = *pi.0;
                        let pid = pi.UniqueProcessId as usize;
                        if let Some(proc_) = (*process_list.0.get()).get_mut(&pid) {
                            proc_.memory = (pi.WorkingSetSize as u64) >> 10u64;
                            proc_.virtual_memory = (pi.VirtualSize as u64) >> 10u64;
                            compute_cpu_usage(proc_, nb_processors, system_time);
                            proc_.updated = true;
                            return None;
                        }
                        let name = get_process_name(&pi, pid);
                        let mut p = Process::new_full(
                            pid,
                            if pi.InheritedFromUniqueProcessId as usize != 0 {
                                Some(pi.InheritedFromUniqueProcessId as usize)
                            } else {
                                None
                            },
                            (pi.WorkingSetSize as u64) >> 10u64,
                            (pi.VirtualSize as u64) >> 10u64,
                            name,
                        );
                        compute_cpu_usage(&mut p, nb_processors, system_time);
                        Some(p)
                    })
                    .collect::<Vec<_>>();
                self.process_list.retain(|_, v| {
                    let x = v.updated;
                    v.updated = false;
                    x
                });
                for p in processes.into_iter() {
                    self.process_list.insert(p.pid(), p);
                }

                break;
            }

            // GetNewBufferSize
            if cb_needed == 0 {
                buffer_size *= 2;
                continue;
            }
            // allocating a few more kilo bytes just in case there are some new process
            // kicked in since new call to NtQuerySystemInformation
            buffer_size = (cb_needed + (1024 * 10)) as usize;
        }
    }

    fn refresh_disks(&mut self) {
        self.disks.par_iter_mut().for_each(|disk| {
            disk.update();
        });
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
            compute_cpu_usage(
                entry,
                s.processors.len() as u64,
                get_system_computation_time(),
            );
        }
        true
    } else {
        false
    }
}

fn get_process_name(process: &SYSTEM_PROCESS_INFORMATION, process_id: usize) -> String {
    let name = &process.ImageName;
    if name.Buffer.is_null() {
        match process_id {
            0 => "Idle".to_owned(),
            4 => "System".to_owned(),
            _ => format!("<no name> Process {}", process_id),
        }
    } else {
        let slice = unsafe {
            std::slice::from_raw_parts(
                name.Buffer,
                //The length is in bytes, not the length of string
                name.Length as usize / std::mem::size_of::<u16>(),
            )
        };

        String::from_utf16_lossy(slice)
    }
}
