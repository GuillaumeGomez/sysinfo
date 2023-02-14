// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{CpuExt, NetworkExt, NetworksExt, Pid, Process, ProcessExt, System, SystemExt};
use libc::{self, c_char, c_float, c_uint, c_void, pid_t, size_t};
use std::borrow::BorrowMut;
use std::ffi::CString;

/// Equivalent of [`System`][crate::System] struct.
pub type CSystem = *mut c_void;
/// Equivalent of [`Process`][crate::Process] struct.
pub type CProcess = *const c_void;
/// C string returned from `CString::into_raw`.
pub type RString = *const c_char;
/// Callback used by [`processes`][crate::System#method.processes].
pub type ProcessLoop = extern "C" fn(pid: pid_t, process: CProcess, data: *mut c_void) -> bool;

/// Equivalent of [`System::new()`][crate::System#method.new].
#[no_mangle]
pub extern "C" fn sysinfo_init() -> CSystem {
    let system = Box::new(System::new());
    Box::into_raw(system) as CSystem
}

/// Equivalent of `System::drop()`. Important in C to cleanup memory.
#[no_mangle]
pub extern "C" fn sysinfo_destroy(system: CSystem) {
    assert!(!system.is_null());
    unsafe {
        drop(Box::from_raw(system as *mut System));
    }
}

/// Equivalent of [`System::refresh_system()`][crate::System#method.refresh_system].
#[no_mangle]
pub extern "C" fn sysinfo_refresh_system(system: CSystem) {
    assert!(!system.is_null());
    unsafe {
        let mut system: Box<System> = Box::from_raw(system as *mut System);
        {
            let system: &mut System = system.borrow_mut();
            system.refresh_system();
        }
        Box::into_raw(system);
    }
}

/// Equivalent of [`System::refresh_memory()`][crate::System#method.refresh_memory].
#[no_mangle]
pub extern "C" fn sysinfo_refresh_memory(system: CSystem) {
    assert!(!system.is_null());
    unsafe {
        let mut system: Box<System> = Box::from_raw(system as *mut System);
        {
            let system: &mut System = system.borrow_mut();
            system.refresh_memory();
        }
        Box::into_raw(system);
    }
}

/// Equivalent of [`System::refresh_cpu()`][crate::System#method.refresh_cpu].
#[no_mangle]
pub extern "C" fn sysinfo_refresh_cpu(system: CSystem) {
    assert!(!system.is_null());
    unsafe {
        let mut system: Box<System> = Box::from_raw(system as *mut System);
        {
            let system: &mut System = system.borrow_mut();
            system.refresh_cpu();
        }
        Box::into_raw(system);
    }
}

/// Equivalent of [`System::refresh_components()`][crate::System#method.refresh_temperatures].
#[no_mangle]
pub extern "C" fn sysinfo_refresh_components(system: CSystem) {
    assert!(!system.is_null());
    unsafe {
        let mut system: Box<System> = Box::from_raw(system as *mut System);
        {
            let system: &mut System = system.borrow_mut();
            system.refresh_components();
        }
        Box::into_raw(system);
    }
}

/// Equivalent of [`System::refresh_all()`][crate::System#method.refresh_all].
#[no_mangle]
pub extern "C" fn sysinfo_refresh_all(system: CSystem) {
    assert!(!system.is_null());
    unsafe {
        let mut system: Box<System> = Box::from_raw(system as *mut System);
        {
            let system: &mut System = system.borrow_mut();
            system.refresh_all();
        }
        Box::into_raw(system);
    }
}

/// Equivalent of [`System::refresh_processes()`][crate::System#method.refresh_processes].
#[no_mangle]
pub extern "C" fn sysinfo_refresh_processes(system: CSystem) {
    assert!(!system.is_null());
    unsafe {
        let mut system: Box<System> = Box::from_raw(system as *mut System);
        {
            let system: &mut System = system.borrow_mut();
            system.refresh_processes();
        }
        Box::into_raw(system);
    }
}

