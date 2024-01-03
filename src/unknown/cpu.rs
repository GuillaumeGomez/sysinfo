// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::OsStr;

pub(crate) struct CpuInner;

impl CpuInner {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn cpu_usage(&self) -> f32 {
        0.0
    }

    pub(crate) fn name(&self) -> &OsStr {
        OsStr::new("")
    }

    pub(crate) fn frequency(&self) -> u64 {
        0
    }

    pub(crate) fn vendor_id(&self) -> &OsStr {
        OsStr::new("")
    }

    pub(crate) fn brand(&self) -> &OsStr {
        OsStr::new("")
    }
}
