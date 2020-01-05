//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use std::borrow::Borrow;
use std::ffi::OsStr;
use std::fmt::{self, Debug, Formatter};
use std::mem;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;

use libc::{c_int, c_void, gid_t, kill, size_t, uid_t};

use Pid;
use ProcessExt;

use sys::ffi;
use sys::system::Wrap;

/// Enum describing the different status of a process.
#[derive(Clone, Copy, Debug)]
pub enum ProcessStatus {
    /// Process being created by fork.
    Idle,
    /// Currently runnable.
    Run,
    /// Sleeping on an address.
    Sleep,
    /// Process debugging or suspension.
    Stop,
    /// Awaiting collection by parent.
    Zombie,
    /// Unknown.
    Unknown(u32),
}

impl From<u32> for ProcessStatus {
    fn from(status: u32) -> ProcessStatus {
        match status {
            1 => ProcessStatus::Idle,
            2 => ProcessStatus::Run,
            3 => ProcessStatus::Sleep,
            4 => ProcessStatus::Stop,
            5 => ProcessStatus::Zombie,
            x => ProcessStatus::Unknown(x),
        }
    }
}

impl ProcessStatus {
    /// Used to display `ProcessStatus`.
    pub fn to_string(&self) -> &str {
        match *self {
            ProcessStatus::Idle => "Idle",
            ProcessStatus::Run => "Runnable",
            ProcessStatus::Sleep => "Sleeping",
            ProcessStatus::Stop => "Stopped",
            ProcessStatus::Zombie => "Zombie",
            ProcessStatus::Unknown(_) => "Unknown",
        }
    }
}

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Enum describing the different status of a thread.
#[derive(Clone, Debug)]
pub enum ThreadStatus {
    /// Thread is running normally.
    Running,
    /// Thread is stopped.
    Stopped,
    /// Thread is waiting normally.
    Waiting,
    /// Thread is in an uninterruptible wait
    Uninterruptible,
    /// Thread is halted at a clean point.
    Halted,
    /// Unknown.
    Unknown(i32),
}

impl From<i32> for ThreadStatus {
    fn from(status: i32) -> ThreadStatus {
        match status {
            1 => ThreadStatus::Running,
            2 => ThreadStatus::Stopped,
            3 => ThreadStatus::Waiting,
            4 => ThreadStatus::Uninterruptible,
            5 => ThreadStatus::Halted,
            x => ThreadStatus::Unknown(x),
        }
    }
}

impl ThreadStatus {
    /// Used to display `ThreadStatus`.
    pub fn to_string(&self) -> &str {
        match *self {
            ThreadStatus::Running => "Running",
            ThreadStatus::Stopped => "Stopped",
            ThreadStatus::Waiting => "Waiting",
            ThreadStatus::Uninterruptible => "Uninterruptible",
            ThreadStatus::Halted => "Halted",
            ThreadStatus::Unknown(_) => "Unknown",
        }
    }
}

impl fmt::Display for ThreadStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Struct containing a process' information.
#[derive(Clone)]
pub struct Process {
    pub(crate) name: String,
    pub(crate) cmd: Vec<String>,
    pub(crate) exe: PathBuf,
    pid: Pid,
    parent: Option<Pid>,
    pub(crate) environ: Vec<String>,
    cwd: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) memory: u64,
    pub(crate) virtual_memory: u64,
    utime: u64,
    stime: u64,
    old_utime: u64,
    old_stime: u64,
    start_time: u64,
    updated: bool,
    cpu_usage: f32,
    /// User id of the process owner.
    pub uid: uid_t,
    /// Group id of the process owner.
    pub gid: gid_t,
    pub(crate) process_status: ProcessStatus,
    /// Status of process (running, stopped, waiting, etc). `None` means `sysinfo` doesn't have
    /// enough rights to get this information.
    ///
    /// This is very likely this one that you want instead of `process_status`.
    pub status: Option<ThreadStatus>,
}

impl Process {
    pub(crate) fn new_with(
        pid: Pid,
        parent: Option<Pid>,
        start_time: u64,
        exe: PathBuf,
        name: String,
        cmd: Vec<String>,
    ) -> Process {
        Process {
            name,
            pid,
            parent,
            cmd,
            environ: Vec::new(),
            exe,
            cwd: PathBuf::new(),
            root: PathBuf::new(),
            memory: 0,
            virtual_memory: 0,
            cpu_usage: 0.,
            utime: 0,
            stime: 0,
            old_utime: 0,
            old_stime: 0,
            updated: true,
            start_time,
            uid: 0,
            gid: 0,
            process_status: ProcessStatus::Unknown(0),
            status: None,
        }
    }

