// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{Disk, DiskKind};

use std::ffi::{c_void, OsStr, OsString};
use std::mem::size_of;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE,
    ERROR_MORE_DATA, ERROR_NO_MORE_FILES, MAX_PATH};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, GetDiskFreeSpaceExW, GetDriveTypeW, GetVolumeInformationW,
    FILE_ACCESS_RIGHTS, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    GetVolumePathNamesForVolumeNameW, FindVolumeClose, FindNextVolumeW, FindFirstVolumeW};
use windows::Win32::System::Ioctl::{
    PropertyStandardQuery, StorageDeviceSeekPenaltyProperty, DEVICE_SEEK_PENALTY_DESCRIPTOR,
    IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_PROPERTY_QUERY,
};
use windows::Win32::System::WindowsProgramming::{DRIVE_FIXED, DRIVE_REMOVABLE};
use windows::Win32::System::IO::DeviceIoControl;

/// Creates a copy of the first zero-terminated wide string in `buf`.
/// The copy includes the zero terminator.
fn from_zero_terminated(buf: &[u16]) -> Vec<u16> {
    let end = buf.iter().position(|&x| x == 0).unwrap_or(buf.len());
    buf[..=end].to_vec()
}

// Realistically, volume names are probably not longer than 44 characters,
// but the example in the Microsoft documentation uses MAX_PATH as well.
// https://learn.microsoft.com/en-us/windows/win32/fileio/displaying-volume-paths
const VOLUME_NAME_SIZE: usize = MAX_PATH as usize + 1;

/// Returns a list of zero-terminated wide strings containing volume GUID paths.
/// Volume GUID paths have the form `\\?\{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}\`.
///
/// Rather confusingly, the Win32 API _also_ calls these "volume names".
pub(crate) fn get_volume_guid_paths() -> Vec<Vec<u16>> {
    let mut volume_names = Vec::new();
    unsafe {
        let mut buf = Box::new([0u16; VOLUME_NAME_SIZE]);
        let handle = match FindFirstVolumeW(&mut buf[..]) {
            Ok(handle) => handle,
            Err(_) => {
                sysinfo_debug!("Error: FindFirstVolumeW() = {:?}", GetLastError());
                return Vec::new();
            }
        };
        volume_names.push(from_zero_terminated(&buf[..]));
        loop {
            match FindNextVolumeW(handle, &mut buf[..]) {
                Ok(_) => (),
                Err(_) => {
                    let find_next_err = GetLastError()
                        .expect_err("GetLastError should return an error after FindNextVolumeW returned zero.");
                    if find_next_err.code() != ERROR_NO_MORE_FILES.to_hresult() {
                        sysinfo_debug!("Error: FindNextVolumeW = {}", find_next_err);
                    }
                    break;
                }
            }
            volume_names.push(from_zero_terminated(&buf[..]));
        }
        if FindVolumeClose(handle) != Ok(()) {
            sysinfo_debug!("Error: FindVolumeClose = {:?}", GetLastError());
        };
    }
    volume_names
}

/// Given a volume GUID path (`\\?\{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}\`), returns all
/// volume paths (drive letters and mount paths) associated with it
/// as zero terminated wide strings.
///
/// # Safety
/// `volume_name` must contain a zero-terminated wide string.
pub(crate) unsafe fn get_volume_path_names_for_volume_name(volume_guid_path: &[u16]) -> Vec<Vec<u16>> {
    let volume_guid_path = PCWSTR::from_raw(volume_guid_path.as_ptr());

    // Initial buffer size is just a guess. There is no direct connection between MAX_PATH
    // the output of GetVolumePathNamesForVolumeNameW.
    let mut path_names_buf = vec![0u16; MAX_PATH as usize];
    let mut path_names_output_size = 0u32;
    for _ in 0..10 {
        match GetVolumePathNamesForVolumeNameW(
            volume_guid_path,
            Some(path_names_buf.as_mut_slice()),
            &mut path_names_output_size)
            .map_err(|_| GetLastError()
                .expect_err("GetLastError should return an error after GetVolumePathNamesForVolumeNameW returned zero.")
                .code()) {
            Ok(()) => break,
            Err(e) if e == ERROR_MORE_DATA.to_hresult() => {
                // We need a bigger buffer. path_names_output_size contains the required buffer size.
                path_names_buf = vec![0u16; path_names_output_size as usize];
                continue;
            },
            Err(_e) => {
                sysinfo_debug!("Error: GetVolumePathNamesForVolumeNameW() = {}", _e);
                return Vec::new();
            }
        }
    }

    // path_names_buf contains multiple zero terminated wide strings.
    // An additional zero terminates the list.
    let mut path_names = Vec::new();
    let mut buf = &path_names_buf[..];
    while buf.len() > 0 && buf[0] != 0 {
        let path = from_zero_terminated(buf);
        buf = &buf[path.len()..];
        path_names.push(path);
    }
    path_names
}

