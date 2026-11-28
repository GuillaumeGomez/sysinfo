// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::utils::HandleWrapper;
use crate::{Disk, DiskKind, DiskRefreshKind, DiskUsage, Error as SError};

use std::ffi::{OsStr, OsString};
use std::mem::size_of;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;

use windows::Win32::Foundation::{CloseHandle, MAX_PATH};
use windows::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, BusTypeFileBackedVirtual, CreateFileW, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, GetDiskFreeSpaceExW, GetDriveTypeW,
    GetFileInformationByHandle, GetLogicalDriveStringsW, GetVolumeInformationW, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::{
    DEVICE_SEEK_PENALTY_DESCRIPTOR, DISK_PERFORMANCE, IOCTL_DISK_PERFORMANCE,
    IOCTL_STORAGE_QUERY_PROPERTY, PropertyStandardQuery, STORAGE_DEVICE_DESCRIPTOR,
    STORAGE_PROPERTY_QUERY, StorageDeviceProperty, StorageDeviceSeekPenaltyProperty,
};
use windows::Win32::System::SystemServices::FILE_READ_ONLY_VOLUME;
use windows::Win32::System::WindowsProgramming::{
    DRIVE_CDROM, DRIVE_FIXED, DRIVE_REMOTE, DRIVE_REMOVABLE,
};
use windows::core::PCWSTR;

pub(crate) struct DiskInner {
    type_: DiskKind,
    name: OsString,
    volume_serial_number: Option<u32>,
    pub(crate) file_system: OsString,
    mount_point: Vec<u16>,
    s_mount_point: OsString,
    total_space: u64,
    available_space: u64,
    is_removable: bool,
    is_read_only: bool,
    device_path: Vec<u16>,
    old_written_bytes: u64,
    old_read_bytes: u64,
    written_bytes: u64,
    read_bytes: u64,
    updated: bool,
}

#[cfg(test)]
impl Default for DiskInner {
    fn default() -> Self {
        Self {
            type_: DiskKind::Unknown(0),
            name: OsString::new(),
            volume_serial_number: None,
            file_system: OsString::new(),
            mount_point: Vec::new(),
            s_mount_point: OsString::new(),
            total_space: 0,
            available_space: 0,
            is_removable: false,
            is_read_only: false,
            device_path: Vec::new(),
            old_written_bytes: 0,
            old_read_bytes: 0,
            written_bytes: 0,
            read_bytes: 0,
            updated: false,
        }
    }
}

impl DiskInner {
    pub(crate) fn kind(&self) -> DiskKind {
        self.type_
    }

    pub(crate) fn name(&self) -> &OsStr {
        &self.name
    }

    pub(crate) fn id(&self) -> Option<u64> {
        self.volume_serial_number.map(|v| v as u64)
    }

    pub(crate) fn file_system(&self) -> &OsStr {
        &self.file_system
    }

    pub(crate) fn mount_point(&self) -> &Path {
        self.s_mount_point.as_ref()
    }

    pub(crate) fn total_space(&self) -> u64 {
        self.total_space
    }

    pub(crate) fn available_space(&self) -> u64 {
        self.available_space
    }

    pub(crate) fn is_removable(&self) -> bool {
        self.is_removable
    }

    pub(crate) fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    pub(crate) fn refresh_specifics(&mut self, refreshes: DiskRefreshKind) -> bool {
        if refreshes.kind() || refreshes.io_usage() {
            unsafe {
                if let Some(handle) =
                    HandleWrapper::new_from_file(&self.device_path, Default::default())
                {
                    if refreshes.kind() && self.type_ == DiskKind::Unknown(-1) {
                        self.type_ = get_disk_kind(&handle);
                    }

                    if refreshes.io_usage() {
                        if let Some((read_bytes, written_bytes)) = get_disk_io(handle) {
                            self.old_read_bytes = self.read_bytes;
                            self.old_written_bytes = self.written_bytes;
                            self.read_bytes = read_bytes;
                            self.written_bytes = written_bytes;
                        } else {
                            sysinfo_debug!("Failed to update disk i/o stats");
                        }
                    }
                }
            }
        }

        if refreshes.storage()
            && let Some((total_space, available_space)) =
                unsafe { get_drive_size(&self.mount_point) }
        {
            self.total_space = total_space;
            self.available_space = available_space;
        }
        true
    }

    pub(crate) fn usage(&self) -> DiskUsage {
        DiskUsage {
            read_bytes: self.read_bytes.saturating_sub(self.old_read_bytes),
            total_read_bytes: self.read_bytes,
            written_bytes: self.written_bytes.saturating_sub(self.old_written_bytes),
            total_written_bytes: self.written_bytes,
        }
    }
}

pub(crate) struct DisksInner {
    pub(crate) disks: Vec<Disk>,
}