/// Equivalent of [`System::refresh_process()`][crate::System#method.refresh_process].
#[cfg(target_os = "linux")]
#[no_mangle]
pub extern "C" fn sysinfo_refresh_process(system: CSystem, pid: pid_t) {
    assert!(!system.is_null());
    unsafe {
        let mut system: Box<System> = Box::from_raw(system as *mut System);
        {
            let system: &mut System = system.borrow_mut();
            system.refresh_process(Pid(pid));
        }
        Box::into_raw(system);
    }
}

/// Equivalent of [`System::refresh_disks()`][crate::System#method.refresh_disks].
#[no_mangle]
pub extern "C" fn sysinfo_refresh_disks(system: CSystem) {
    assert!(!system.is_null());
    unsafe {
        let mut system: Box<System> = Box::from_raw(system as *mut System);
        {
            let system: &mut System = system.borrow_mut();
            system.refresh_disks();
        }
        Box::into_raw(system);
    }
}

/// Equivalent of [`System::refresh_disks_list()`][crate::System#method.refresh_disks_list].
#[no_mangle]
pub extern "C" fn sysinfo_refresh_disks_list(system: CSystem) {
    assert!(!system.is_null());
    unsafe {
        let mut system: Box<System> = Box::from_raw(system as *mut System);
        {
            let system: &mut System = system.borrow_mut();
            system.refresh_disks_list();
        }
        Box::into_raw(system);
    }
}

/// Equivalent of [`System::total_memory()`][crate::System#method.total_memory].
#[no_mangle]
pub extern "C" fn sysinfo_total_memory(system: CSystem) -> size_t {
    assert!(!system.is_null());
    unsafe {
        let system: Box<System> = Box::from_raw(system as *mut System);
        let ret = system.total_memory() as size_t;
        Box::into_raw(system);
        ret
    }
}

/// Equivalent of [`System::free_memory()`][crate::System#method.free_memory].
#[no_mangle]
pub extern "C" fn sysinfo_free_memory(system: CSystem) -> size_t {
    assert!(!system.is_null());
    unsafe {
        let system: Box<System> = Box::from_raw(system as *mut System);
        let ret = system.free_memory() as size_t;
        Box::into_raw(system);
        ret
    }
}

/// Equivalent of [`System::used_memory()`][crate::System#method.used_memory].
#[no_mangle]
pub extern "C" fn sysinfo_used_memory(system: CSystem) -> size_t {
    assert!(!system.is_null());
    let system: Box<System> = unsafe { Box::from_raw(system as *mut System) };
    let ret = system.used_memory() as size_t;
    Box::into_raw(system);
    ret
}

/// Equivalent of [`System::total_swap()`][crate::System#method.total_swap].
#[no_mangle]
pub extern "C" fn sysinfo_total_swap(system: CSystem) -> size_t {
    assert!(!system.is_null());
    unsafe {
        let system: Box<System> = Box::from_raw(system as *mut System);
        let ret = system.total_swap() as size_t;
        Box::into_raw(system);
        ret
    }
}

/// Equivalent of [`System::free_swap()`][crate::System#method.free_swap].
#[no_mangle]
pub extern "C" fn sysinfo_free_swap(system: CSystem) -> size_t {
    assert!(!system.is_null());
    unsafe {
        let system: Box<System> = Box::from_raw(system as *mut System);
        let ret = system.free_swap() as size_t;
        Box::into_raw(system);
        ret
    }
}

/// Equivalent of [`System::used_swap()`][crate::System#method.used_swap].
#[no_mangle]
pub extern "C" fn sysinfo_used_swap(system: CSystem) -> size_t {
    assert!(!system.is_null());
    unsafe {
        let system: Box<System> = Box::from_raw(system as *mut System);
        let ret = system.used_swap() as size_t;
        Box::into_raw(system);
        ret
    }
}

