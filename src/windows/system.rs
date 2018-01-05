//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use sys::component::{self, Component};
use sys::disk::{new_disk, Disk, DiskType};
use sys::processor::*;
use sys::process::*;

use std::collections::HashMap;
use std::mem::{size_of, zeroed};
use std::str;

use winapi::ctypes::c_void;

use Pid;
use ProcessExt;
use SystemExt;

use windows::tools::KeyHandler;
use windows::network::{self, NetworkData};
use windows::processor::CounterValue;

use winapi::um::minwinbase::STILL_ACTIVE;
use winapi::shared::minwindef::{BYTE, DWORD, FALSE, MAX_PATH, TRUE};
use winapi::um::fileapi::{
    CreateFileA, GetDriveTypeA, GetLogicalDrives, GetVolumeInformationA, OPEN_EXISTING,
};
use winapi::um::handleapi::CloseHandle;
use winapi::um::ioapiset::DeviceIoControl;
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::pdh::PdhEnumObjectItemsW;
use winapi::um::processthreadsapi::GetExitCodeProcess;
use winapi::um::psapi::K32EnumProcesses;
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::sysinfoapi::{
    GetSystemInfo, GlobalMemoryStatusEx, MEMORYSTATUSEX, SYSTEM_INFO,
};
use winapi::um::winioctl::{
    DISK_GEOMETRY, IOCTL_STORAGE_QUERY_PROPERTY, IOCTL_DISK_GET_DRIVE_GEOMETRY,
};
use winapi::um::winnt::{
    BOOLEAN, FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE, HANDLE,
};
use winapi::um::winbase::DRIVE_FIXED;

/*#[allow(non_snake_case)]
#[allow(unused)]
unsafe fn browser() {
    use winapi::um::pdh::{PdhBrowseCountersA, PDH_BROWSE_DLG_CONFIG_A};
    use winapi::shared::winerror::ERROR_SUCCESS;

    let mut BrowseDlgData: PDH_BROWSE_DLG_CONFIG_A = ::std::mem::zeroed();
    let mut CounterPathBuffer: [i8; 255] = ::std::mem::zeroed();
    const PERF_DETAIL_WIZARD: u32 = 400;
    let text = b"Select a counter to monitor.\0";

    BrowseDlgData.set_IncludeInstanceIndex(FALSE as u32);
    BrowseDlgData.set_SingleCounterPerAdd(TRUE as u32);
    BrowseDlgData.set_SingleCounterPerDialog(TRUE as u32);
    BrowseDlgData.set_LocalCountersOnly(FALSE as u32);
    BrowseDlgData.set_WildCardInstances(TRUE as u32);
    BrowseDlgData.set_HideDetailBox(TRUE as u32);
    BrowseDlgData.set_InitializePath(FALSE as u32);
    BrowseDlgData.set_DisableMachineSelection(FALSE as u32);
    BrowseDlgData.set_IncludeCostlyObjects(FALSE as u32);
    BrowseDlgData.set_ShowObjectBrowser(FALSE as u32);
    BrowseDlgData.hWndOwner = ::std::ptr::null_mut();
    BrowseDlgData.szReturnPathBuffer = CounterPathBuffer.as_mut_ptr();
    BrowseDlgData.cchReturnPathLength = 255;
    BrowseDlgData.pCallBack = None;
    BrowseDlgData.dwCallBackArg = 0;
    BrowseDlgData.CallBackStatus = ERROR_SUCCESS as i32;
    BrowseDlgData.dwDefaultDetailLevel = PERF_DETAIL_WIZARD;
    BrowseDlgData.szDialogBoxCaption = text as *const _ as usize as *mut i8;
    let ret = PdhBrowseCountersA(&mut BrowseDlgData as *mut _);
    println!("browser: {:?}", ret);
    for x in CounterPathBuffer.iter() {
        print!("{:?} ", *x);
    }
    println!("");
    for x in 0..256 {
        print!("{:?} ", *BrowseDlgData.szReturnPathBuffer.offset(x));
    }
    println!("");
}*/