impl DisksInner {
    pub(crate) fn new() -> Result<Self, SError> {
        Ok(Self {
            disks: Vec::with_capacity(2),
        })
    }

    pub(crate) fn refresh_specifics(
        &mut self,
        remove_not_listed_disks: bool,
        refreshes: DiskRefreshKind,
    ) {
        unsafe {
            get_list(&mut self.disks, remove_not_listed_disks, refreshes);
        }
    }

    pub(crate) fn list(&self) -> &[Disk] {
        &self.disks
    }

    pub(crate) fn list_mut(&mut self) -> &mut [Disk] {
        &mut self.disks
    }

    pub(crate) fn into_vec(self) -> Vec<Disk> {
        self.disks
    }
}

unsafe fn get_drive_size(mount_point: &[u16]) -> Option<(u64, u64)> {
    let mut total_size = 0;
    let mut available_space = 0;
    let lpdirectoryname = PCWSTR::from_raw(mount_point.as_ptr());
    if unsafe {
        GetDiskFreeSpaceExW(
            lpdirectoryname,
            None,
            Some(&mut total_size),
            Some(&mut available_space),
        )
    }
    .is_ok()
    {
        Some((total_size, available_space))
    } else {
        None
    }
}

pub(crate) unsafe fn get_list(
    disks: &mut Vec<Disk>,
    remove_not_listed_disks: bool,
    refreshes: DiskRefreshKind,
) {
    for mount_path in get_logical_drive_strings() {
        let raw_mount_path = PCWSTR::from_raw(mount_path.as_ptr());
        let drive_type = unsafe { GetDriveTypeW(raw_mount_path) };

        if !(matches!(drive_type, DRIVE_FIXED | DRIVE_REMOVABLE | DRIVE_CDROM)
            || (drive_type == DRIVE_REMOTE && refreshes.network()))
        {
            continue;
        }

        let is_removable = matches!(drive_type, DRIVE_REMOVABLE | DRIVE_CDROM | DRIVE_REMOTE);

        let mut name = [0u16; MAX_PATH as usize + 1];
        let mut file_system = [0u16; 32];
        let mut flags = 0;
        let volume_info_res = unsafe {
            GetVolumeInformationW(
                raw_mount_path,
                Some(&mut name),
                None,
                None,
                Some(&mut flags),
                Some(&mut file_system),
            )
        }
        .is_ok();
        if !volume_info_res {
            sysinfo_debug!(
                "Error: GetVolumeInformationW = {:?}",
                windows::core::Error::from_thread().code()
            );
            continue;
        }
        let is_read_only = (flags & FILE_READ_ONLY_VOLUME) != 0;

        // The `device_path` that `CreateFileW` expects is `\\.\${local_path}` without trailing backslash.
        let device_path = r"\\.\"
            .encode_utf16()
            .chain(
                mount_path[..mount_path.len().saturating_sub(2)]
                    .iter()
                    .copied(),
            )
            .chain([0])
            .collect::<Vec<_>>();

        let name = os_string_from_zero_terminated(&name);
        let file_system = os_string_from_zero_terminated(&file_system);

        if let Some(disk) = disks
            .iter_mut()
            .find(|d| d.inner.mount_point == mount_path && d.inner.file_system == file_system)
        {
            disk.refresh_specifics(refreshes);
            disk.inner.updated = true;
            continue;
        }

        //  FIXME: replace with `std::os::windows::fs::MetadataExt::volume_serial_number` when stable
        //  https://github.com/rust-lang/rust/issues/63010
        let volume_serial_number = unsafe {
            if let Ok(handle) = CreateFileW(
                PCWSTR(mount_path.as_ptr()),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                None,
            ) {
                let mut info: BY_HANDLE_FILE_INFORMATION = std::mem::zeroed();
                let volume_serial_number = if GetFileInformationByHandle(handle, &mut info).is_ok()
                {
                    Some(info.dwVolumeSerialNumber)
                } else {
                    None
                };
                let _ = CloseHandle(handle);
                volume_serial_number
            } else {
                None
            }
        };

        let type_ = if drive_type == DRIVE_REMOTE {
            DiskKind::Network
        } else {
            DiskKind::Unknown(-1)
        };

        let mut disk = DiskInner {
            type_,
            name: name.clone(),
            volume_serial_number,
            file_system: file_system.clone(),
            s_mount_point: OsString::from_wide(&mount_path[..mount_path.len().saturating_sub(1)]),
            mount_point: mount_path,
            total_space: 0,
            available_space: 0,
            is_removable,
            is_read_only,
            device_path,
            old_read_bytes: 0,
            old_written_bytes: 0,
            read_bytes: 0,
            written_bytes: 0,
            updated: true,
        };
        disk.refresh_specifics(refreshes);
        disks.push(Disk { inner: disk });
    }

    if remove_not_listed_disks {
        disks.retain_mut(|disk| {
            if !disk.inner.updated {
                return false;
            }
            disk.inner.updated = false;
            true
        });
    } else {
        for c in disks.iter_mut() {
            c.inner.updated = false;
        }
    }
}

