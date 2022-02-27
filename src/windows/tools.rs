// Take a look at the license at the top of the repository in the LICENSE file.

use crate::DiskType;

use crate::sys::disk::{new_disk, Disk};
use crate::sys::processor::{self, Processor, Query};

use std::ffi::OsStr;
use std::mem::{size_of, zeroed};

use winapi::shared::ntdef::ULARGE_INTEGER;
use winapi::{ctypes::c_void, um::winbase::DRIVE_REMOVABLE};

use winapi::shared::minwindef::{DWORD, MAX_PATH};
use winapi::um::fileapi::{
    CreateFileW, GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    OPEN_EXISTING,
};
use winapi::um::handleapi::CloseHandle;
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::ioapiset::DeviceIoControl;
use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};
use winapi::um::winbase::DRIVE_FIXED;
use winapi::um::winioctl::{
    DEVICE_TRIM_DESCRIPTOR, IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_PROPERTY_QUERY,
};
use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE, HANDLE};

pub(crate) struct KeyHandler {
    pub unique_id: String,
}

impl KeyHandler {
    pub fn new(unique_id: String) -> KeyHandler {
        KeyHandler { unique_id }
    }
}

pub(crate) fn init_processors() -> (Vec<Processor>, String, String) {
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
        std::ptr::null_mut(),
        OPEN_EXISTING,
        0,
        std::ptr::null_mut(),
    )
}

pub unsafe fn get_drive_size(mount_point: &[u16]) -> u64 {
    let mut tmp: ULARGE_INTEGER = std::mem::zeroed();
    if GetDiskFreeSpaceExW(
        mount_point.as_ptr(),
        std::ptr::null_mut(),
        &mut tmp,
        std::ptr::null_mut(),
    ) != 0
    {
        *tmp.QuadPart() as u64
    } else {
        0
    }
}

pub unsafe fn get_disks() -> Vec<Disk> {
    let drives = GetLogicalDrives();
    if drives == 0 {
        return Vec::new();
    }

    #[cfg(feature = "multithread")]
    use rayon::iter::ParallelIterator;

    crate::utils::into_iter(0..DWORD::BITS)
        .filter_map(|x| {
            if (drives >> x) & 1 == 0 {
                return None;
            }
            let mount_point = [b'A' as u16 + x as u16, b':' as u16, b'\\' as u16, 0];

            let drive_type = GetDriveTypeW(mount_point.as_ptr());

            let is_removable = drive_type == DRIVE_REMOVABLE;

            if drive_type != DRIVE_FIXED && drive_type != DRIVE_REMOVABLE {
                return None;
            }
            let mut name = [0u16; MAX_PATH + 1];
            let mut file_system = [0u16; 32];
            if GetVolumeInformationW(
                mount_point.as_ptr(),
                name.as_mut_ptr(),
                name.len() as DWORD,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
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
                return new_disk(
                    name,
                    &mount_point,
                    &file_system,
                    DiskType::Unknown(-1),
                    0,
                    is_removable,
                );
            }
            let disk_size = get_drive_size(&mount_point);
            /*let mut spq_trim: STORAGE_PROPERTY_QUERY = std::mem::zeroed();
            spq_trim.PropertyId = StorageDeviceTrimProperty;
            spq_trim.QueryType = PropertyStandardQuery;
            let mut dtd: DEVICE_TRIM_DESCRIPTOR = std::mem::zeroed();*/
            let mut spq_trim = STORAGE_PROPERTY_QUERY {
                PropertyId: 8,
                QueryType: 0,
                AdditionalParameters: [0],
            };
            let mut dtd: DEVICE_TRIM_DESCRIPTOR = std::mem::zeroed();

            let mut dw_size = 0;
            if DeviceIoControl(
                handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                &mut spq_trim as *mut STORAGE_PROPERTY_QUERY as *mut c_void,
                size_of::<STORAGE_PROPERTY_QUERY>() as DWORD,
                &mut dtd as *mut DEVICE_TRIM_DESCRIPTOR as *mut c_void,
                size_of::<DEVICE_TRIM_DESCRIPTOR>() as DWORD,
                &mut dw_size,
                std::ptr::null_mut(),
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
                    is_removable,
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
                is_removable,
            )
        })
        .collect::<Vec<_>>()
}

pub(crate) fn add_english_counter(
    s: String,
    query: &mut Query,
    keys: &mut Option<KeyHandler>,
    counter_name: String,
) {
    let mut full = s.encode_utf16().collect::<Vec<_>>();
    full.push(0);
    if query.add_english_counter(&counter_name, full) {
        *keys = Some(KeyHandler::new(counter_name));
    }
}