fn init_processors() -> Vec<Processor> {
    unsafe {
        let mut sys_info: SYSTEM_INFO = zeroed();
        GetSystemInfo(&mut sys_info);
        let mut ret = Vec::with_capacity(sys_info.dwNumberOfProcessors as usize + 1);
        for nb in 0..sys_info.dwNumberOfProcessors {
            ret.push(create_processor(&format!("CPU {}", nb + 1)));
        }
        ret.insert(0, create_processor("Total CPU"));
        ret
    }
}

unsafe fn open_drive(drive_name: &[u8], open_rights: DWORD) -> HANDLE {
    CreateFileA(drive_name.as_ptr() as *const i8,
                open_rights,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ::std::ptr::null_mut(), OPEN_EXISTING,
                0,
                ::std::ptr::null_mut())
}

unsafe fn get_drive_size(drive_name: &[u8]) -> u64 {
    let mut pdg: DISK_GEOMETRY = ::std::mem::zeroed();
    let handle = open_drive(drive_name, 0);
    if handle == INVALID_HANDLE_VALUE {
        return 0;
    }
    let mut junk = 0;
    let result = DeviceIoControl(handle,
                                 IOCTL_DISK_GET_DRIVE_GEOMETRY,
                                 ::std::ptr::null_mut(),
                                 0,
                                 &mut pdg as *mut DISK_GEOMETRY as *mut c_void,
                                 size_of::<DISK_GEOMETRY>() as DWORD,
                                 &mut junk,
                                 ::std::ptr::null_mut());
    CloseHandle(handle);
    if result == TRUE {
        *pdg.Cylinders.QuadPart() as u64 * pdg.TracksPerCylinder as u64 * pdg.SectorsPerTrack as u64 * pdg.BytesPerSector as u64
    } else {
        0
    }
}

unsafe fn get_disks() -> Vec<Disk> {
    let mut disks = Vec::new();
    let drives = GetLogicalDrives();
    if drives == 0 {
        return disks;
    }
    for x in 0..size_of::<DWORD>() * 8 {
        if (drives >> x) & 1 == 0 {
            continue
        }
        let mount_point = [b'A' + x as u8, b':', b'\\', 0];
        if GetDriveTypeA(mount_point.as_ptr() as *const i8) != DRIVE_FIXED {
            continue
        }
        let mut name = [0u8; MAX_PATH + 1];
        let mut file_system = [0u8; 32];
        if GetVolumeInformationA(mount_point.as_ptr() as *const i8,
                                           name.as_mut_ptr() as *mut i8,
                                           name.len() as DWORD, ::std::ptr::null_mut(),
                                           ::std::ptr::null_mut(),
                                           ::std::ptr::null_mut(),
                                           file_system.as_mut_ptr() as *mut i8,
                                           file_system.len() as DWORD) == 0 {
            continue
        }
        let mut pos = 0;
        for x in name.iter() {
            if *x == 0 {
                break
            }
            pos += 1;
        }
        let name = str::from_utf8_unchecked(&name[..pos]);

        pos = 0;
        for x in file_system.iter() {
            if *x == 0 {
                break
            }
            pos += 1;
        }
        let file_system = str::from_utf8_unchecked(&file_system[..pos]);

        let drive_name = [b'\\', b'\\', b'.', b'\\', b'a' + x as u8, b':', 0];
        let handle = open_drive(&drive_name, GENERIC_READ | GENERIC_WRITE);
        if handle == INVALID_HANDLE_VALUE {
            disks.push(new_disk(name, &mount_point, file_system, DiskType::Unknown(-1), 0));
            CloseHandle(handle);
            continue
        }
        let disk_size = get_drive_size(&drive_name);
        /*let mut spq_trim: ffi::STORAGE_PROPERTY_QUERY = ::std::mem::zeroed();
        spq_trim.PropertyId = ffi::StorageDeviceTrimProperty;
        spq_trim.QueryType = ffi::PropertyStandardQuery;
        let mut dtd: ffi::DEVICE_TRIM_DESCRIPTOR = ::std::mem::zeroed();*/
        #[allow(non_snake_case)]
        #[repr(C)]
        struct STORAGE_PROPERTY_QUERY {
            PropertyId: i32,
            QueryType: i32,
            AdditionalParameters: [BYTE; 1]
        }
        #[allow(non_snake_case)]
        #[repr(C)]
        struct DEVICE_TRIM_DESCRIPTOR {
            Version: DWORD,
            Size: DWORD,
            TrimEnabled: BOOLEAN,
        }
        let mut spq_trim = STORAGE_PROPERTY_QUERY {
            PropertyId: 8i32,
            QueryType: 0i32,
            AdditionalParameters: [0],
        };
        let mut dtd: DEVICE_TRIM_DESCRIPTOR = ::std::mem::zeroed();

        let mut dw_size = 0;
        if DeviceIoControl(handle, IOCTL_STORAGE_QUERY_PROPERTY,
                           &mut spq_trim as *mut STORAGE_PROPERTY_QUERY as *mut c_void,
                           size_of::<STORAGE_PROPERTY_QUERY>() as DWORD,
                           &mut dtd as *mut DEVICE_TRIM_DESCRIPTOR as *mut c_void,
                           size_of::<DEVICE_TRIM_DESCRIPTOR>() as DWORD,
                           &mut dw_size,
                           ::std::ptr::null_mut()) == 0 ||
           dw_size != size_of::<DEVICE_TRIM_DESCRIPTOR>() as DWORD {
            disks.push(new_disk(name, &mount_point as &[u8], file_system, DiskType::Unknown(-1),
                                disk_size));
            CloseHandle(handle);
            continue
        }
        let is_ssd = dtd.TrimEnabled != 0;
        CloseHandle(handle);
        disks.push(new_disk(name, &mount_point as &[u8], file_system,
                            if is_ssd { DiskType::SSD } else { DiskType::HDD },
                            disk_size));
    }
    disks
}

