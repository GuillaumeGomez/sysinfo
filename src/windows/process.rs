//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

use std::fmt::{self, Formatter, Debug};
use std::mem::{size_of, zeroed};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::str;

use libc::{c_uint, c_void, memcpy};

use Pid;
use ProcessExt;

use winapi::shared::minwindef::{DWORD, FALSE, FILETIME, MAX_PATH/*, TRUE, USHORT*/};
use winapi::um::handleapi::CloseHandle;
use winapi::um::winnt::{
    HANDLE, ULARGE_INTEGER, /*THREAD_GET_CONTEXT, THREAD_QUERY_INFORMATION, THREAD_SUSPEND_RESUME,*/
    /*, PWSTR*/ PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE, PROCESS_VM_READ,
};
use winapi::um::processthreadsapi::{GetProcessTimes, OpenProcess, TerminateProcess};
use winapi::um::psapi::{
    GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
    EnumProcessModulesEx, GetModuleBaseNameW, GetModuleFileNameExW, LIST_MODULES_ALL,
};
use winapi::um::sysinfoapi::GetSystemTimeAsFileTime;
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
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
    let options = PROCESS_QUERY_INFORMATION | PROCESS_VM_READ | PROCESS_TERMINATE;
    let process_handler = unsafe { OpenProcess(options, FALSE, pid as DWORD) };
    if process_handler.is_null() {
        let options = PROCESS_QUERY_INFORMATION | PROCESS_VM_READ;
        let process_handler = unsafe { OpenProcess(options, FALSE, pid as DWORD) };
        if process_handler.is_null() {
            None
        } else {
            Some(process_handler)
        }
    } else {
        Some(process_handler)
    }
}

#[derive(Clone)]
struct HandleWrapper(HANDLE);

impl Deref for HandleWrapper {
    type Target = HANDLE;

    fn deref(&self) -> &HANDLE {
        &self.0
    }
}

unsafe impl Send for HandleWrapper {}
unsafe impl Sync for HandleWrapper {}

/// Struct containing a process' information.
#[derive(Clone)]
pub struct Process {
    name: String,
    cmd: Vec<String>,
    exe: PathBuf,
    pid: Pid,
    environ: Vec<String>,
    cwd: PathBuf,
    root: PathBuf,
    memory: u64,
    parent: Option<Pid>,
    status: ProcessStatus,
    handle: HandleWrapper,
    old_cpu: u64,
    old_sys_cpu: u64,
    old_user_cpu: u64,
    start_time: u64,
    cpu_usage: f32,
}

// TODO: it's possible to get environment variables like it's done in
// https://github.com/processhacker/processhacker
//
// They have a very nice function called PhGetProcessEnvironment. Just very complicated as it
// seems...
impl ProcessExt for Process {
    fn new(pid: Pid, parent: Option<Pid>, _: u64) -> Process {
        if let Some(process_handler) = get_process_handler(pid) {
            let mut h_mod = ::std::ptr::null_mut();
            let mut process_name = [0u16; MAX_PATH + 1];
            let mut cb_needed = 0;

            unsafe {
                if EnumProcessModulesEx(process_handler,
                                        &mut h_mod,
                                        ::std::mem::size_of::<DWORD>() as DWORD,
                                        &mut cb_needed,
                                        LIST_MODULES_ALL) != 0 {
                    GetModuleBaseNameW(process_handler,
                                       h_mod,
                                       process_name.as_mut_ptr(),
                                       MAX_PATH as DWORD + 1);
                }
                let mut pos = 0;
                for x in process_name.iter() {
                    if *x == 0 {
                        break
                    }
                    pos += 1;
                }
                let name = String::from_utf16_lossy(&process_name[..pos]);
                let environ = get_proc_env(process_handler, pid as u32, &name);

                let mut exe_buf = [0u16; MAX_PATH + 1];
                GetModuleFileNameExW(process_handler,
                                     h_mod,
                                     exe_buf.as_mut_ptr(),
                                     MAX_PATH as DWORD + 1);

                pos = 0;
                for x in exe_buf.iter() {
                    if *x == 0 {
                        break
                    }
                    pos += 1;
                }

                let exe = PathBuf::from(String::from_utf16_lossy(&exe_buf[..pos]));
                let mut root = exe.clone();
                root.pop();
                Process {
                    handle: HandleWrapper(process_handler),
                    name: name,
                    pid: pid,
                    parent: parent,
                    cmd: get_cmd_line(pid),
                    environ: environ,
                    exe: exe,
                    cwd: PathBuf::new(),
                    root: root,
                    status: ProcessStatus::Run,
                    memory: 0,
                    cpu_usage: 0.,
                    old_cpu: 0,
                    old_sys_cpu: 0,
                    old_user_cpu: 0,
                    start_time: get_start_time(process_handler),
                }
            }
        } else {
            Process {
                handle: HandleWrapper(::std::ptr::null_mut()),
                name: String::new(),
                pid: pid,
                parent: parent,
                cmd: get_cmd_line(pid),
                environ: Vec::new(),
                exe: get_executable_path(pid),
                cwd: PathBuf::new(),
                root: PathBuf::new(),
                status: ProcessStatus::Run,
                memory: 0,
                cpu_usage: 0.,
                old_cpu: 0,
                old_sys_cpu: 0,
                old_user_cpu: 0,
                start_time: 0,
            }
        }
    }

    fn kill(&self, signal: ::Signal) -> bool {
        if self.handle.is_null() {
            false
        } else {
            unsafe { TerminateProcess(*self.handle, signal as c_uint) != 0 }
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
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            if self.handle.is_null() {
                return
            }
            CloseHandle(*self.handle);
        }
    }
}

