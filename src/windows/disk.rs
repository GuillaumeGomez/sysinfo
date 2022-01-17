// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{DiskExt, DiskType, DiskUsage};

use std::ffi::{OsStr, OsString};
use std::path::Path;

use ntapi::ntrtl::RtlGetVersion;
use winapi::shared::minwindef::FALSE;
use winapi::shared::winerror::ERROR_MORE_DATA;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::fileapi::GetDiskFreeSpaceExW;
use winapi::um::ioapiset::DeviceIoControl;
use winapi::um::winbase::FILE_FLAG_BACKUP_SEMANTICS;
use winapi::um::winnt::{RTL_OSVERSIONINFOEXW, ULARGE_INTEGER};
use winapi::um::handleapi::{INVALID_HANDLE_VALUE, CloseHandle};
use winapi::shared::ntdef::NT_SUCCESS;
use once_cell::sync::Lazy;

pub(crate) fn new_disk(
    disk_idx: u16,
    name: &OsStr,
    mount_point: &[u16],
    file_system: &[u8],
    type_: DiskType,
    total_space: u64,
    is_removable: bool,
) -> Option<Disk> {
    if total_space == 0 {
        return None;
    }
    let mut d = Disk {
        disk_idx,
        type_,
        name: name.to_owned(),
        file_system: file_system.to_vec(),
        mount_point: mount_point.to_vec(),
        s_mount_point: String::from_utf16_lossy(&mount_point[..mount_point.len() - 1]),
        total_space,
        available_space: 0,
        is_removable,
    
        old_read_bytes: 0,
        old_written_bytes: 0,
        read_bytes: 0,
        written_bytes: 0,
        old_read_ops: 0,
        old_written_ops: 0,
        read_ops: 0,
        written_ops: 0,
    };
    d.refresh();
    Some(d)
}

#[doc = include_str!("../../md_doc/disk.md")]
pub struct Disk {
    disk_idx: u16,

    type_: DiskType,
    name: OsString,
    file_system: Vec<u8>,
    mount_point: Vec<u16>,
    s_mount_point: String,
    total_space: u64,
    available_space: u64,
    is_removable: bool,
    
    old_read_bytes: u64,
    old_written_bytes: u64,
    read_bytes: u64,
    written_bytes: u64,

    old_read_ops: u64,
    old_written_ops: u64,
    read_ops: u64,
    written_ops: u64
}

