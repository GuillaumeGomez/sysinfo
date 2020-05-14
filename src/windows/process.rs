//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

use std::fmt::{self, Debug};
use std::mem::{size_of, zeroed, MaybeUninit};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process;
use std::ptr::null_mut;
use std::str;

use libc::{c_void, memcpy};

use once_cell::sync::Lazy;

use DiskUsage;
use Pid;
use ProcessExt;

use ntapi::ntpsapi::{
    NtQueryInformationProcess, ProcessBasicInformation, ProcessCommandLineInformation,
    PROCESSINFOCLASS, PROCESS_BASIC_INFORMATION,
};
use ntapi::ntrtl::RtlGetVersion;
use winapi::shared::minwindef::{DWORD, FALSE, FILETIME, MAX_PATH, TRUE, ULONG};
use winapi::shared::ntdef::{NT_SUCCESS, UNICODE_STRING};
use winapi::shared::ntstatus::{
    STATUS_BUFFER_OVERFLOW, STATUS_BUFFER_TOO_SMALL, STATUS_INFO_LENGTH_MISMATCH,
};
use winapi::um::handleapi::CloseHandle;
use winapi::um::processthreadsapi::{GetProcessTimes, OpenProcess};
use winapi::um::psapi::{
    EnumProcessModulesEx, GetModuleBaseNameW, GetModuleFileNameExW, GetProcessMemoryInfo,
    LIST_MODULES_ALL, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
};
use winapi::um::sysinfoapi::GetSystemTimeAsFileTime;
use winapi::um::winbase::GetProcessIoCounters;
use winapi::um::winnt::{
    HANDLE, IO_COUNTERS, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ, RTL_OSVERSIONINFOEXW,
    ULARGE_INTEGER,
};

/// Enum describing the different status of a process.
#[derive(Clone, Copy, Debug)]
pub enum ProcessStatus {
    /// Currently runnable.
    Run,
}

impl ProcessStatus {
    /// Used to display `ProcessStatus`.
    pub fn to_string(&self) -> &str {
        match *self {
            ProcessStatus::Run => "Runnable",
        }
    }
}

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

fn get_process_handler(pid: Pid) -> Option<HANDLE> {
    if pid == 0 {
        return None;
    }
    let options = PROCESS_QUERY_INFORMATION | PROCESS_VM_READ;
    let process_handler = unsafe { OpenProcess(options, FALSE, pid as DWORD) };
    if process_handler.is_null() {
        None
    } else {
        Some(process_handler)
    }
}

#[derive(Clone)]
struct PtrWrapper<T: Clone>(T);

impl<T: Clone> Deref for PtrWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl<T: Clone> Send for PtrWrapper<T> {}
unsafe impl<T: Clone> Sync for PtrWrapper<T> {}

/// Struct containing a process' information.
pub struct Process {
    name: String,
    cmd: Vec<String>,
    exe: PathBuf,
    pid: Pid,
    environ: Vec<String>,
    cwd: PathBuf,
    root: PathBuf,
    pub(crate) memory: u64,
    pub(crate) virtual_memory: u64,
    parent: Option<Pid>,
    status: ProcessStatus,
    handle: PtrWrapper<HANDLE>,
    old_cpu: u64,
    old_sys_cpu: u64,
    old_user_cpu: u64,
    start_time: u64,
    cpu_usage: f32,
    pub(crate) updated: bool,
    old_read_bytes: u64,
    old_written_bytes: u64,
    read_bytes: u64,
    written_bytes: u64,
}

static WINDOWS_8_1_OR_NEWER: Lazy<bool> = Lazy::new(|| {
    let mut version_info: RTL_OSVERSIONINFOEXW = unsafe { MaybeUninit::zeroed().assume_init() };

    version_info.dwOSVersionInfoSize = std::mem::size_of::<RTL_OSVERSIONINFOEXW>() as u32;
    if !NT_SUCCESS(unsafe {
        RtlGetVersion(&mut version_info as *mut RTL_OSVERSIONINFOEXW as *mut _)
    }) {
        return true;
    }

    // Windows 8.1 is 6.3
    version_info.dwMajorVersion > 6
        || version_info.dwMajorVersion == 6 && version_info.dwMinorVersion >= 3
});

