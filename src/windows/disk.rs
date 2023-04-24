// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{DiskExt, DiskKind};

use std::ffi::{OsStr, OsString};
use std::mem::size_of;
use std::path::Path;

use winapi::ctypes::c_void;
use winapi::shared::minwindef::{DWORD, MAX_PATH};
use winapi::um::fileapi::{
    CreateFileW, GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    OPEN_EXISTING,
};
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::ioapiset::DeviceIoControl;
use winapi::um::winbase::{DRIVE_FIXED, DRIVE_REMOVABLE};
use winapi::um::winioctl::{
    PropertyStandardQuery, StorageDeviceSeekPenaltyProperty, IOCTL_STORAGE_QUERY_PROPERTY,
    STORAGE_PROPERTY_QUERY,
};
use winapi::um::winnt::{BOOLEAN, FILE_SHARE_READ, FILE_SHARE_WRITE, HANDLE, ULARGE_INTEGER};

#[doc = include_str!("../../md_doc/disk.md")]
pub struct Disk {
    type_: DiskKind,
    name: OsString,
    file_system: Vec<u8>,
    mount_point: Vec<u16>,
    s_mount_point: String,
    total_space: u64,
    available_space: u64,
    is_removable: bool,
}

impl DiskExt for Disk {
    fn kind(&self) -> DiskKind {
        self.type_
    }

    fn name(&self) -> &OsStr {
        &self.name
    }

    fn file_system(&self) -> &[u8] {
        &self.file_system
    }

    fn mount_point(&self) -> &Path {
        Path::new(&self.s_mount_point)
    }

    fn total_space(&self) -> u64 {
        self.total_space
    }

    fn available_space(&self) -> u64 {
        self.available_space
    }

    fn is_removable(&self) -> bool {
        self.is_removable
    }

    fn refresh(&mut self) -> bool {
        if self.total_space != 0 {
            unsafe {
                let mut tmp: ULARGE_INTEGER = std::mem::zeroed();
                if GetDiskFreeSpaceExW(
                    self.mount_point.as_ptr(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut tmp,
                ) != 0
                {
                    self.available_space = *tmp.QuadPart();
                    return true;
                }
            }
        }
        false
    }
}

struct HandleWrapper(HANDLE);

impl HandleWrapper {
    unsafe fn new(drive_name: &[u16], open_rights: DWORD) -> Option<Self> {
        let handle = CreateFileW(
            drive_name.as_ptr(),
            open_rights,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        if handle == INVALID_HANDLE_VALUE {
            CloseHandle(handle);
            None
        } else {
            Some(Self(handle))
        }
    }
}

impl Drop for HandleWrapper {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

unsafe fn get_drive_size(mount_point: &[u16]) -> Option<(u64, u64)> {
    let mut total_size: ULARGE_INTEGER = std::mem::zeroed();
    let mut available_space: ULARGE_INTEGER = std::mem::zeroed();
    if GetDiskFreeSpaceExW(
        mount_point.as_ptr(),
        std::ptr::null_mut(),
        &mut total_size,
        &mut available_space,
    ) != 0
    {
        Some((
            *total_size.QuadPart() as _,
            *available_space.QuadPart() as _,
        ))
    } else {
        None
    }
}

// FIXME: To be removed once <https://github.com/retep998/winapi-rs/pull/1028> has been merged.
#[allow(non_snake_case)]
#[repr(C)]
struct DEVICE_SEEK_PENALTY_DESCRIPTOR {
    Version: DWORD,
    Size: DWORD,
    IncursSeekPenalty: BOOLEAN,
}

pub(crate) unsafe fn get_disks() -> Vec<Disk> {
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
            let handle = HandleWrapper::new(&drive_name, 0)?;
            let (total_space, available_space) = get_drive_size(&mount_point)?;
            if total_space == 0 {
                return None;
            }
            let mut spq_trim = STORAGE_PROPERTY_QUERY {
                PropertyId: StorageDeviceSeekPenaltyProperty,
                QueryType: PropertyStandardQuery,
                AdditionalParameters: [0],
            };
            let mut result: DEVICE_SEEK_PENALTY_DESCRIPTOR = std::mem::zeroed();

            let mut dw_size = 0;
            let type_ = if DeviceIoControl(
                handle.0,
                IOCTL_STORAGE_QUERY_PROPERTY,
                &mut spq_trim as *mut STORAGE_PROPERTY_QUERY as *mut c_void,
                size_of::<STORAGE_PROPERTY_QUERY>() as DWORD,
                &mut result as *mut DEVICE_SEEK_PENALTY_DESCRIPTOR as *mut c_void,
                size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() as DWORD,
                &mut dw_size,
                std::ptr::null_mut(),
            ) == 0
                || dw_size != size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() as DWORD
            {
                DiskKind::Unknown(-1)
            } else {
                let is_ssd = result.IncursSeekPenalty == 0;
                if is_ssd {
                    DiskKind::SSD
                } else {
                    DiskKind::HDD
                }
            };
            Some(Disk {
                type_,
                name: name.to_owned(),
                file_system: file_system.to_vec(),
                mount_point: mount_point.to_vec(),
                s_mount_point: String::from_utf16_lossy(&mount_point[..mount_point.len() - 1]),
                total_space,
                available_space,
                is_removable,
            })
        })
        .collect::<Vec<_>>()
}