/// Equivalent of
/// `system::networks().iter().fold(0, |acc, (_, data)| acc + data.received() as size_t)`.
#[no_mangle]
pub extern "C" fn sysinfo_networks_received(system: CSystem) -> size_t {
    assert!(!system.is_null());
    unsafe {
        let system: Box<System> = Box::from_raw(system as *mut System);
        let ret = system.networks().iter().fold(0, |acc: size_t, (_, data)| {
            acc.saturating_add(data.received() as size_t)
        });
        Box::into_raw(system);
        ret
    }
}

/// Equivalent of
/// `system::networks().iter().fold(0, |acc, (_, data)| acc + data.transmitted() as size_t)`.
#[no_mangle]
pub extern "C" fn sysinfo_networks_transmitted(system: CSystem) -> size_t {
    assert!(!system.is_null());
    unsafe {
        let system: Box<System> = Box::from_raw(system as *mut System);
        let ret = system.networks().iter().fold(0, |acc: size_t, (_, data)| {
            acc.saturating_add(data.transmitted() as size_t)
        });
        Box::into_raw(system);
        ret
    }
}

/// Equivalent of [`System::cpus_usage()`][crate::System#method.cpus_usage].
///
/// * `length` will contain the number of CPU usage added into `procs`.
/// * `procs` will be allocated if it's null and will contain of CPU usage.
#[no_mangle]
pub extern "C" fn sysinfo_cpus_usage(
    system: CSystem,
    length: *mut c_uint,
    procs: *mut *mut c_float,
) {
    assert!(!system.is_null());
    if procs.is_null() || length.is_null() {
        return;
    }
    unsafe {
        let system: Box<System> = Box::from_raw(system as *mut System);
        {
            let cpus = system.cpus();
            if (*procs).is_null() {
                (*procs) =
                    libc::malloc(::std::mem::size_of::<c_float>() * cpus.len()) as *mut c_float;
            }
            for (pos, cpu) in cpus.iter().skip(1).enumerate() {
                (*(*procs).offset(pos as isize)) = cpu.cpu_usage();
            }
            *length = cpus.len() as c_uint - 1;
        }
        Box::into_raw(system);
    }
}

/// Equivalent of [`System::processes()`][crate::System#method.processes]. Returns an
/// array ended by a null pointer. Must be freed.
///
/// # ⚠️ WARNING ⚠️
///
/// While having this method returned processes, you should *never* call any refresh method!
#[no_mangle]
pub extern "C" fn sysinfo_processes(
    system: CSystem,
    fn_pointer: Option<ProcessLoop>,
    data: *mut c_void,
) -> size_t {
    assert!(!system.is_null());
    if let Some(fn_pointer) = fn_pointer {
        unsafe {
            let system: Box<System> = Box::from_raw(system as *mut System);
            let len = {
                let entries = system.processes();
                for (pid, process) in entries {
                    if !fn_pointer(pid.0, process as *const Process as CProcess, data) {
                        break;
                    }
                }
                entries.len() as size_t
            };
            Box::into_raw(system);
            len
        }
    } else {
        0
    }
}

/// Equivalent of [`System::process()`][crate::System#method.process].
///
/// # ⚠️ WARNING ⚠️
///
/// While having this method returned process, you should *never* call any
/// refresh method!
#[no_mangle]
pub extern "C" fn sysinfo_process_by_pid(system: CSystem, pid: pid_t) -> CProcess {
    assert!(!system.is_null());
    unsafe {
        let system: Box<System> = Box::from_raw(system as *mut System);
        let ret = if let Some(process) = system.process(Pid(pid)) {
            process as *const Process as CProcess
        } else {
            std::ptr::null()
        };
        Box::into_raw(system);
        ret
    }
}

