//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

use windows::processor::{self, Processor, Query};

use sys::disk::{new_disk, Disk};
use DiskType;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::mem::{size_of, zeroed};

use rayon::iter::{IntoParallelIterator, ParallelIterator};

use winapi::ctypes::c_void;

use winapi::shared::minwindef::{BYTE, DWORD, MAX_PATH, TRUE};
use winapi::um::fileapi::{
    CreateFileW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW, OPEN_EXISTING,
};
use winapi::um::handleapi::CloseHandle;
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::ioapiset::DeviceIoControl;
use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};
use winapi::um::winbase::DRIVE_FIXED;
use winapi::um::winioctl::{
    DISK_GEOMETRY, IOCTL_DISK_GET_DRIVE_GEOMETRY, IOCTL_STORAGE_QUERY_PROPERTY,
};
use winapi::um::winnt::{BOOLEAN, FILE_SHARE_READ, FILE_SHARE_WRITE, HANDLE};

pub struct KeyHandler {
    pub unique_id: String,
    pub win_key: Vec<u16>,
}

impl KeyHandler {
    pub fn new(unique_id: String, win_key: Vec<u16>) -> KeyHandler {
        KeyHandler {
            unique_id: unique_id,
            win_key: win_key,
        }
    }
}

pub fn init_processors() -> (Vec<Processor>, String, String) {
    unsafe {
        let mut sys_info: SYSTEM_INFO = zeroed();
        GetSystemInfo(&mut sys_info);
        let (vendor_id, brand) = processor::get_vendor_id_and_brand(&sys_info);
        let frequencies = processor::get_frequencies(sys_info.dwNumberOfProcessors as usize);
        let mut ret = Vec::with_capacity(sys_info.dwNumberOfProcessors as usize + 1);
        for nb in 0..sys_info.dwNumberOfProcessors {
            ret.push(Processor::new_with_values(
                &format!("CPU {}", nb + 1),
                vendor_id.clone(),
                brand.clone(),
                frequencies[nb as usize],
            ));
        }
        (ret, vendor_id, brand)
    }
}

pub unsafe fn open_drive(drive_name: &[u16], open_rights: DWORD) -> HANDLE {
    CreateFileW(
        drive_name.as_ptr(),
        open_rights,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        ::std::ptr::null_mut(),
        OPEN_EXISTING,
        0,
        ::std::ptr::null_mut(),
    )
}

pub unsafe fn get_drive_size(handle: HANDLE) -> u64 {
    let mut pdg: DISK_GEOMETRY = ::std::mem::zeroed();
    let mut junk = 0;
    let result = DeviceIoControl(
        handle,
        IOCTL_DISK_GET_DRIVE_GEOMETRY,
        ::std::ptr::null_mut(),
        0,
        &mut pdg as *mut DISK_GEOMETRY as *mut c_void,
        size_of::<DISK_GEOMETRY>() as DWORD,
        &mut junk,
        ::std::ptr::null_mut(),
    );
    if result == TRUE {
        *pdg.Cylinders.QuadPart() as u64
            * pdg.TracksPerCylinder as u64
            * pdg.SectorsPerTrack as u64
            * pdg.BytesPerSector as u64
    } else {
        0
    }
}

