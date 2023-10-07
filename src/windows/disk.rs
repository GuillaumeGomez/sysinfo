// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{DiskExt, DiskKind};

use std::ffi::{c_void, OsStr, OsString};
use std::mem::size_of;
use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, MAX_PATH};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    FILE_ACCESS_RIGHTS, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Ioctl::{
    PropertyStandardQuery, StorageDeviceSeekPenaltyProperty, DEVICE_SEEK_PENALTY_DESCRIPTOR,
    IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_PROPERTY_QUERY,
};
use windows::Win32::System::WindowsProgramming::{DRIVE_FIXED, DRIVE_REMOVABLE};
use windows::Win32::System::IO::DeviceIoControl;

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
                let mut tmp = 0;
                let lpdirectoryname = PCWSTR::from_raw(self.mount_point.as_ptr());
                if GetDiskFreeSpaceExW(lpdirectoryname, None, None, Some(&mut tmp)).is_ok() {
                    self.available_space = tmp;
                    return true;
                }
            }
        }
        false
    }
}

pub(crate) struct DisksInner {
    pub(crate) disks: Vec<Disk>,
}

impl DisksInner {
    pub(crate) fn new() -> Self {
        Self {
            disks: Vec::with_capacity(2),
        }
    }

    pub(crate) fn refresh_list(&mut self) {
        unsafe {
            self.disks = get_disks();
        }
    }

    pub(crate) fn disks(&self) -> &[Disk] {
        &self.disks
    }

    pub(crate) fn disks_mut(&mut self) -> &mut [Disk] {
        &mut self.disks
    }
}

struct HandleWrapper(HANDLE);

impl HandleWrapper {
    unsafe fn new(drive_name: &[u16], open_rights: FILE_ACCESS_RIGHTS) -> Option<Self> {
        let lpfilename = PCWSTR::from_raw(drive_name.as_ptr());
        let handle = CreateFileW(
            lpfilename,
            open_rights.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            Default::default(),
            HANDLE::default(),
        )
        .ok()?;
        Some(Self(handle))
    }
}

impl Drop for HandleWrapper {
    fn drop(&mut self) {
        let _err = unsafe { CloseHandle(self.0) };
    }
}

unsafe fn get_drive_size(mount_point: &[u16]) -> Option<(u64, u64)> {
    let mut total_size = 0;
    let mut available_space = 0;
    let lpdirectoryname = PCWSTR::from_raw(mount_point.as_ptr());
    if GetDiskFreeSpaceExW(
        lpdirectoryname,
        None,
        Some(&mut total_size),
        Some(&mut available_space),
    )
    .is_ok()
    {
        Some((total_size, available_space))
    } else {
        None
    }
}

pub(crate) unsafe fn get_disks() -> Vec<Disk> {
    let drives = GetLogicalDrives();
    if drives == 0 {
        return Vec::new();
    }

    #[cfg(feature = "multithread")]
    use rayon::iter::ParallelIterator;

    crate::utils::into_iter(0..u32::BITS)
        .filter_map(|x| {
            if (drives >> x) & 1 == 0 {
                return None;
            }
            let mount_point = [b'A' as u16 + x as u16, b':' as u16, b'\\' as u16, 0];

            let raw_mount_point = PCWSTR::from_raw(mount_point.as_ptr());
            let drive_type = GetDriveTypeW(raw_mount_point);

            let is_removable = drive_type == DRIVE_REMOVABLE;

            if drive_type != DRIVE_FIXED && drive_type != DRIVE_REMOVABLE {
                return None;
            }
            let mut name = [0u16; MAX_PATH as usize + 1];
            let mut file_system = [0u16; 32];
            let volume_info_res = GetVolumeInformationW(
                raw_mount_point,
                Some(&mut name),
                None,
                None,
                None,
                Some(&mut file_system),
            )
            .is_ok();
            if !volume_info_res {
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
            let handle = HandleWrapper::new(&drive_name, Default::default())?;
            let (total_space, available_space) = get_drive_size(&mount_point)?;
            if total_space == 0 {
                return None;
            }
            let spq_trim = STORAGE_PROPERTY_QUERY {
                PropertyId: StorageDeviceSeekPenaltyProperty,
                QueryType: PropertyStandardQuery,
                AdditionalParameters: [0],
            };
            let mut result: DEVICE_SEEK_PENALTY_DESCRIPTOR = std::mem::zeroed();

            let mut dw_size = 0;
            let device_io_control = DeviceIoControl(
                handle.0,
                IOCTL_STORAGE_QUERY_PROPERTY,
                Some(&spq_trim as *const STORAGE_PROPERTY_QUERY as *const c_void),
                size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                Some(&mut result as *mut DEVICE_SEEK_PENALTY_DESCRIPTOR as *mut c_void),
                size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() as u32,
                Some(&mut dw_size),
                None,
            )
            .is_ok();
            let type_ = if !device_io_control
                || dw_size != size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() as u32
            {
                DiskKind::Unknown(-1)
            } else {
                let is_hdd = result.IncursSeekPenalty.as_bool();
                if is_hdd {
                    DiskKind::HDD
                } else {
                    DiskKind::SSD
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