impl Disk {
    // Yes, seriously, for whatever reason, the underlying APIs for getting I/O statistics requires the leading backslash
    fn stats_id(&self) -> [u16; 8] {
        [
            b'\\' as u16,
            b'\\' as u16,
            b'.' as u16,
            b'\\' as u16,
            b'A' as u16 + self.disk_idx as u16,
            b':' as u16,
            b'\\' as u16,
            0,
        ]
    }
}
impl DiskExt for Disk {
    fn type_(&self) -> DiskType {
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

    fn usage(&self) -> DiskUsage {
        DiskUsage {
            written_bytes: self.written_bytes - self.old_written_bytes,
            total_written_bytes: self.written_bytes,
            read_bytes: self.read_bytes - self.old_read_bytes,
            total_read_bytes: self.read_bytes,
        }
    }

    fn refresh_usage(&mut self) -> bool {
        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Ioctl/constant.FSCTL_FILESYSTEM_GET_STATISTICS_EX.html
        const FSCTL_FILESYSTEM_GET_STATISTICS_EX: u32 = 590732u32;

        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Ioctl/type.FILESYSTEM_STATISTICS_TYPE.html
        type FileSystemStatisticsType = u16;

        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Ioctl/struct.FILESYSTEM_STATISTICS.html
        #[repr(C)]
        #[derive(Debug)]
        #[allow(non_snake_case)]
        pub struct FILESYSTEM_STATISTICS {
            pub FileSystemType: FileSystemStatisticsType,
            pub Version: u16,
            pub SizeOfCompleteStructure: u32,
            pub UserFileReads: u32,
            pub UserFileReadBytes: u32,
            pub UserDiskReads: u32,
            pub UserFileWrites: u32,
            pub UserFileWriteBytes: u32,
            pub UserDiskWrites: u32,
            pub MetaDataReads: u32,
            pub MetaDataReadBytes: u32,
            pub MetaDataDiskReads: u32,
            pub MetaDataWrites: u32,
            pub MetaDataWriteBytes: u32,
            pub MetaDataDiskWrites: u32,
        }

        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Ioctl/struct.FILESYSTEM_STATISTICS_EX.html
        #[repr(C)]
        #[derive(Debug)]
        #[allow(non_snake_case)]
        pub struct FILESYSTEM_STATISTICS_EX {
            pub FileSystemType: FileSystemStatisticsType,
            pub Version: u16,
            pub SizeOfCompleteStructure: u32,
            pub UserFileReads: u64,
            pub UserFileReadBytes: u64,
            pub UserDiskReads: u64,
            pub UserFileWrites: u64,
            pub UserFileWriteBytes: u64,
            pub UserDiskWrites: u64,
            pub MetaDataReads: u64,
            pub MetaDataReadBytes: u64,
            pub MetaDataDiskReads: u64,
            pub MetaDataWrites: u64,
            pub MetaDataWriteBytes: u64,
            pub MetaDataDiskWrites: u64,
        }

        static WINDOWS_10_OR_NEWER: Lazy<bool> = Lazy::new(|| {
            let mut version_info: RTL_OSVERSIONINFOEXW = unsafe { std::mem::zeroed() };
        
            version_info.dwOSVersionInfoSize = std::mem::size_of::<RTL_OSVERSIONINFOEXW>() as u32;
            if !NT_SUCCESS(unsafe {
                RtlGetVersion(&mut version_info as *mut RTL_OSVERSIONINFOEXW as *mut _)
            }) {
                return true;
            }
        
            version_info.dwMajorVersion >= 10
        });

        macro_rules! update_stats {
            ($buffer:ident) => {
                self.old_written_bytes = self.written_bytes;
                self.old_read_bytes = self.read_bytes;
                self.old_written_ops = self.written_ops;
                self.old_read_ops = self.read_ops;
                self.written_bytes = ($buffer.UserFileWriteBytes + $buffer.MetaDataWriteBytes) as u64;
                self.read_bytes = ($buffer.UserFileReadBytes + $buffer.MetaDataReadBytes) as u64;
                self.written_ops = ($buffer.UserFileWrites + $buffer.MetaDataWrites) as u64;
                self.read_ops = ($buffer.UserFileReads + $buffer.MetaDataReads) as u64;
            }
        }

        unsafe {
            let handle = super::tools::open_drive(&self.stats_id(), 0, FILE_FLAG_BACKUP_SEMANTICS);
            if handle == INVALID_HANDLE_VALUE {
                CloseHandle(handle);
                return false;
            }

            if *WINDOWS_10_OR_NEWER {
                let mut buffer: FILESYSTEM_STATISTICS_EX = std::mem::zeroed();
                if DeviceIoControl(
                    handle,
                    FSCTL_FILESYSTEM_GET_STATISTICS_EX,
                    std::ptr::null_mut(),
                    0,
                    &mut buffer as *mut _ as *mut _,
                    std::mem::size_of::<FILESYSTEM_STATISTICS_EX>() as _,
                    &mut 0,
                    std::ptr::null_mut()
                ) == FALSE {
                    // Many drivers/filesystems will return a bit more data, but we can safely ignore it
                    if GetLastError() != ERROR_MORE_DATA {
                        return false;
                    }
                }
                update_stats!(buffer);
            } else {
                let mut buffer: FILESYSTEM_STATISTICS = std::mem::zeroed();
                if DeviceIoControl(
                    handle,
                    winapi::um::winioctl::FSCTL_FILESYSTEM_GET_STATISTICS,
                    std::ptr::null_mut(),
                    0,
                    &mut buffer as *mut _ as *mut _,
                    std::mem::size_of::<FILESYSTEM_STATISTICS>() as _,
                    &mut 0,
                    std::ptr::null_mut()
                ) == FALSE {
                    // Many drivers/filesystems will return a bit more data, but we can safely ignore it
                    if GetLastError() != ERROR_MORE_DATA {
                        return false;
                    }
                }
                update_stats!(buffer);
            }
        }

        true
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