    pub(crate) fn new_with2(
        pid: Pid,
        parent: Option<Pid>,
        start_time: u64,
        exe: PathBuf,
        name: String,
        cmd: Vec<String>,
        environ: Vec<String>,
        root: PathBuf,
    ) -> Process {
        Process {
            name,
            pid,
            parent,
            cmd,
            environ,
            exe,
            cwd: PathBuf::new(),
            root,
            memory: 0,
            virtual_memory: 0,
            cpu_usage: 0.,
            utime: 0,
            stime: 0,
            old_utime: 0,
            old_stime: 0,
            updated: true,
            start_time,
            uid: 0,
            gid: 0,
            process_status: ProcessStatus::Unknown(0),
            status: None,
        }
    }
}

impl ProcessExt for Process {
    fn new(pid: Pid, parent: Option<Pid>, start_time: u64) -> Process {
        Process {
            name: String::new(),
            pid,
            parent,
            cmd: Vec::new(),
            environ: Vec::new(),
            exe: PathBuf::new(),
            cwd: PathBuf::new(),
            root: PathBuf::new(),
            memory: 0,
            virtual_memory: 0,
            cpu_usage: 0.,
            utime: 0,
            stime: 0,
            old_utime: 0,
            old_stime: 0,
            updated: true,
            start_time,
            uid: 0,
            gid: 0,
            process_status: ProcessStatus::Unknown(0),
            status: None,
        }
    }

    fn kill(&self, signal: ::Signal) -> bool {
        unsafe { kill(self.pid, signal as c_int) == 0 }
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
        self.process_status
    }

    fn start_time(&self) -> u64 {
        self.start_time
    }

    fn cpu_usage(&self) -> f32 {
        self.cpu_usage
    }
}

#[allow(unused_must_use)]
impl Debug for Process {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        writeln!(f, "pid: {}", self.pid);
        writeln!(f, "parent: {:?}", self.parent);
        writeln!(f, "name: {}", self.name);
        writeln!(f, "environment:");
        for var in &self.environ {
            if !var.is_empty() {
                writeln!(f, "\t{}", var);
            }
        }
        writeln!(f, "command:");
        for arg in &self.cmd {
            writeln!(f, "\t{}", arg);
        }
        writeln!(f, "executable path: {:?}", self.exe);
        writeln!(f, "current working directory: {:?}", self.cwd);
        writeln!(f, "owner/group: {}:{}", self.uid, self.gid);
        writeln!(f, "memory usage: {} kB", self.memory);
        writeln!(f, "virtual memory usage: {} kB", self.virtual_memory);
        writeln!(f, "cpu usage: {}%", self.cpu_usage);
        writeln!(
            f,
            "status: {}",
            match self.status {
                Some(ref v) => v.to_string(),
                None => "Unknown",
            }
        );
        write!(f, "root path: {:?}", self.root)
    }
}

pub(crate) fn compute_cpu_usage(p: &mut Process, time: u64, task_time: u64) {
    let system_time_delta = task_time - p.old_utime;
    let time_delta = time - p.old_stime;
    p.old_utime = task_time;
    p.old_stime = time;
    p.cpu_usage = if time_delta == 0 {
        0f32
    } else {
        (system_time_delta as f64 * 100f64 / time_delta as f64) as f32
    };
    p.updated = true;
}

/*pub fn set_time(p: &mut Process, utime: u64, stime: u64) {
    p.old_utime = p.utime;
    p.old_stime = p.stime;
    p.utime = utime;
    p.stime = stime;
    p.updated = true;
}*/

pub(crate) fn has_been_updated(p: &mut Process) -> bool {
    let old = p.updated;
    p.updated = false;
    old
}

pub(crate) fn force_update(p: &mut Process) {
    p.updated = true;
}