#[allow(unused_must_use)]
impl Debug for Process {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        writeln!(f, "pid: {}", self.pid);
        writeln!(f, "name: {}", self.name);
        writeln!(f, "environment:");
        for var in self.environ.iter() {
        if var.len() > 0 {
                writeln!(f, "\t{}", var);
            }
        }
        writeln!(f, "command:");
        for arg in &self.cmd {
            writeln!(f, "\t{}", arg);
        }
        writeln!(f, "executable path: {:?}", self.exe);
        writeln!(f, "current working directory: {:?}", self.cwd);
        writeln!(f, "memory usage: {} kB", self.memory);
        writeln!(f, "cpu usage: {}", self.cpu_usage);
        writeln!(f, "root path: {:?}", self.root)
    }
}

unsafe fn get_start_time(handle: HANDLE) -> u64 {
    let mut fstart: FILETIME = zeroed();
    let mut x = zeroed();

    GetProcessTimes(handle,
                    &mut fstart as *mut FILETIME,
                    &mut x as *mut FILETIME,
                    &mut x as *mut FILETIME,
                    &mut x as *mut FILETIME);
    let tmp = (fstart.dwHighDateTime as u64) << 32 | (fstart.dwLowDateTime as u64);
    tmp / 10_000_000 - 11_644_473_600
}

pub unsafe fn get_parent_process_id(pid: Pid) -> Option<Pid> {
    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    let mut entry: PROCESSENTRY32 = zeroed();
    entry.dwSize = size_of::<PROCESSENTRY32>() as u32;
    let mut not_the_end = Process32First(snapshot, &mut entry);
    while not_the_end != 0 {
        if pid == entry.th32ProcessID as usize {
            // TODO: if some day I have the motivation to add threads:
            // ListProcessThreads(entry.th32ProcessID);
            CloseHandle(snapshot);
            return Some(entry.th32ParentProcessID as usize);
        }
        not_the_end = Process32Next(snapshot, &mut entry);
    }
    CloseHandle(snapshot);
    None
}

/*fn run_wmi(args: &[&str]) -> Option<String> {
    use std::process::Command;

    if let Ok(out) = Command::new("wmic")
                             .args(args)
                             .output() {
        if out.status.success() {
            return Some(String::from_utf8_lossy(&out.stdout).into_owned());
        }
    }
    None
}*/

fn get_cmd_line(_pid: Pid) -> Vec<String> {
    /*let where_req = format!("ProcessId={}", pid);

    if let Some(ret) = run_wmi(&["process", "where", &where_req, "get", "CommandLine"]) {
        for line in ret.lines() {
            if line.is_empty() || line == "CommandLine" {
                continue
            }
            return vec![line.to_owned()];
        }
    }*/
    Vec::new()
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
                                                               ::std::ptr::null_mut()) != 0 {
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

pub fn get_executable_path(_pid: Pid) -> PathBuf {
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

pub fn compute_cpu_usage(p: &mut Process, nb_processors: u64) {
    unsafe {
        let mut now: ULARGE_INTEGER = ::std::mem::zeroed();
        let mut sys: ULARGE_INTEGER = ::std::mem::zeroed();
        let mut user: ULARGE_INTEGER = ::std::mem::zeroed();
        let mut ftime: FILETIME = zeroed();
        let mut fsys: FILETIME = zeroed();
        let mut fuser: FILETIME = zeroed();

        GetSystemTimeAsFileTime(&mut ftime);
        memcpy(&mut now as *mut ULARGE_INTEGER as *mut c_void,
               &mut ftime as *mut FILETIME as *mut c_void,
               size_of::<FILETIME>());

        GetProcessTimes(*p.handle,
                        &mut ftime as *mut FILETIME,
                        &mut ftime as *mut FILETIME,
                        &mut fsys as *mut FILETIME,
                        &mut fuser as *mut FILETIME);
        memcpy(&mut sys as *mut ULARGE_INTEGER as *mut c_void,
               &mut fsys as *mut FILETIME as *mut c_void,
               size_of::<FILETIME>());
        memcpy(&mut user as *mut ULARGE_INTEGER as *mut c_void,
               &mut fuser as *mut FILETIME as *mut c_void,
               size_of::<FILETIME>());
        p.cpu_usage = ((*sys.QuadPart() - p.old_sys_cpu) as f32 +
            (*user.QuadPart() - p.old_user_cpu) as f32)
            / (*now.QuadPart() - p.old_cpu) as f32
            / nb_processors as f32 * 100.;
        p.old_cpu = *now.QuadPart();
        p.old_user_cpu = *user.QuadPart();
        p.old_sys_cpu = *sys.QuadPart();
    }
}

pub fn get_handle(p: &Process) -> HANDLE {
    *p.handle
}

pub fn update_proc_info(p: &mut Process) {
    update_memory(p);
}

pub fn update_memory(p: &mut Process) {
    unsafe {
        let mut pmc: PROCESS_MEMORY_COUNTERS_EX = zeroed();
        if GetProcessMemoryInfo(*p.handle,
                                &mut pmc as *mut PROCESS_MEMORY_COUNTERS_EX as *mut c_void as *mut PROCESS_MEMORY_COUNTERS,
                                size_of::<PROCESS_MEMORY_COUNTERS_EX>() as DWORD) != 0 {
            p.memory = (pmc.PrivateUsage as u64) >> 10u64; // / 1024;
        }
    }
}