unsafe fn get_process_name(process_handler: HANDLE, h_mod: *mut c_void) -> String {
    let mut process_name = [0u16; MAX_PATH + 1];

    GetModuleBaseNameW(
        process_handler,
        h_mod as _,
        process_name.as_mut_ptr(),
        MAX_PATH as DWORD + 1,
    );
    let mut pos = 0;
    for x in process_name.iter() {
        if *x == 0 {
            break;
        }
        pos += 1;
    }
    String::from_utf16_lossy(&process_name[..pos])
}

unsafe fn get_h_mod(process_handler: HANDLE, h_mod: &mut *mut c_void) -> bool {
    let mut cb_needed = 0;
    EnumProcessModulesEx(
        process_handler,
        h_mod as *mut *mut c_void as _,
        size_of::<DWORD>() as DWORD,
        &mut cb_needed,
        LIST_MODULES_ALL,
    ) != 0
}

unsafe fn get_exe(process_handler: HANDLE, h_mod: *mut c_void) -> PathBuf {
    let mut exe_buf = [0u16; MAX_PATH + 1];
    GetModuleFileNameExW(
        process_handler,
        h_mod as _,
        exe_buf.as_mut_ptr(),
        MAX_PATH as DWORD + 1,
    );

    let mut pos = 0;
    for x in exe_buf.iter() {
        if *x == 0 {
            break;
        }
        pos += 1;
    }

    PathBuf::from(String::from_utf16_lossy(&exe_buf[..pos]))
}

impl Process {
    #[allow(clippy::uninit_assumed_init)]
    pub(crate) fn new_from_pid(pid: Pid) -> Option<Process> {
        let process_handler = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, pid as _) };
        if process_handler.is_null() {
            return None;
        }
        let mut info: PROCESS_BASIC_INFORMATION = unsafe { MaybeUninit::uninit().assume_init() };
        if unsafe {
            NtQueryInformationProcess(
                process_handler,
                ProcessBasicInformation,
                &mut info as *mut _ as *mut _,
                size_of::<PROCESS_BASIC_INFORMATION>() as _,
                null_mut(),
            )
        } != 0
        {
            unsafe { CloseHandle(process_handler) };
            return None;
        }
        Some(Process::new_with_handle(
            pid,
            if info.InheritedFromUniqueProcessId as usize != 0 {
                Some(info.InheritedFromUniqueProcessId as usize)
            } else {
                None
            },
            process_handler,
        ))
    }

    pub(crate) fn new_full(
        pid: Pid,
        parent: Option<Pid>,
        memory: u64,
        virtual_memory: u64,
        name: String,
    ) -> Process {
        if let Some(handle) = get_process_handler(pid) {
            let mut h_mod = null_mut();
            unsafe { get_h_mod(handle, &mut h_mod) };
            let environ = unsafe { get_proc_env(handle, pid as u32, &name) };

            let exe = unsafe { get_exe(handle, h_mod) };
            let mut root = exe.clone();
            root.pop();
            Process {
                handle: PtrWrapper(handle),
                name,
                pid,
                parent,
                cmd: get_cmd_line(handle),
                environ,
                exe,
                cwd: PathBuf::new(),
                root,
                status: ProcessStatus::Run,
                memory,
                virtual_memory,
                cpu_usage: 0.,
                old_cpu: 0,
                old_sys_cpu: 0,
                old_user_cpu: 0,
                start_time: unsafe { get_start_time(handle) },
                updated: true,
                old_read_bytes: 0,
                old_written_bytes: 0,
                read_bytes: 0,
                written_bytes: 0,
            }
        } else {
            Process {
                handle: PtrWrapper(null_mut()),
                name,
                pid,
                parent,
                cmd: Vec::new(),
                environ: Vec::new(),
                exe: get_executable_path(pid),
                cwd: PathBuf::new(),
                root: PathBuf::new(),
                status: ProcessStatus::Run,
                memory,
                virtual_memory,
                cpu_usage: 0.,
                old_cpu: 0,
                old_sys_cpu: 0,
                old_user_cpu: 0,
                start_time: 0,
                updated: true,
                old_read_bytes: 0,
                old_written_bytes: 0,
                read_bytes: 0,
                written_bytes: 0,
            }
        }
    }

    fn new_with_handle(pid: Pid, parent: Option<Pid>, process_handler: HANDLE) -> Process {
        let mut h_mod = null_mut();

        unsafe {
            let name = if get_h_mod(process_handler, &mut h_mod) {
                get_process_name(process_handler, h_mod)
            } else {
                String::new()
            };
            let environ = get_proc_env(process_handler, pid as u32, &name);

            let exe = get_exe(process_handler, h_mod);
            let mut root = exe.clone();
            root.pop();
            Process {
                handle: PtrWrapper(process_handler),
                name,
                pid,
                parent,
                cmd: get_cmd_line(process_handler),
                environ,
                exe,
                cwd: PathBuf::new(),
                root,
                status: ProcessStatus::Run,
                memory: 0,
                virtual_memory: 0,
                cpu_usage: 0.,
                old_cpu: 0,
                old_sys_cpu: 0,
                old_user_cpu: 0,
                start_time: get_start_time(process_handler),
                updated: true,
                old_read_bytes: 0,
                old_written_bytes: 0,
                read_bytes: 0,
                written_bytes: 0,
            }
        }
    }
}