pub(crate) fn update_process(
    wrap: &Wrap,
    pid: Pid,
    taskallinfo_size: i32,
    taskinfo_size: i32,
    threadinfo_size: i32,
    mib: &mut [c_int],
    mut size: size_t,
) -> Result<Option<Process>, ()> {
    let mut proc_args = Vec::with_capacity(size as usize);
    unsafe {
        let mut thread_info = mem::zeroed::<libc::proc_threadinfo>();
        let (user_time, system_time, thread_status) = if ffi::proc_pidinfo(
            pid,
            libc::PROC_PIDTHREADINFO,
            0,
            &mut thread_info as *mut libc::proc_threadinfo as *mut c_void,
            threadinfo_size,
        ) != 0
        {
            (
                thread_info.pth_user_time,
                thread_info.pth_system_time,
                Some(ThreadStatus::from(thread_info.pth_run_state)),
            )
        } else {
            (0, 0, None)
        };
        if let Some(ref mut p) = (*wrap.0.get()).get_mut(&pid) {
            if p.memory == 0 {
                // We don't have access to this process' information.
                force_update(p);
                return Ok(None);
            }
            p.status = thread_status;
            let mut task_info = mem::zeroed::<libc::proc_taskinfo>();
            if ffi::proc_pidinfo(
                pid,
                libc::PROC_PIDTASKINFO,
                0,
                &mut task_info as *mut libc::proc_taskinfo as *mut c_void,
                taskinfo_size,
            ) != taskinfo_size
            {
                return Err(());
            }
            let task_time =
                user_time + system_time + task_info.pti_total_user + task_info.pti_total_system;
            let time = ffi::mach_absolute_time();
            compute_cpu_usage(p, time, task_time);

            p.memory = task_info.pti_resident_size >> 10; // divide by 1024
            p.virtual_memory = task_info.pti_virtual_size >> 10; // divide by 1024
            return Ok(None);
        }

        let mut task_info = mem::zeroed::<libc::proc_taskallinfo>();
        if ffi::proc_pidinfo(
            pid,
            libc::PROC_PIDTASKALLINFO,
            0,
            &mut task_info as *mut libc::proc_taskallinfo as *mut c_void,
            taskallinfo_size as i32,
        ) != taskallinfo_size as i32
        {
            match Command::new("/bin/ps") // not very nice, might be worth running a which first.
                .arg("wwwe")
                .arg("-o")
                .arg("ppid=,command=")
                .arg(pid.to_string().as_str())
                .output()
            {
                Ok(o) => {
                    let o = String::from_utf8(o.stdout).unwrap_or_else(|_| String::new());
                    let o = o.split(' ').filter(|c| !c.is_empty()).collect::<Vec<_>>();
                    if o.len() < 2 {
                        return Err(());
                    }
                    let mut command = parse_command_line(&o[1..]);
                    if let Some(ref mut x) = command.last_mut() {
                        **x = x.replace("\n", "");
                    }
                    let p = match i32::from_str_radix(&o[0].replace("\n", ""), 10) {
                        Ok(x) => x,
                        _ => return Err(()),
                    };
                    let exe = PathBuf::from(&command[0]);
                    let name = match exe.file_name() {
                        Some(x) => x.to_str().unwrap_or_else(|| "").to_owned(),
                        None => String::new(),
                    };
                    return Ok(Some(Process::new_with(
                        pid,
                        if p == 0 { None } else { Some(p) },
                        0,
                        exe,
                        name,
                        command,
                    )));
                }
                _ => {
                    return Err(());
                }
            }
        }

        let parent = match task_info.pbsd.pbi_ppid as Pid {
            0 => None,
            p => Some(p),
        };

        let ptr: *mut u8 = proc_args.as_mut_slice().as_mut_ptr();
        mib[0] = libc::CTL_KERN;
        mib[1] = libc::KERN_PROCARGS2;
        mib[2] = pid as c_int;
        /*
         * /---------------\ 0x00000000
         * | ::::::::::::: |
         * |---------------| <-- Beginning of data returned by sysctl() is here.
         * | argc          |
         * |---------------|
         * | exec_path     |
         * |---------------|
         * | 0             |
         * |---------------|
         * | arg[0]        |
         * |---------------|
         * | 0             |
         * |---------------|
         * | arg[n]        |
         * |---------------|
         * | 0             |
         * |---------------|
         * | env[0]        |
         * |---------------|
         * | 0             |
         * |---------------|
         * | env[n]        |
         * |---------------|
         * | ::::::::::::: |
         * |---------------| <-- Top of stack.
         * :               :
         * :               :
         * \---------------/ 0xffffffff
         */
        if libc::sysctl(
            mib.as_mut_ptr(),
            3,
            ptr as *mut c_void,
            &mut size,
            ::std::ptr::null_mut(),
            0,
        ) == -1
        {
            return Err(()); // not enough rights I assume?
        }
        let mut n_args: c_int = 0;
        libc::memcpy(
            (&mut n_args) as *mut c_int as *mut c_void,
            ptr as *const c_void,
            mem::size_of::<c_int>(),
        );

        let mut cp = ptr.add(mem::size_of::<c_int>());
        let mut start = cp;

        let mut p = if cp < ptr.add(size) {
            while cp < ptr.add(size) && *cp != 0 {
                cp = cp.offset(1);
            }
            let exe = Path::new(get_unchecked_str(cp, start).as_str()).to_path_buf();
            let name = exe
                .file_name()
                .unwrap_or_else(|| OsStr::new(""))
                .to_str()
                .unwrap_or_else(|| "")
                .to_owned();
            while cp < ptr.add(size) && *cp == 0 {
                cp = cp.offset(1);
            }
            start = cp;
            let mut c = 0;
            let mut cmd = Vec::with_capacity(n_args as usize);
            while c < n_args && cp < ptr.add(size) {
                if *cp == 0 {
                    c += 1;
                    cmd.push(get_unchecked_str(cp, start));
                    start = cp.offset(1);
                }
                cp = cp.offset(1);
            }

            #[inline]
            fn do_nothing(_: &str, _: &mut PathBuf, _: &mut bool) {}
            #[inline]
            fn do_something(env: &str, root: &mut PathBuf, check: &mut bool) {
                if *check && env.starts_with("PATH=") {
                    *check = false;
                    *root = Path::new(&env[6..]).to_path_buf();
                }
            }

            #[inline]
            unsafe fn get_environ<F: Fn(&str, &mut PathBuf, &mut bool)>(
                ptr: *mut u8,
                mut cp: *mut u8,
                size: size_t,
                mut root: PathBuf,
                callback: F,
            ) -> (Vec<String>, PathBuf) {
                let mut environ = Vec::with_capacity(10);
                let mut start = cp;
                let mut check = true;
                while cp < ptr.add(size) {
                    if *cp == 0 {
                        if cp == start {
                            break;
                        }
                        let e = get_unchecked_str(cp, start);
                        callback(&e, &mut root, &mut check);
                        environ.push(e);
                        start = cp.offset(1);
                    }
                    cp = cp.offset(1);
                }
                (environ, root)
            }

            let (environ, root) = if exe.is_absolute() {
                if let Some(parent) = exe.parent() {
                    get_environ(ptr, cp, size, parent.to_path_buf(), do_nothing)
                } else {
                    get_environ(ptr, cp, size, PathBuf::new(), do_something)
                }
            } else {
                get_environ(ptr, cp, size, PathBuf::new(), do_something)
            };

            Process::new_with2(
                pid,
                parent,
                task_info.pbsd.pbi_start_tvsec,
                exe,
                name,
                parse_command_line(&cmd),
                environ,
                root,
            )
        } else {
            Process::new(pid, parent, task_info.pbsd.pbi_start_tvsec)
        };

        p.memory = task_info.ptinfo.pti_resident_size >> 10; // divide by 1024
        p.virtual_memory = task_info.ptinfo.pti_virtual_size >> 10; // divide by 1024

        p.uid = task_info.pbsd.pbi_uid;
        p.gid = task_info.pbsd.pbi_gid;
        p.process_status = ProcessStatus::from(task_info.pbsd.pbi_status);

        Ok(Some(p))
    }
}