#[allow(non_snake_case)]
unsafe fn load_symbols() -> HashMap<String, u32> {
    use winapi::um::winreg::{HKEY_PERFORMANCE_DATA, RegQueryValueExA};

    let mut cbCounters = 0;
    let mut dwType = 0;
    let mut ret = HashMap::new();

    let _dwStatus = RegQueryValueExA(HKEY_PERFORMANCE_DATA,
                                     b"Counter 009\0".as_ptr() as *const _,
                                     ::std::ptr::null_mut(),
                                     &mut dwType as *mut i32 as *mut _,
                                     ::std::ptr::null_mut(),
                                     &mut cbCounters as *mut i32 as *mut _);

    let mut lpmszCounters = Vec::with_capacity(cbCounters as usize);
    lpmszCounters.set_len(cbCounters as usize);
    let _dwStatus = RegQueryValueExA(HKEY_PERFORMANCE_DATA,
                                     b"Counter 009\0".as_ptr() as *const _,
                                     ::std::ptr::null_mut(),
                                     &mut dwType as *mut i32 as *mut _,
                                     lpmszCounters.as_mut_ptr(),
                                     &mut cbCounters as *mut i32 as *mut _);

    for (pos, s) in lpmszCounters.split(|x| *x == 0)
                                 .collect::<Vec<_>>()
                                 .chunks(2)
                                 .filter(|&x| x.len() == 2 && !x[0].is_empty() && !x[1].is_empty())
                                 .map(|x| (u32::from_str_radix(String::from_utf8(x[0].to_vec()).unwrap().as_str(), 10).unwrap(), String::from_utf8(x[1].to_vec()).expect("invalid string"))) {
        ret.insert(s, pos as u32);
    }
    ret
}