// TODO: it's possible to get environment variables like it's done in
// https://github.com/processhacker/processhacker
//
// They have a very nice function called PhGetProcessEnvironment. Just very complicated as it
// seems...
impl ProcessExt for Process {
    fn new(pid: Pid, parent: Option<Pid>, _: u64) -> Process {
        if let Some(process_handler) = get_process_handler(pid) {
            Process::new_with_handle(pid, parent, process_handler)
        } else {
            Process {
                handle: PtrWrapper(null_mut()),
                name: String::new(),
                pid,
                parent,
                cmd: Vec::new(),
                environ: Vec::new(),
                exe: get_executable_path(pid),
                cwd: PathBuf::new(),
                root: PathBuf::new(),
                status: ProcessStatus::Run,
                memory: 0,
                virtual_memory: 0,
                cpu_usage: 0.,
                old_cpu: 0,
                old_sys_cpu: 0,
                old_user_cpu: 0,
                start_time: 0,
                updated: true,
                old_read_bytes: 0,
                old_written_bytes: 0,
                read_bytes: 0,
                written_bytes: 0,
            }
        }
    }

    fn kill(&self, _signal: ::Signal) -> bool {
        let mut kill = process::Command::new("taskkill.exe");
        kill.arg("/PID").arg(self.pid().to_string()).arg("/F");
        match kill.output() {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn cmd(&self) -> &[String] {
        &self.cmd
    }

    fn exe(&self) -> &Path {
        self.exe.as_path()
    }

    fn pid(&self) -> Pid {
        self.pid
    }

    fn environ(&self) -> &[String] {
        &self.environ
    }

    fn cwd(&self) -> &Path {
        self.cwd.as_path()
    }

    fn root(&self) -> &Path {
        self.root.as_path()
    }

    fn memory(&self) -> u64 {
        self.memory
    }

    fn virtual_memory(&self) -> u64 {
        self.virtual_memory
    }

    fn parent(&self) -> Option<Pid> {
        self.parent
    }

    fn status(&self) -> ProcessStatus {
        self.status
    }

    fn start_time(&self) -> u64 {
        self.start_time
    }

    fn cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    fn disk_usage(&self) -> DiskUsage {
        DiskUsage {
            written_bytes: self.written_bytes - self.old_written_bytes,
            total_written_bytes: self.written_bytes,
            read_bytes: self.read_bytes - self.old_read_bytes,
            total_read_bytes: self.read_bytes,
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            if self.handle.is_null() {
                return;
            }
            CloseHandle(*self.handle);
        }
    }
}

unsafe fn get_start_time(handle: HANDLE) -> u64 {
    let mut fstart: FILETIME = zeroed();
    let mut x = zeroed();

    GetProcessTimes(
        handle,
        &mut fstart as *mut FILETIME,
        &mut x as *mut FILETIME,
        &mut x as *mut FILETIME,
        &mut x as *mut FILETIME,
    );
    let tmp = (fstart.dwHighDateTime as u64) << 32 | (fstart.dwLowDateTime as u64);
    tmp / 10_000_000 - 11_644_473_600
}

unsafe fn ph_query_process_variable_size(
    process_handle: HANDLE,
    process_information_class: PROCESSINFOCLASS,
) -> Option<Vec<u16>> {
    let mut return_length = MaybeUninit::<ULONG>::uninit();

    let mut status = NtQueryInformationProcess(
        process_handle,
        process_information_class,
        std::ptr::null_mut(),
        0,
        return_length.as_mut_ptr() as *mut _,
    );

    if status != STATUS_BUFFER_OVERFLOW
        && status != STATUS_BUFFER_TOO_SMALL
        && status != STATUS_INFO_LENGTH_MISMATCH
    {
        return None;
    }

    let mut return_length = return_length.assume_init();
    let buf_len = (return_length as usize) / 2;
    let mut buffer: Vec<u16> = Vec::with_capacity(buf_len + 1);
    buffer.set_len(buf_len);

    status = NtQueryInformationProcess(
        process_handle,
        process_information_class,
        buffer.as_mut_ptr() as *mut _,
        return_length,
        &mut return_length as *mut _,
    );
    if !NT_SUCCESS(status) {
        return None;
    }
    buffer.push(0);
    return Some(buffer);
}

unsafe fn get_cmdline_from_buffer(buffer: *const u16) -> Vec<String> {
    // Get argc and argv from the command line
    let mut argc = MaybeUninit::<i32>::uninit();
    let argv_p = winapi::um::shellapi::CommandLineToArgvW(buffer, argc.as_mut_ptr());
    if argv_p.is_null() {
        return Vec::new();
    }
    let argc = argc.assume_init();
    let argv = std::slice::from_raw_parts(argv_p, argc as usize);

    let mut res = Vec::new();
    for arg in argv {
        let len = libc::wcslen(*arg);
        let str_slice = std::slice::from_raw_parts(*arg, len);
        res.push(String::from_utf16_lossy(str_slice));
    }

    winapi::um::winbase::LocalFree(argv_p as *mut _);

    res
}

fn get_cmd_line_old(handle: HANDLE) -> Vec<String> {
    use ntapi::ntpebteb::{PEB, PPEB};
    use ntapi::ntrtl::{PRTL_USER_PROCESS_PARAMETERS, RTL_USER_PROCESS_PARAMETERS};
    use winapi::shared::basetsd::SIZE_T;
    use winapi::um::memoryapi::ReadProcessMemory;

    unsafe {
        let mut pinfo = MaybeUninit::<PROCESS_BASIC_INFORMATION>::uninit();
        if NtQueryInformationProcess(
            handle,
            0, // ProcessBasicInformation
            pinfo.as_mut_ptr() as *mut _,
            size_of::<PROCESS_BASIC_INFORMATION>() as u32,
            null_mut(),
        ) != 0
        {
            return Vec::new();
        }
        let pinfo = pinfo.assume_init();

        let ppeb: PPEB = pinfo.PebBaseAddress;
        let mut peb_copy = MaybeUninit::<PEB>::uninit();
        if ReadProcessMemory(
            handle,
            ppeb as *mut _,
            peb_copy.as_mut_ptr() as *mut _,
            size_of::<PEB>() as SIZE_T,
            ::std::ptr::null_mut(),
        ) != TRUE
        {
            return Vec::new();
        }
        let peb_copy = peb_copy.assume_init();

        let proc_param = peb_copy.ProcessParameters;
        let mut rtl_proc_param_copy = MaybeUninit::<RTL_USER_PROCESS_PARAMETERS>::uninit();
        if ReadProcessMemory(
            handle,
            proc_param as *mut PRTL_USER_PROCESS_PARAMETERS as *mut _,
            rtl_proc_param_copy.as_mut_ptr() as *mut _,
            size_of::<RTL_USER_PROCESS_PARAMETERS>() as SIZE_T,
            ::std::ptr::null_mut(),
        ) != TRUE
        {
            return Vec::new();
        }
        let rtl_proc_param_copy = rtl_proc_param_copy.assume_init();

        let len = rtl_proc_param_copy.CommandLine.Length as usize;
        let len = len / 2;

        // For len symbols + '\0'
        let mut buffer_copy: Vec<u16> = Vec::with_capacity(len + 1);
        buffer_copy.set_len(len);
        if ReadProcessMemory(
            handle,
            rtl_proc_param_copy.CommandLine.Buffer as *mut _,
            buffer_copy.as_mut_ptr() as *mut _,
            len * 2 as SIZE_T,
            ::std::ptr::null_mut(),
        ) != TRUE
        {
            return Vec::new();
        }
        buffer_copy.push(0);

        get_cmdline_from_buffer(buffer_copy.as_ptr())
    }
}

#[allow(clippy::cast_ptr_alignment)]
fn get_cmd_line_new(handle: HANDLE) -> Vec<String> {
    unsafe {
        if let Some(buffer) = ph_query_process_variable_size(handle, ProcessCommandLineInformation)
        {
            let buffer = (*(buffer.as_ptr() as *const UNICODE_STRING)).Buffer;

            get_cmdline_from_buffer(buffer)
        } else {
            vec![]
        }
    }
}

fn get_cmd_line(handle: HANDLE) -> Vec<String> {
    if *WINDOWS_8_1_OR_NEWER {
        get_cmd_line_new(handle)
    } else {
        get_cmd_line_old(handle)
    }
}

unsafe fn get_proc_env(_handle: HANDLE, _pid: u32, _name: &str) -> Vec<String> {
    let ret = Vec::new();
    /*
    println!("current pid: {}", kernel32::GetCurrentProcessId());
    if kernel32::GetCurrentProcessId() == pid {
        println!("current proc!");
        for (key, value) in env::vars() {
            ret.push(format!("{}={}", key, value));
        }
        return ret;
    }
    println!("1");
    let snapshot_handle = kernel32::CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
    if !snapshot_handle.is_null() {
        println!("2");
        let mut target_thread: THREADENTRY32 = zeroed();
        target_thread.dwSize = size_of::<THREADENTRY32>() as DWORD;
        if kernel32::Thread32First(snapshot_handle, &mut target_thread) == TRUE {
            println!("3");
            loop {
                if target_thread.th32OwnerProcessID == pid {
                    println!("4");
                    let thread_handle = kernel32::OpenThread(THREAD_SUSPEND_RESUME | THREAD_QUERY_INFORMATION | THREAD_GET_CONTEXT,
                                                             FALSE,
                                                             target_thread.th32ThreadID);
                    if !thread_handle.is_null() {
                        println!("5 -> {}", pid);
                        if kernel32::SuspendThread(thread_handle) != DWORD::max_value() {
                            println!("6");
                            let mut context = zeroed();
                            if kernel32::GetThreadContext(thread_handle, &mut context) != 0 {
                                println!("7 --> {:?}", context);
                                let mut x = vec![0u8; 10];
                                if kernel32::ReadProcessMemory(handle,
                                                               context.MxCsr as usize as *mut winapi::c_void,
                                                               x.as_mut_ptr() as *mut winapi::c_void,
                                                               x.len() as u64,
                                                               null_mut()) != 0 {
                                    for y in x {
                                        print!("{}", y as char);
                                    }
                                    println!("");
                                } else {
                                    println!("failure... {:?}", kernel32::GetLastError());
                                }
                            } else {
                                println!("-> {:?}", kernel32::GetLastError());
                            }
                            kernel32::ResumeThread(thread_handle);
                        }
                        kernel32::CloseHandle(thread_handle);
                    }
                    break;
                }
                if kernel32::Thread32Next(snapshot_handle, &mut target_thread) != TRUE {
                    break;
                }
            }
        }
        kernel32::CloseHandle(snapshot_handle);
    }*/
    ret
}

pub(crate) fn get_executable_path(_pid: Pid) -> PathBuf {
    /*let where_req = format!("ProcessId={}", pid);

    if let Some(ret) = run_wmi(&["process", "where", &where_req, "get", "ExecutablePath"]) {
        for line in ret.lines() {
            if line.is_empty() || line == "ExecutablePath" {
                continue
            }
            return line.to_owned();
        }
    }*/
    PathBuf::new()
}

pub(crate) fn get_system_computation_time() -> ULARGE_INTEGER {
    unsafe {
        let mut now: ULARGE_INTEGER = ::std::mem::zeroed();
        let mut ftime: FILETIME = zeroed();

        GetSystemTimeAsFileTime(&mut ftime);
        memcpy(
            &mut now as *mut ULARGE_INTEGER as *mut c_void,
            &mut ftime as *mut FILETIME as *mut c_void,
            size_of::<FILETIME>(),
        );
        now
    }
}

#[inline]
fn check_sub(a: u64, b: u64) -> u64 {
    if a < b {
        a
    } else {
        a - b
    }
}

pub(crate) fn compute_cpu_usage(p: &mut Process, nb_processors: u64, now: ULARGE_INTEGER) {
    unsafe {
        let mut sys: ULARGE_INTEGER = ::std::mem::zeroed();
        let mut user: ULARGE_INTEGER = ::std::mem::zeroed();
        let mut ftime: FILETIME = zeroed();
        let mut fsys: FILETIME = zeroed();
        let mut fuser: FILETIME = zeroed();

        GetProcessTimes(
            *p.handle,
            &mut ftime as *mut FILETIME,
            &mut ftime as *mut FILETIME,
            &mut fsys as *mut FILETIME,
            &mut fuser as *mut FILETIME,
        );
        memcpy(
            &mut sys as *mut ULARGE_INTEGER as *mut c_void,
            &mut fsys as *mut FILETIME as *mut c_void,
            size_of::<FILETIME>(),
        );
        memcpy(
            &mut user as *mut ULARGE_INTEGER as *mut c_void,
            &mut fuser as *mut FILETIME as *mut c_void,
            size_of::<FILETIME>(),
        );
        p.cpu_usage = (check_sub(*sys.QuadPart(), p.old_sys_cpu) as f32
            + check_sub(*user.QuadPart(), p.old_user_cpu) as f32)
            / check_sub(*now.QuadPart(), p.old_cpu) as f32
            / nb_processors as f32
            * 100.;
        p.old_cpu = *now.QuadPart();
        p.old_user_cpu = *user.QuadPart();
        p.old_sys_cpu = *sys.QuadPart();
    }
}

pub fn get_handle(p: &Process) -> HANDLE {
    *p.handle
}

pub fn update_disk_usage(p: &mut Process) {
    let mut counters = MaybeUninit::<IO_COUNTERS>::uninit();
    let ret = unsafe { GetProcessIoCounters(*p.handle, counters.as_mut_ptr()) };
    if ret == 0 {
        sysinfo_debug!("GetProcessIoCounters call failed on process {}", p.pid());
    } else {
        let counters = unsafe { counters.assume_init() };
        p.old_read_bytes = p.read_bytes;
        p.old_written_bytes = p.written_bytes;
        p.read_bytes = counters.ReadTransferCount;
        p.written_bytes = counters.WriteTransferCount;
    }
}

pub fn update_memory(p: &mut Process) {
    unsafe {
        let mut pmc: PROCESS_MEMORY_COUNTERS_EX = zeroed();
        if GetProcessMemoryInfo(
            *p.handle,
            &mut pmc as *mut PROCESS_MEMORY_COUNTERS_EX as *mut c_void
                as *mut PROCESS_MEMORY_COUNTERS,
            size_of::<PROCESS_MEMORY_COUNTERS_EX>() as DWORD,
        ) != 0
        {
            p.memory = (pmc.WorkingSetSize as u64) / 1_000;
            p.virtual_memory = (pmc.PrivateUsage as u64) / 1_000;
        }
    }
}
