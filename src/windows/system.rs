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

use libc::c_char;

use winapi::shared::minwindef::{BYTE, DWORD, FALSE, MAX_PATH, TRUE};
use winapi::um::fileapi::{
    CreateFileA, GetDriveTypeA, GetLogicalDrives, GetVolumeInformationA, OPEN_EXISTING,
};
use winapi::um::handleapi::CloseHandle;
use winapi::um::ioapiset::DeviceIoControl;
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::psapi::{
    K32EnumProcesses, K32EnumProcessModulesEx, K32GetModuleBaseNameA, LIST_MODULES_ALL,
};
use winapi::um::sysinfoapi::{
    GetSystemInfo, GlobalMemoryStatusEx, MEMORYSTATUSEX, SYSTEM_INFO,
};
use winapi::um::winioctl::{
    DISK_GEOMETRY, IOCTL_STORAGE_QUERY_PROPERTY, IOCTL_DISK_GET_DRIVE_GEOMETRY,
};
use winapi::um::winnt::{
    BOOLEAN, FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE, HANDLE,
    PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};
use winapi::um::winbase::DRIVE_FIXED;

/// Structs containing system's information.
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
    keys: Vec<String>,
}

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
                0, ::std::ptr::null_mut())
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
        #[repr(C)]
        struct STORAGE_PROPERTY_QUERY {
            PropertyId: i32,
            QueryType: i32,
            AdditionalParameters: [BYTE; 1]
        }
        #[repr(C)]
        struct DEVICE_TRIM_DESCRIPTOR {
            Version: DWORD,
            Size: DWORD,
            TrimEnabled: BOOLEAN,
        }
        let spq_trim = STORAGE_PROPERTY_QUERY {
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

impl System {
    /// Creates a new `System` instance. It only contains the disks' list at this stage. Use the
    /// [`refresh_all`] method to update its internal information (or any of the `refresh_` method).
    ///
    /// [`refresh_all`]: #method.refresh_all
    pub fn new() -> System {
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
            keys: Vec::new(),
        };
        if let Some(query) = s.query {
            let tmp = "0_0".to_owned();
            query.add_counter(&tmp, b"\\Processor(_Total)\\% Processor Time\0");
            s.keys.push(tmp);
            let tmp = "0_1".to_owned();
            query.add_counter(&tmp, b"\\Processor(_Total)\\% Idle Time\0");
            s.keys.push(tmp);
            for (pos, _) in s.processors.iter().skip(1).enumerate() {
                let tmp = format!("{}_0", pos);
                query.add_counter(&tmp,
                                  format!("\\Processor({})\\% Processor Time\0", pos).as_bytes());
                s.keys.push(tmp);
                let tmp = format!("{}_1", pos);
                query.add_counter(&tmp,
                                  format!("\\Processor({})\\% Idle Time\0", pos).as_bytes());
                s.keys.push(tmp);
            }
            query.start();
        }
        s.refresh_all();
        s
    }

    /// Refresh system information (such as memory, swap, CPU usage and components' temperature).
    pub fn refresh_system(&mut self) {
        unsafe {
            let mut mem_info: MEMORYSTATUSEX = zeroed();
            mem_info.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
            GlobalMemoryStatusEx(&mut mem_info);
            self.mem_total = auto_cast!(mem_info.ullTotalPhys, u64);
            self.mem_free = auto_cast!(mem_info.ullAvailPhys, u64);
            self.swap_total = auto_cast!(mem_info.ullTotalPageFile - mem_info.ullTotalPhys, u64);
            self.mem_free = auto_cast!(mem_info.ullAvailPageFile, u64);
        }
        if let Some(query) = self.query {
            for (keys, p) in self.keys.windows(2).zip(self.processors.iter_mut()) {
                let proc_time = query.get(&keys[0]).unwrap_or(0.);
                let idle_time = query.get(&keys[1]).unwrap_or(0.);
                if proc_time != 0. {
                    set_cpu_usage(p, (idle_time / proc_time * 100.) as f32);
                } else {
                    set_cpu_usage(p, 0.);
                }
            }
        }
    }

    /// Get all processes and update their information.
    pub fn refresh_process(&mut self) {
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
                let pid = process_ids[i];
                if pid == 0 {
                    continue
                }
                let options = PROCESS_QUERY_INFORMATION | PROCESS_VM_READ;
                let process_handler = OpenProcess(options, FALSE, pid);
                if process_handler.is_null() {
                    continue
                }
                if let Some(ref mut entry) = self.process_list.get_mut(&(pid as usize)) {
                    update_proc_info(entry);
                    continue
                }
                let mut h_mod = ::std::ptr::null_mut();
                let mut process_name = [0 as u8; MAX_PATH];

                if K32EnumProcessModulesEx(process_handler,
                                           &mut h_mod,
                                           ::std::mem::size_of::<DWORD>() as DWORD,
                                           &mut cb_needed,
                                           LIST_MODULES_ALL) != 0 {
                    K32GetModuleBaseNameA(process_handler,
                                          h_mod,
                                          process_name.as_mut_ptr() as *mut c_char,
                                          MAX_PATH as DWORD);
                }
                let mut p = Process::new(process_handler, pid, 0,
                                         String::from_utf8_unchecked(process_name.to_vec())); // TODO: should be start time, not 0
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

    /// Refreshes the listed disks' information.
    pub fn refresh_disks(&mut self) {
        for disk in &mut self.disks {
            disk.update();
        }
    }
    
    // COMMON PART
    //
    // Need to be moved into a "common" file to avoid duplication.

    /// Refreshes all system, processes and disks information.
    pub fn refresh_all(&mut self) {
        self.refresh_system();
        self.refresh_process();
        self.refresh_disks();
    }

    /// Returns the process list.
    pub fn get_process_list(&self) -> &HashMap<usize, Process> {
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
    pub fn get_processor_list(&self) -> &[Processor] {
        &self.processors[..]
    }

    /// Returns total RAM size (in kB).
    pub fn get_total_memory(&self) -> u64 {
        self.mem_total >> 10
    }

    /// Returns free RAM size (in kB).
    pub fn get_free_memory(&self) -> u64 {
        self.mem_free >> 10
    }

    /// Returns used RAM size (in kB).
    pub fn get_used_memory(&self) -> u64 {
        (self.mem_total - self.mem_free) >> 10
    }

    /// Returns SWAP size (in kB).
    pub fn get_total_swap(&self) -> u64 {
        self.swap_total >> 10
    }

    /// Returns free SWAP size (in kB).
    pub fn get_free_swap(&self) -> u64 {
        self.swap_free >> 10
    }

    /// Returns used SWAP size (in kB).
    pub fn get_used_swap(&self) -> u64 {
        (self.swap_total - self.swap_free) >> 10
    }

    /// Returns components list.
    pub fn get_components_list(&self) -> &[Component] {
        &self.temperatures[..]
    }

    /// Returns disks' list.
    pub fn get_disks(&self) -> &[Disk] {
        &self.disks[..]
    }
}

/*fn get_page_size() -> u64 {
    let mut system_info = unsafe { ::std::mem::zeroed() };
    unsafe { kernel32::GetSystemInfo(&mut system_info); }
    system_info.dwPageSize as u64
}*/