fn get_translation(s: &String, map: &HashMap<String, u32>) -> Option<String> {
    use winapi::um::pdh::PdhLookupPerfNameByIndexW;

    if let Some(index) = map.get(s) {
        let mut size: usize = 0;
        unsafe {
            let _res = PdhLookupPerfNameByIndexW(::std::ptr::null(),
                                                *index,
                                                ::std::ptr::null_mut(),
                                                &mut size as *mut usize as *mut _);
            let mut v = Vec::with_capacity(size);
            v.set_len(size);
            let _res = PdhLookupPerfNameByIndexW(::std::ptr::null(),
                                                *index,
                                                v.as_mut_ptr() as *mut _,
                                                &mut size as *mut usize as *mut _);
            return Some(String::from_utf16(&v[..size - 1]).expect("invalid utf16"));
        }
    }
    None
}

fn add_counter(s: String, query: &mut Query, keys: &mut Option<KeyHandler>, counter_name: String,
               value: CounterValue) {
    let mut full = s.encode_utf16().collect::<Vec<_>>();
    full.push(0);
    if query.add_counter(&counter_name, full.clone(), value) {
        *keys = Some(KeyHandler::new(counter_name, full));
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
        };
        if let Some(ref mut query) = s.query {
            let x = unsafe { load_symbols() };
            let processor_trans = get_translation(&"Processor".to_owned(), &x).expect("translation failed");
            let idle_time_trans = get_translation(&"% Idle Time".to_owned(), &x).expect("translation failed");
            let proc_time_trans = get_translation(&"% Processor Time".to_owned(), &x).expect("translation failed");
            add_counter(format!("\\{}(_Total)\\{}", processor_trans, proc_time_trans),
                        query,
                        get_key_used(&mut s.processors[0]),
                        "tot_0".to_owned(),
                        CounterValue::Float(0.));
            add_counter(format!("\\{}(_Total)\\{}", processor_trans, idle_time_trans),
                        query,
                        get_key_idle(&mut s.processors[0]),
                        "tot_1".to_owned(),
                        CounterValue::Float(0.));
            for (pos, proc_) in s.processors.iter_mut().skip(1).enumerate() {
                add_counter(format!("\\{}({})\\{}", processor_trans, pos, proc_time_trans),
                            query,
                            get_key_used(proc_),
                            format!("{}_0", pos),
                            CounterValue::Float(0.));
                add_counter(format!("\\{}({})\\{}", processor_trans, pos, idle_time_trans),
                            query,
                            get_key_idle(proc_),
                            format!("{}_1", pos),
                            CounterValue::Float(0.));
            }

            let network_trans = get_translation(&"Network Interface".to_owned(), &x).expect("translation failed");
            let network_in_trans = get_translation(&"Bytes Received/Sec".to_owned(), &x).expect("translation failed");
            let network_out_trans = get_translation(&"Bytes Sent/sec".to_owned(), &x).expect("translation failed");

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
            self.swap_total = auto_cast!(mem_info.ullTotalPageFile - mem_info.ullTotalPhys, u64);
            self.mem_free = auto_cast!(mem_info.ullAvailPageFile, u64);
        }
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
        let mut process_ids: [DWORD; 1024] = [0; 1024];
        let mut cb_needed = 0;

        unsafe {
            let size = ::std::mem::size_of::<DWORD>() * process_ids.len();
            if K32EnumProcesses(process_ids.as_mut_ptr(),
                                size as DWORD,
                                &mut cb_needed) == 0 {
                return
            }
            let nb_processes = cb_needed / ::std::mem::size_of::<DWORD>() as DWORD;

            for i in 0..nb_processes as usize {
                let pid = process_ids[i] as Pid;
                if refresh_existing_process(self, pid, false) == true {
                    continue
                }
                let mut p = Process::new(pid, get_parent_process_id(pid), 0);
                update_proc_info(&mut p);
                self.process_list.insert(pid, p);
            }
        }
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

    fn get_process_by_name(&self, name: &str) -> Vec<&Process> {
        let mut ret = vec!();
        for val in self.process_list.values() {
            if val.name.starts_with(name) {
                ret.push(val);
            }
        }
        ret
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
}
