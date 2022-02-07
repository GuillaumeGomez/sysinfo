// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{DiskExt, DiskType, DiskUsageExt};

use std::{ffi::OsStr, path::Path};

#[doc = include_str!("../../md_doc/disk.md")]
pub struct Disk {}

impl DiskExt for Disk {
    fn type_(&self) -> DiskType {
        unreachable!()
    }

    fn name(&self) -> &OsStr {
        unreachable!()
    }

    fn file_system(&self) -> &[u8] {
        &[]
    }

    fn mount_point(&self) -> &Path {
        Path::new("")
    }

    fn total_space(&self) -> u64 {
        0
    }

    fn available_space(&self) -> u64 {
        0
    }

    fn is_removable(&self) -> bool {
        false
    }

    fn usage(&self) -> DiskUsageExt {
        DiskUsageExt {
            written_bytes: 0,
            total_written_bytes: 0,
            read_bytes: 0,
            total_read_bytes: 0,
            written_ops: 0,
            total_written_ops: 0,
            read_ops: 0,
            total_read_ops: 0,
        }
    }

    fn refresh_usage(&mut self) -> bool {
        true
    }

    fn refresh(&mut self) -> bool {
        true
    }
}