fn get_logical_drive_strings() -> Vec<Vec<u16>> {
    unsafe {
        let required = GetLogicalDriveStringsW(None);

        if required == 0 {
            sysinfo_debug!(
                "GetLogicalDriveStringsW failed: {:?}",
                windows::core::Error::from_thread().code()
            );
            return Vec::new();
        }

        let mut buffer = vec![0u16; required as usize];

        let len = GetLogicalDriveStringsW(Some(&mut buffer));

        if len == 0 || len > required {
            sysinfo_debug!(
                "GetLogicalDriveStringsW failed: {:?}",
                windows::core::Error::from_thread().code()
            );
            return Vec::new();
        }

        // `GetLogicalDriveStringsW` returns a multi-string.
        // '\0' terminates a string; an additional '\0' terminates the list.
        buffer[..(len as usize)]
            .split_inclusive(|w| *w == 0)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_vec())
            .collect()
    }
}

fn os_string_from_zero_terminated(name: &[u16]) -> OsString {
    let len = name.iter().position(|&x| x == 0).unwrap_or(name.len());
    OsString::from_wide(&name[..len])
}

unsafe fn get_disk_kind(handle: &HandleWrapper) -> DiskKind {
    let query = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageDeviceProperty,
        QueryType: PropertyStandardQuery,
        AdditionalParameters: [0],
    };
    let mut device_descriptor: STORAGE_DEVICE_DESCRIPTOR = unsafe { std::mem::zeroed() };
    let mut dw_size = 0;

    let ret = unsafe {
        DeviceIoControl(
            handle.0,
            IOCTL_STORAGE_QUERY_PROPERTY,
            Some(&query as *const STORAGE_PROPERTY_QUERY as *const _),
            size_of::<STORAGE_PROPERTY_QUERY>() as _,
            Some(&mut device_descriptor as *mut STORAGE_DEVICE_DESCRIPTOR as *mut _),
            size_of::<STORAGE_DEVICE_DESCRIPTOR>() as _,
            Some(&mut dw_size),
            None,
        )
        .is_ok()
    };

    if ret && device_descriptor.BusType == BusTypeFileBackedVirtual {
        return DiskKind::DiskImage;
    }

    let query = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageDeviceSeekPenaltyProperty,
        QueryType: PropertyStandardQuery,
        AdditionalParameters: [0],
    };
    let mut seek_penalty_descriptor: DEVICE_SEEK_PENALTY_DESCRIPTOR = unsafe { std::mem::zeroed() };
    let mut dw_size = 0;

    let ret = unsafe {
        DeviceIoControl(
            handle.0,
            IOCTL_STORAGE_QUERY_PROPERTY,
            Some(&query as *const STORAGE_PROPERTY_QUERY as *const _),
            size_of::<STORAGE_PROPERTY_QUERY>() as _,
            Some(&mut seek_penalty_descriptor as *mut DEVICE_SEEK_PENALTY_DESCRIPTOR as *mut _),
            size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() as _,
            Some(&mut dw_size),
            None,
        )
        .is_ok()
    };

    if ret && dw_size == size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() as u32 {
        if seek_penalty_descriptor.IncursSeekPenalty {
            DiskKind::HDD
        } else {
            DiskKind::SSD
        }
    } else {
        DiskKind::Unknown(-1)
    }
}

/// Returns a tuple consisting of the total number of bytes read and written by the volume with the
/// specified device path
fn get_disk_io(handle: HandleWrapper) -> Option<(u64, u64)> {
    let mut disk_perf = DISK_PERFORMANCE::default();
    let mut bytes_returned = 0;

    // SAFETY: the handle is checked for validity above
    unsafe {
        // See <https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ni-winioctl-ioctl_disk_performance> for reference
        DeviceIoControl(
            handle.0,
            IOCTL_DISK_PERFORMANCE,
            None, // Must be None as per docs
            0,
            Some(&mut disk_perf as *mut _ as _),
            size_of::<DISK_PERFORMANCE>() as u32,
            Some(&mut bytes_returned),
            None,
        )
    }
    .inspect_err(|_err| {
        sysinfo_debug!(
            "Error: DeviceIoControl(IOCTL_DISK_PERFORMANCE) = {:?}",
            _err
        );
    })
    .ok()?;

    Some((
        disk_perf.BytesRead.try_into().ok()?,
        disk_perf.BytesWritten.try_into().ok()?,
    ))
}