pub(crate) fn get_proc_list() -> Option<Vec<Pid>> {
    let count = unsafe { ffi::proc_listallpids(::std::ptr::null_mut(), 0) };
    if count < 1 {
        return None;
    }
    let mut pids: Vec<Pid> = Vec::with_capacity(count as usize);
    unsafe {
        pids.set_len(count as usize);
    }
    let count = count * mem::size_of::<Pid>() as i32;
    let x = unsafe { ffi::proc_listallpids(pids.as_mut_ptr() as *mut c_void, count) };

    if x < 1 || x as usize >= pids.len() {
        None
    } else {
        unsafe {
            pids.set_len(x as usize);
        }
        Some(pids)
    }
}

unsafe fn get_unchecked_str(cp: *mut u8, start: *mut u8) -> String {
    let len = cp as usize - start as usize;
    let part = Vec::from_raw_parts(start, len, len);
    let tmp = String::from_utf8_unchecked(part.clone());
    mem::forget(part);
    tmp
}

fn parse_command_line<T: Deref<Target = str> + Borrow<str>>(cmd: &[T]) -> Vec<String> {
    let mut x = 0;
    let mut command = Vec::with_capacity(cmd.len());
    while x < cmd.len() {
        let mut y = x;
        if cmd[y].starts_with('\'') || cmd[y].starts_with('"') {
            let c = if cmd[y].starts_with('\'') { '\'' } else { '"' };
            while y < cmd.len() && !cmd[y].ends_with(c) {
                y += 1;
            }
            command.push(cmd[x..y].join(" "));
            x = y;
        } else {
            command.push(cmd[x].to_owned());
        }
        x += 1;
    }
    command
}
