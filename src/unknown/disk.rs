// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{DiskExt, DiskKind};

use std::{ffi::OsStr, path::Path};

#[doc = include_str!("../../md_doc/disk.md")]
pub struct Disk {}

impl DiskExt for Disk {
    fn kind(&self) -> DiskKind {
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

    fn refresh(&mut self) -> bool {
        true
    }
}