/// Equivalent of iterating over [`Process::tasks()`][crate::Process#method.tasks].
///
/// # ⚠️ WARNING ⚠️
///
/// While having this method processes, you should *never* call any refresh method!
#[cfg(target_os = "linux")]
#[no_mangle]
pub extern "C" fn sysinfo_process_tasks(
    process: CProcess,
    fn_pointer: Option<ProcessLoop>,
    data: *mut c_void,
) -> size_t {
    assert!(!process.is_null());
    if let Some(fn_pointer) = fn_pointer {
        unsafe {
            let process = process as *const Process;
            for (pid, process) in (*process).tasks.iter() {
                if !fn_pointer(pid.0, process as *const Process as CProcess, data) {
                    break;
                }
            }
            (*process).tasks.len() as size_t
        }
    } else {
        0
    }
}

/// Equivalent of [`Process::pid()`][crate::Process#method.pid].
#[no_mangle]
pub extern "C" fn sysinfo_process_pid(process: CProcess) -> pid_t {
    assert!(!process.is_null());
    let process = process as *const Process;
    unsafe { (*process).pid().0 }
}

/// Equivalent of [`Process::parent()`][crate::Process#method.parent].
///
/// In case there is no known parent, it returns `0`.
#[no_mangle]
pub extern "C" fn sysinfo_process_parent_pid(process: CProcess) -> pid_t {
    assert!(!process.is_null());
    let process = process as *const Process;
    unsafe { (*process).parent().unwrap_or(Pid(0)).0 }
}

/// Equivalent of [`Process::cpu_usage()`][crate::Process#method.cpu_usage].
#[no_mangle]
pub extern "C" fn sysinfo_process_cpu_usage(process: CProcess) -> c_float {
    assert!(!process.is_null());
    let process = process as *const Process;
    unsafe { (*process).cpu_usage() }
}

/// Equivalent of [`Process::memory()`][crate::Process#method.memory].
#[no_mangle]
pub extern "C" fn sysinfo_process_memory(process: CProcess) -> size_t {
    assert!(!process.is_null());
    let process = process as *const Process;
    unsafe { (*process).memory() as usize }
}

/// Equivalent of [`Process::virtual_memory()`][crate::Process#method.virtual_memory].
#[no_mangle]
pub extern "C" fn sysinfo_process_virtual_memory(process: CProcess) -> size_t {
    assert!(!process.is_null());
    let process = process as *const Process;
    unsafe { (*process).virtual_memory() as usize }
}

/// Equivalent of [`Process::exe()`][crate::Process#method.exe].
#[no_mangle]
pub extern "C" fn sysinfo_process_executable_path(process: CProcess) -> RString {
    assert!(!process.is_null());
    let process = process as *const Process;
    unsafe {
        if let Some(p) = (*process).exe().to_str() {
            if let Ok(c) = CString::new(p) {
                return c.into_raw() as _;
            }
        }
        std::ptr::null()
    }
}

/// Equivalent of [`Process::root()`][crate::Process#method.root].
#[no_mangle]
pub extern "C" fn sysinfo_process_root_directory(process: CProcess) -> RString {
    assert!(!process.is_null());
    let process = process as *const Process;
    unsafe {
        if let Some(p) = (*process).root().to_str() {
            if let Ok(c) = CString::new(p) {
                return c.into_raw() as _;
            }
        }
        std::ptr::null()
    }
}

/// Equivalent of [`Process::cwd()`][crate::Process#method.cwd].
#[no_mangle]
pub extern "C" fn sysinfo_process_current_directory(process: CProcess) -> RString {
    assert!(!process.is_null());
    let process = process as *const Process;
    unsafe {
        if let Some(p) = (*process).cwd().to_str() {
            if let Ok(c) = CString::new(p) {
                return c.into_raw() as _;
            }
        }
        std::ptr::null()
    }
}

/// Frees a C string created with `CString::into_raw()`.
#[no_mangle]
pub extern "C" fn sysinfo_rstring_free(s: RString) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s as usize as *mut _);
        }
    }
}
