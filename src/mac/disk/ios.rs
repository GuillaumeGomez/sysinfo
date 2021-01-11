//
// Sysinfo
//
// Copyright (c) 2021 Guillaume Gomez
//

use Disk;
use sys::ffi;

pub(crate) fn get_disks(_: ffi::DASessionRef) -> Vec<Disk> {
    Vec::new()
}
