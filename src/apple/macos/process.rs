//
// Sysinfo
//
// Copyright (c) 2021 Guillaume Gomez
//

use std::mem::{self, MaybeUninit};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use std::borrow::Borrow;

use libc::{c_int, c_void, gid_t, kill, size_t, uid_t};

use crate::{DiskUsage, Pid, ProcessExt, ProcessStatus, Signal};

use crate::sys::ffi;
use crate::sys::process::ThreadStatus;
use crate::sys::system::Wrap;

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
    pub(crate) old_read_bytes: u64,
    pub(crate) old_written_bytes: u64,
    pub(crate) read_bytes: u64,
    pub(crate) written_bytes: u64,
}

impl Process {
    pub(crate) fn new_empty(pid: Pid, exe: PathBuf, name: String) -> Process {
        Process {
            name,
            pid,
            parent: None,
            cmd: Vec::new(),
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
            start_time: 0,
            uid: 0,
            gid: 0,
            process_status: ProcessStatus::Unknown(0),
            status: None,
            old_read_bytes: 0,
            old_written_bytes: 0,
            read_bytes: 0,
            written_bytes: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_with(
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
            old_read_bytes: 0,
            old_written_bytes: 0,
            read_bytes: 0,
            written_bytes: 0,
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
            old_read_bytes: 0,
            old_written_bytes: 0,
            read_bytes: 0,
            written_bytes: 0,
        }
    }

    fn kill(&self, signal: Signal) -> bool {
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

    fn disk_usage(&self) -> DiskUsage {
        DiskUsage {
            read_bytes: self.read_bytes - self.old_read_bytes,
            total_read_bytes: self.read_bytes,
            written_bytes: self.written_bytes - self.old_written_bytes,
            total_written_bytes: self.written_bytes,
        }
    }
}

pub(crate) fn compute_cpu_usage(
    p: &mut Process,
    task_info: libc::proc_taskinfo,
    system_time: u64,
    user_time: u64,
    time_interval: Option<f64>,
) {
    if let Some(time_interval) = time_interval {
        let total_existing_time = p.old_stime + p.old_utime;
        if time_interval > 0.000001 && total_existing_time > 0 {
            let total_current_time = task_info.pti_total_system + task_info.pti_total_user;

            let total_time_diff = total_current_time.saturating_sub(total_existing_time);
            p.cpu_usage = (total_time_diff as f64 / time_interval * 100.) as f32;
        } else {
            p.cpu_usage = 0.;
        }
        p.old_stime = task_info.pti_total_system;
        p.old_utime = task_info.pti_total_user;
    } else {
        // This is the "backup way" of CPU computation.
        let time = unsafe { ffi::mach_absolute_time() };
        let task_time =
            user_time + system_time + task_info.pti_total_user + task_info.pti_total_system;

        let system_time_delta = if task_time < p.old_utime {
            task_time
        } else {
            task_time - p.old_utime
        };
        let time_delta = if time < p.old_stime {
            time
        } else {
            time - p.old_stime
        };
        p.old_utime = task_time;
        p.old_stime = time;
        p.cpu_usage = if time_delta == 0 {
            0f32
        } else {
            (system_time_delta as f64 * 100f64 / time_delta as f64) as f32
        };
    }
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

unsafe fn get_task_info(pid: Pid) -> libc::proc_taskinfo {
    let mut task_info = mem::zeroed::<libc::proc_taskinfo>();
    // If it doesn't work, we just don't have memory information for this process
    // so it's "fine".
    ffi::proc_pidinfo(
        pid,
        libc::PROC_PIDTASKINFO,
        0,
        &mut task_info as *mut libc::proc_taskinfo as *mut c_void,
        mem::size_of::<libc::proc_taskinfo>() as _,
    );
    task_info
}

pub(crate) fn update_process(
    wrap: &Wrap,
    pid: Pid,
    mut size: size_t,
    time_interval: Option<f64>,
) -> Result<Option<Process>, ()> {
    let mut mib: [c_int; 3] = [libc::CTL_KERN, libc::KERN_ARGMAX, 0];
    let mut proc_args = Vec::with_capacity(size as usize);

    unsafe {
        if let Some(ref mut p) = (*wrap.0.get()).get_mut(&pid) {
            if p.memory == 0 {
                // We don't have access to this process' information.
                force_update(p);
                return Ok(None);
            }
            let task_info = get_task_info(pid);
            let mut thread_info = mem::zeroed::<libc::proc_threadinfo>();
            let (user_time, system_time, thread_status) = if ffi::proc_pidinfo(
                pid,
                libc::PROC_PIDTHREADINFO,
                0,
                &mut thread_info as *mut libc::proc_threadinfo as *mut c_void,
                mem::size_of::<libc::proc_threadinfo>() as _,
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
            p.status = thread_status;
            compute_cpu_usage(p, task_info, system_time, user_time, time_interval);

            p.memory = task_info.pti_resident_size / 1_000;
            p.virtual_memory = task_info.pti_virtual_size / 1_000;
            update_proc_disk_activity(p);
            return Ok(None);
        }

        let mut info = mem::zeroed::<libc::proc_bsdinfo>();
        if ffi::proc_pidinfo(
            pid,
            ffi::PROC_PIDTBSDINFO,
            0,
            &mut info as *mut _ as *mut _,
            mem::size_of::<libc::proc_bsdinfo>() as _,
        ) != mem::size_of::<libc::proc_bsdinfo>() as _
        {
            let mut buffer: Vec<u8> = Vec::with_capacity(ffi::PROC_PIDPATHINFO_MAXSIZE as _);
            match ffi::proc_pidpath(
                pid,
                buffer.as_mut_ptr() as *mut _,
                ffi::PROC_PIDPATHINFO_MAXSIZE,
            ) {
                x if x > 0 => {
                    buffer.set_len(x as _);
                    let tmp = String::from_utf8_unchecked(buffer);
                    let exe = PathBuf::from(tmp);
                    let name = exe
                        .file_name()
                        .and_then(|x| x.to_str())
                        .unwrap_or("")
                        .to_owned();
                    return Ok(Some(Process::new_empty(pid, exe, name)));
                }
                _ => {}
            }
            return Err(());
        }
        let parent = match info.pbi_ppid as i32 {
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
            std::ptr::null_mut(),
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
                .and_then(|x| x.to_str())
                .unwrap_or("")
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

            Process::new_with(
                pid,
                parent,
                info.pbi_start_tvsec,
                exe,
                name,
                parse_command_line(&cmd),
                environ,
                root,
            )
        } else {
            Process::new(pid, parent, info.pbi_start_tvsec)
        };

        let task_info = get_task_info(pid);

        p.memory = task_info.pti_resident_size / 1_000;
        p.virtual_memory = task_info.pti_virtual_size / 1_000;

        p.uid = info.pbi_uid;
        p.gid = info.pbi_gid;
        p.process_status = ProcessStatus::from(info.pbi_status);
        update_proc_disk_activity(&mut p);
        Ok(Some(p))
    }
}

fn update_proc_disk_activity(p: &mut Process) {
    p.old_read_bytes = p.read_bytes;
    p.old_written_bytes = p.written_bytes;

    let mut pidrusage = MaybeUninit::<ffi::RUsageInfoV2>::uninit();
    let retval = unsafe { ffi::proc_pid_rusage(p.pid() as c_int, 2, pidrusage.as_mut_ptr() as _) };

    if retval < 0 {
        sysinfo_debug!("proc_pid_rusage failed: {:?}", retval);
    } else {
        let pidrusage = unsafe { pidrusage.assume_init() };
        p.read_bytes = pidrusage.ri_diskio_bytesread;
        p.written_bytes = pidrusage.ri_diskio_byteswritten;
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
