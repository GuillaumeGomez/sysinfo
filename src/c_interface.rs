//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use libc::{self, c_uint, c_float, size_t};
use ::{ProcessorExt, System, SystemExt};

fn get_system<'a>() -> *mut System {
    static mut SYSTEM: *mut System = 0 as *mut System;

    unsafe {
        if SYSTEM.is_null() {
            SYSTEM = libc::malloc(::std::mem::size_of::<System>()) as *mut System;
            *SYSTEM = System::new();
        }
        SYSTEM
    }
}

/// Equivalent of `System.refresh_system()`.
#[no_mangle]
pub extern "C" fn sysinfo_refresh_system() {
    unsafe { (*get_system()).refresh_system(); }
}

/// Equivalent of `System.get_total_memory()`.
#[no_mangle]
pub extern "C" fn sysinfo_get_total_memory() -> size_t {
    unsafe { (*get_system()).get_total_memory() as size_t }
}

/// Equivalent of `System.get_free_memory()`.
#[no_mangle]
pub extern "C" fn sysinfo_get_free_memory() -> size_t {
    unsafe { (*get_system()).get_free_memory() as size_t }
}

/// Equivalent of `System.get_used_memory()`.
#[no_mangle]
pub extern "C" fn sysinfo_get_used_memory() -> size_t {
    unsafe { (*get_system()).get_used_memory() as size_t }
}

/// Equivalent of `System.get_total_swap()`.
#[no_mangle]
pub extern "C" fn sysinfo_get_total_swap() -> size_t {
    unsafe { (*get_system()).get_total_swap() as size_t }
}

/// Equivalent of `System.get_free_swap()`.
#[no_mangle]
pub extern "C" fn sysinfo_get_free_swap() -> size_t {
    unsafe { (*get_system()).get_free_swap() as size_t }
}

/// Equivalent of `System.get_used_swap()`.
#[no_mangle]
pub extern "C" fn sysinfo_get_used_swap() -> size_t {
    unsafe { (*get_system()).get_used_swap() as size_t }
}

/// Equivalent of `System.get_processors_usage()`.
///
/// * `length` will contain the number of cpu usage added into `procs`.
/// * `procs` will be allocated if it's null and will contain of cpu usage.
#[no_mangle]
pub extern "C" fn sysinfo_get_processors_usage(length: *mut c_uint,
                                               procs: *mut *mut c_float) {
    if procs.is_null() || length.is_null() {
        return;
    }
    let processors = unsafe { (*get_system()).get_processor_list() };
    unsafe {
        if (*procs).is_null() {
            (*procs) = libc::malloc(::std::mem::size_of::<c_float>() * processors.len()) as *mut c_float;
        }
        for (pos, processor) in processors.iter().skip(1).enumerate() {
            (*(*procs).offset(pos as isize)) = processor.get_cpu_usage();
        }
        *length = processors.len() as c_uint - 1;
    }
}