pub unsafe fn get_disks() -> Vec<Disk> {
    let drives = GetLogicalDrives();
    if drives == 0 {
        return Vec::new();
    }
    (0..size_of::<DWORD>() * 8)
        .into_par_iter()
        .filter_map(|x| {
            if (drives >> x) & 1 == 0 {
                return None;
            }
            let mount_point = [b'A' as u16 + x as u16, b':' as u16, b'\\' as u16, 0];
            if GetDriveTypeW(mount_point.as_ptr()) != DRIVE_FIXED {
                return None;
            }
            let mut name = [0u16; MAX_PATH + 1];
            let mut file_system = [0u16; 32];
            if GetVolumeInformationW(
                mount_point.as_ptr(),
                name.as_mut_ptr(),
                name.len() as DWORD,
                ::std::ptr::null_mut(),
                ::std::ptr::null_mut(),
                ::std::ptr::null_mut(),
                file_system.as_mut_ptr(),
                file_system.len() as DWORD,
            ) == 0
            {
                return None;
            }
            let mut pos = 0;
            for x in name.iter() {
                if *x == 0 {
                    break;
                }
                pos += 1;
            }
            let name = String::from_utf16_lossy(&name[..pos]);
            let name = OsStr::new(&name);

            pos = 0;
            for x in file_system.iter() {
                if *x == 0 {
                    break;
                }
                pos += 1;
            }
            let file_system: Vec<u8> = file_system[..pos].iter().map(|x| *x as u8).collect();

            let drive_name = [
                b'\\' as u16,
                b'\\' as u16,
                b'.' as u16,
                b'\\' as u16,
                b'A' as u16 + x as u16,
                b':' as u16,
                0,
            ];
            let handle = open_drive(&drive_name, 0);
            if handle == INVALID_HANDLE_VALUE {
                CloseHandle(handle);
                return new_disk(name, &mount_point, &file_system, DiskType::Unknown(-1), 0);
            }
            let disk_size = get_drive_size(handle);
            /*let mut spq_trim: ffi::STORAGE_PROPERTY_QUERY = ::std::mem::zeroed();
            spq_trim.PropertyId = ffi::StorageDeviceTrimProperty;
            spq_trim.QueryType = ffi::PropertyStandardQuery;
            let mut dtd: ffi::DEVICE_TRIM_DESCRIPTOR = ::std::mem::zeroed();*/
            #[allow(non_snake_case)]
            #[repr(C)]
            struct STORAGE_PROPERTY_QUERY {
                PropertyId: i32,
                QueryType: i32,
                AdditionalParameters: [BYTE; 1],
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
            if DeviceIoControl(
                handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                &mut spq_trim as *mut STORAGE_PROPERTY_QUERY as *mut c_void,
                size_of::<STORAGE_PROPERTY_QUERY>() as DWORD,
                &mut dtd as *mut DEVICE_TRIM_DESCRIPTOR as *mut c_void,
                size_of::<DEVICE_TRIM_DESCRIPTOR>() as DWORD,
                &mut dw_size,
                ::std::ptr::null_mut(),
            ) == 0
                || dw_size != size_of::<DEVICE_TRIM_DESCRIPTOR>() as DWORD
            {
                CloseHandle(handle);
                return new_disk(
                    name,
                    &mount_point,
                    &file_system,
                    DiskType::Unknown(-1),
                    disk_size,
                );
            }
            let is_ssd = dtd.TrimEnabled != 0;
            CloseHandle(handle);
            new_disk(
                name,
                &mount_point,
                &file_system,
                if is_ssd { DiskType::SSD } else { DiskType::HDD },
                disk_size,
            )
        })
        .collect::<Vec<_>>()
}

#[allow(non_snake_case)]
pub unsafe fn load_symbols() -> HashMap<String, u32> {
    use winapi::um::winreg::{RegQueryValueExA, HKEY_PERFORMANCE_DATA};

    let mut cbCounters = 0;
    let mut dwType = 0;
    let mut ret = HashMap::new();

    let _dwStatus = RegQueryValueExA(
        HKEY_PERFORMANCE_DATA,
        b"Counter 009\0".as_ptr() as *const _,
        ::std::ptr::null_mut(),
        &mut dwType as *mut i32 as *mut _,
        ::std::ptr::null_mut(),
        &mut cbCounters as *mut i32 as *mut _,
    );

    let mut lpmszCounters = Vec::with_capacity(cbCounters as usize);
    lpmszCounters.set_len(cbCounters as usize);
    let _dwStatus = RegQueryValueExA(
        HKEY_PERFORMANCE_DATA,
        b"Counter 009\0".as_ptr() as *const _,
        ::std::ptr::null_mut(),
        &mut dwType as *mut i32 as *mut _,
        lpmszCounters.as_mut_ptr(),
        &mut cbCounters as *mut i32 as *mut _,
    );
    for (pos, s) in lpmszCounters
        .split(|x| *x == 0)
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .chunks(2)
        .filter(|&x| x.len() == 2)
        .filter_map(
            |x| match (std::str::from_utf8(x[0]), String::from_utf8(x[1].to_vec())) {
                (Ok(n), Ok(s)) => {
                    if let Ok(n) = u32::from_str_radix(n, 10) {
                        Some((n, s))
                    } else {
                        None
                    }
                }
                _ => None,
            },
        )
    {
        ret.insert(s, pos as u32);
    }
    ret
}

pub fn get_translation(s: &String, map: &HashMap<String, u32>) -> Option<String> {
    use winapi::um::pdh::PdhLookupPerfNameByIndexW;

    if let Some(index) = map.get(s) {
        let mut size: usize = 0;
        unsafe {
            let _res = PdhLookupPerfNameByIndexW(
                ::std::ptr::null(),
                *index,
                ::std::ptr::null_mut(),
                &mut size as *mut usize as *mut _,
            );
            if size == 0 {
                return Some(String::new());
            } else {
                let mut v = Vec::with_capacity(size);
                v.set_len(size);
                let _res = PdhLookupPerfNameByIndexW(
                    ::std::ptr::null(),
                    *index,
                    v.as_mut_ptr() as *mut _,
                    &mut size as *mut usize as *mut _,
                );
                return Some(String::from_utf16(&v[..size - 1]).expect("invalid utf16"));
            }
        }
    }
    None
}

pub fn add_counter(
    s: String,
    query: &mut Query,
    keys: &mut Option<KeyHandler>,
    counter_name: String,
) {
    let mut full = s.encode_utf16().collect::<Vec<_>>();
    full.push(0);
    if query.add_counter(&counter_name, full.clone()) {
        *keys = Some(KeyHandler::new(counter_name, full));
    }
}
