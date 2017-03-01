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
#[no_mangle]
pub extern "C" fn sysinfo_get_processors_usage(length: *mut c_uint) -> *const c_float {
    static mut PROCS: *mut Vec<c_float> = 0 as *mut Vec<c_float>;

    if length.is_null() {
        return ::std::ptr::null();
    }
    unsafe {
        if PROCS.is_null() {
            PROCS = libc::malloc(::std::mem::size_of::<Vec<c_float>>()) as *mut Vec<c_float>;
            *PROCS = Vec::new();
        }
    }
    let processors = unsafe { (*get_system()).get_processor_list() };
    if unsafe { (*PROCS).is_empty() } {
        for processor in processors.iter().skip(1) {
            unsafe { (*PROCS).push(processor.get_cpu_usage()); }
        }
    } else {
        for (pos, processor) in processors.iter().skip(1).enumerate() {
            unsafe { (*PROCS)[pos] = processor.get_cpu_usage(); }
        }
    }
    unsafe {
        *length = (*PROCS).len() as c_uint;
        (*PROCS).as_ptr()
    }
}