pub(crate) struct DiskInner {
    type_: DiskKind,
    name: OsString,
    file_system: Vec<u8>,
    mount_point: Vec<u16>,
    s_mount_point: OsString,
    total_space: u64,
    available_space: u64,
    is_removable: bool,
}

impl DiskInner {
    pub(crate) fn kind(&self) -> DiskKind {
        self.type_
    }

    pub(crate) fn name(&self) -> &OsStr {
        &self.name
    }

    pub(crate) fn file_system(&self) -> &[u8] {
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

    pub(crate) fn refresh(&mut self) -> bool {
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
            self.disks = get_list();
        }
    }

    pub(crate) fn list(&self) -> &[Disk] {
        &self.disks
    }

    pub(crate) fn list_mut(&mut self) -> &mut [Disk] {
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

pub(crate) unsafe fn get_list() -> Vec<Disk> {

    #[cfg(feature = "multithread")]
    use rayon::iter::ParallelIterator;

    crate::utils::into_iter(get_volume_guid_paths())
        .flat_map(|volume_name| {
            let raw_volume_name = PCWSTR::from_raw(volume_name.as_ptr());
            let drive_type = GetDriveTypeW(raw_volume_name);

            let is_removable = drive_type == DRIVE_REMOVABLE;

            if drive_type != DRIVE_FIXED && drive_type != DRIVE_REMOVABLE {
                return vec![];
            }
            let mut name = [0u16; MAX_PATH as usize + 1];
            let mut file_system = [0u16; 32];
            let volume_info_res = GetVolumeInformationW(
                raw_volume_name,
                Some(&mut name),
                None,
                None,
                None,
                Some(&mut file_system),
            )
            .is_ok();
            if !volume_info_res {
                sysinfo_debug!("Error: GetVolumeInformationW = {:?}", GetLastError());
                return vec![];
            }

            let mount_paths = get_volume_path_names_for_volume_name(&volume_name[..]);
            if mount_paths.len() == 0 {
                return vec![];
            }

            // The device path is the volume name without the trailing backslash.
            let device_path = volume_name[..(volume_name.len()-2)].iter().copied().chain([0]).collect::<Vec<_>>();
            let handle = match HandleWrapper::new(&device_path[..], Default::default()) {
                Some(h) => h,
                None => {
                    return vec![];
                }
            };
            let (total_space, available_space) = match get_drive_size(&mount_paths[0][..]) {
                Some(space) => space,
                None => {
                    return vec![];
                }
            };
            if total_space == 0 {
                sysinfo_debug!("total_space == 0");
                return vec![];
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

            let name_len = name.iter().position(|&x| x == 0).unwrap_or(name.len());
            let name = OsString::from_wide(&name[..name_len]);
            let file_system = file_system.iter()
                .take_while(|c| **c != 0)
                .map(|c| *c as u8)
                .collect::<Vec<_>>();
            mount_paths.into_iter().map(move |mount_path| Disk {
                inner: DiskInner {
                    type_,
                    name: name.clone(),
                    file_system: file_system.clone(),
                    s_mount_point: OsString::from_wide(&mount_path[..mount_path.len()-1]),
                    mount_point: mount_path,
                    total_space,
                    available_space,
                    is_removable,
                },
            }).collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}
