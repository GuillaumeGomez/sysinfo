// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::CStr;
use std::mem::{self, MaybeUninit};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use std::borrow::Borrow;

use libc::{c_int, c_void, kill, size_t};

use crate::{DiskUsage, Gid, Pid, ProcessExt, ProcessRefreshKind, ProcessStatus, Signal, Uid};

use crate::sys::process::ThreadStatus;
use crate::sys::system::Wrap;

#[doc = include_str!("../../../md_doc/process.md")]
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
    old_utime: u64,
    old_stime: u64,
    start_time: u64,
    run_time: u64,
    pub(crate) updated: bool,
    cpu_usage: f32,
    user_id: Option<Uid>,
    effective_user_id: Option<Uid>,
    group_id: Option<Gid>,
    effective_group_id: Option<Gid>,
    pub(crate) process_status: ProcessStatus,
    /// Status of process (running, stopped, waiting, etc). `None` means `sysinfo` doesn't have
    /// enough rights to get this information.
    ///
    /// This is very likely this one that you want instead of `process_status`.
    pub(crate) status: Option<ThreadStatus>,
    pub(crate) old_read_bytes: u64,
    pub(crate) old_written_bytes: u64,
    pub(crate) read_bytes: u64,
    pub(crate) written_bytes: u64,
}

impl Process {
    pub(crate) fn new_empty(pid: Pid, exe: PathBuf, name: String, cwd: PathBuf) -> Process {
        Process {
            name,
            pid,
            parent: None,
            cmd: Vec::new(),
            environ: Vec::new(),
            exe,
            cwd,
            root: PathBuf::new(),
            memory: 0,
            virtual_memory: 0,
            cpu_usage: 0.,
            old_utime: 0,
            old_stime: 0,
            updated: true,
            start_time: 0,
            run_time: 0,
            user_id: None,
            effective_user_id: None,
            group_id: None,
            effective_group_id: None,
            process_status: ProcessStatus::Unknown(0),
            status: None,
            old_read_bytes: 0,
            old_written_bytes: 0,
            read_bytes: 0,
            written_bytes: 0,
        }
    }

    pub(crate) fn new(pid: Pid, parent: Option<Pid>, start_time: u64, run_time: u64) -> Process {
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
            old_utime: 0,
            old_stime: 0,
            updated: true,
            start_time,
            run_time,
            user_id: None,
            effective_user_id: None,
            group_id: None,
            effective_group_id: None,
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
    fn kill_with(&self, signal: Signal) -> Option<bool> {
        let c_signal = crate::sys::system::convert_signal(signal)?;
        unsafe { Some(kill(self.pid.0, c_signal) == 0) }
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
        // If the status is `Run`, then it's very likely wrong so we instead
        // return a `ProcessStatus` converted from the `ThreadStatus`.
        if self.process_status == ProcessStatus::Run {
            if let Some(thread_status) = self.status {
                return ProcessStatus::from(thread_status);
            }
        }
        self.process_status
    }

    fn start_time(&self) -> u64 {
        self.start_time
    }

    fn run_time(&self) -> u64 {
        self.run_time
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

    fn user_id(&self) -> Option<&Uid> {
        self.user_id.as_ref()
    }

    fn effective_user_id(&self) -> Option<&Uid> {
        self.effective_user_id.as_ref()
    }

    fn group_id(&self) -> Option<Gid> {
        self.group_id
    }

    fn effective_group_id(&self) -> Option<Gid> {
        self.effective_group_id
    }

    fn wait(&self) {
        let mut status = 0;
        // attempt waiting
        unsafe {
            if retry_eintr!(libc::waitpid(self.pid.0, &mut status, 0)) < 0 {
                // attempt failed (non-child process) so loop until process ends
                let duration = std::time::Duration::from_millis(10);
                while kill(self.pid.0, 0) == 0 {
                    std::thread::sleep(duration);
                }
            }
        }
    }

    fn session_id(&self) -> Option<Pid> {
        unsafe {
            let session_id = libc::getsid(self.pid.0);
            if session_id < 0 {
                None
            } else {
                Some(Pid(session_id))
            }
        }
    }
}

#[allow(deprecated)] // Because of libc::mach_absolute_time.
pub(crate) fn compute_cpu_usage(
    p: &mut Process,
    task_info: libc::proc_taskinfo,
    system_time: u64,
    user_time: u64,
    time_interval: Option<f64>,
) {
    if let Some(time_interval) = time_interval {
        let total_existing_time = p.old_stime.saturating_add(p.old_utime);
        let mut updated_cpu_usage = false;
        if time_interval > 0.000001 && total_existing_time > 0 {
            let total_current_time = task_info
                .pti_total_system
                .saturating_add(task_info.pti_total_user);

            let total_time_diff = total_current_time.saturating_sub(total_existing_time);
            if total_time_diff > 0 {
                p.cpu_usage = (total_time_diff as f64 / time_interval * 100.) as f32;
                updated_cpu_usage = true;
            }
        }
        if !updated_cpu_usage {
            p.cpu_usage = 0.;
        }
        p.old_stime = task_info.pti_total_system;
        p.old_utime = task_info.pti_total_user;
    } else {
        unsafe {
            // This is the "backup way" of CPU computation.
            let time = libc::mach_absolute_time();
            let task_time = user_time
                .saturating_add(system_time)
                .saturating_add(task_info.pti_total_user)
                .saturating_add(task_info.pti_total_system);

            let system_time_delta = if task_time < p.old_utime {
                task_time
            } else {
                task_time.saturating_sub(p.old_utime)
            };
            let time_delta = if time < p.old_stime {
                time
            } else {
                time.saturating_sub(p.old_stime)
            };
            p.old_utime = task_time;
            p.old_stime = time;
            p.cpu_usage = if time_delta == 0 {
                0f32
            } else {
                (system_time_delta as f64 * 100f64 / time_delta as f64) as f32
            };
        }
    }
}

unsafe fn get_task_info(pid: Pid) -> libc::proc_taskinfo {
    let mut task_info = mem::zeroed::<libc::proc_taskinfo>();
    // If it doesn't work, we just don't have memory information for this process
    // so it's "fine".
    libc::proc_pidinfo(
        pid.0,
        libc::PROC_PIDTASKINFO,
        0,
        &mut task_info as *mut libc::proc_taskinfo as *mut c_void,
        mem::size_of::<libc::proc_taskinfo>() as _,
    );
    task_info
}

#[inline]
fn check_if_pid_is_alive(pid: Pid, check_if_alive: bool) -> bool {
    // In case we are iterating all pids we got from `proc_listallpids`, then
    // there is no point checking if the process is alive since it was returned
    // from this function.
    if !check_if_alive {
        return true;
    }
    unsafe {
        if kill(pid.0, 0) == 0 {
            return true;
        }
        // `kill` failed but it might not be because the process is dead.
        let errno = crate::libc_errno();
        // If errno is equal to ESCHR, it means the process is dead.
        !errno.is_null() && *errno != libc::ESRCH
    }
}

#[inline]
fn do_not_get_env_path(_: &str, _: &mut PathBuf, _: &mut bool) {}

#[inline]
fn do_get_env_path(env: &str, root: &mut PathBuf, check: &mut bool) {
    if *check && env.starts_with("PATH=") {
        *check = false;
        *root = Path::new(&env[5..]).to_path_buf();
    }
}

unsafe fn get_bsd_info(pid: Pid) -> Option<libc::proc_bsdinfo> {
    let mut info = mem::zeroed::<libc::proc_bsdinfo>();

    if libc::proc_pidinfo(
        pid.0,
        libc::PROC_PIDTBSDINFO,
        0,
        &mut info as *mut _ as *mut _,
        mem::size_of::<libc::proc_bsdinfo>() as _,
    ) != mem::size_of::<libc::proc_bsdinfo>() as _
    {
        None
    } else {
        Some(info)
    }
}

unsafe fn create_new_process(
    pid: Pid,
    mut size: size_t,
    now: u64,
    refresh_kind: ProcessRefreshKind,
    info: Option<libc::proc_bsdinfo>,
) -> Result<Option<Process>, ()> {
    let mut vnodepathinfo = mem::zeroed::<libc::proc_vnodepathinfo>();
    let result = libc::proc_pidinfo(
        pid.0,
        libc::PROC_PIDVNODEPATHINFO,
        0,
        &mut vnodepathinfo as *mut _ as *mut _,
        mem::size_of::<libc::proc_vnodepathinfo>() as _,
    );
    let cwd = if result > 0 {
        let buffer = vnodepathinfo.pvi_cdir.vip_path;
        let buffer = CStr::from_ptr(buffer.as_ptr() as _);
        buffer
            .to_str()
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::new())
    } else {
        PathBuf::new()
    };

    let info = match info {
        Some(info) => info,
        None => {
            let mut buffer: Vec<u8> = Vec::with_capacity(libc::PROC_PIDPATHINFO_MAXSIZE as _);
            match libc::proc_pidpath(
                pid.0,
                buffer.as_mut_ptr() as *mut _,
                libc::PROC_PIDPATHINFO_MAXSIZE as _,
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
                    return Ok(Some(Process::new_empty(pid, exe, name, cwd)));
                }
                _ => {}
            }
            return Err(());
        }
    };
    let parent = match info.pbi_ppid as i32 {
        0 => None,
        p => Some(Pid(p)),
    };

    let mut proc_args = Vec::with_capacity(size as _);
    let ptr: *mut u8 = proc_args.as_mut_slice().as_mut_ptr();
    let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid.0 as _];
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
        mib.len() as _,
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

    let start_time = info.pbi_start_tvsec;
    let run_time = now.saturating_sub(start_time);

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
            if let Some(parent_path) = exe.parent() {
                get_environ(
                    ptr,
                    cp,
                    size,
                    parent_path.to_path_buf(),
                    do_not_get_env_path,
                )
            } else {
                get_environ(ptr, cp, size, PathBuf::new(), do_get_env_path)
            }
        } else {
            get_environ(ptr, cp, size, PathBuf::new(), do_get_env_path)
        };
        let mut p = Process::new(pid, parent, start_time, run_time);

        p.exe = exe;
        p.name = name;
        p.cwd = cwd;
        p.cmd = parse_command_line(&cmd);
        p.environ = environ;
        p.root = root;
        p
    } else {
        Process::new(pid, parent, start_time, run_time)
    };

    let task_info = get_task_info(pid);

    p.memory = task_info.pti_resident_size;
    p.virtual_memory = task_info.pti_virtual_size;

    p.user_id = Some(Uid(info.pbi_ruid));
    p.effective_user_id = Some(Uid(info.pbi_uid));
    p.group_id = Some(Gid(info.pbi_rgid));
    p.effective_group_id = Some(Gid(info.pbi_gid));
    p.process_status = ProcessStatus::from(info.pbi_status);
    if refresh_kind.disk_usage() {
        update_proc_disk_activity(&mut p);
    }
    Ok(Some(p))
}

pub(crate) fn update_process(
    wrap: &Wrap,
    pid: Pid,
    size: size_t,
    time_interval: Option<f64>,
    now: u64,
    refresh_kind: ProcessRefreshKind,
    check_if_alive: bool,
) -> Result<Option<Process>, ()> {
    unsafe {
        if let Some(ref mut p) = (*wrap.0.get()).get_mut(&pid) {
            if p.memory == 0 {
                // We don't have access to this process' information.
                return if check_if_pid_is_alive(pid, check_if_alive) {
                    p.updated = true;
                    Ok(None)
                } else {
                    Err(())
                };
            }
            if let Some(info) = get_bsd_info(pid) {
                if info.pbi_start_tvsec != p.start_time {
                    // We don't it to be removed, just replaced.
                    p.updated = true;
                    // The owner of this PID changed.
                    return create_new_process(pid, size, now, refresh_kind, Some(info));
                }
            }
            let task_info = get_task_info(pid);
            let mut thread_info = mem::zeroed::<libc::proc_threadinfo>();
            let (user_time, system_time, thread_status) = if libc::proc_pidinfo(
                pid.0,
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
                // It very likely means that the process is dead...
                if check_if_pid_is_alive(pid, check_if_alive) {
                    (0, 0, Some(ThreadStatus::Running))
                } else {
                    return Err(());
                }
            };
            p.status = thread_status;

            if refresh_kind.cpu() {
                compute_cpu_usage(p, task_info, system_time, user_time, time_interval);
            }

            p.memory = task_info.pti_resident_size;
            p.virtual_memory = task_info.pti_virtual_size;
            if refresh_kind.disk_usage() {
                update_proc_disk_activity(p);
            }
            p.updated = true;
            return Ok(None);
        }
        create_new_process(pid, size, now, refresh_kind, get_bsd_info(pid))
    }
}

fn update_proc_disk_activity(p: &mut Process) {
    p.old_read_bytes = p.read_bytes;
    p.old_written_bytes = p.written_bytes;

    let mut pidrusage = MaybeUninit::<libc::rusage_info_v2>::uninit();

    unsafe {
        let retval = libc::proc_pid_rusage(
            p.pid().0 as _,
            libc::RUSAGE_INFO_V2,
            pidrusage.as_mut_ptr() as _,
        );

        if retval < 0 {
            sysinfo_debug!("proc_pid_rusage failed: {:?}", retval);
        } else {
            let pidrusage = pidrusage.assume_init();
            p.read_bytes = pidrusage.ri_diskio_bytesread;
            p.written_bytes = pidrusage.ri_diskio_byteswritten;
        }
    }
}

#[allow(unknown_lints)]
#[allow(clippy::uninit_vec)]
pub(crate) fn get_proc_list() -> Option<Vec<Pid>> {
    unsafe {
        let count = libc::proc_listallpids(::std::ptr::null_mut(), 0);
        if count < 1 {
            return None;
        }
        let mut pids: Vec<Pid> = Vec::with_capacity(count as usize);
        pids.set_len(count as usize);
        let count = count * mem::size_of::<Pid>() as i32;
        let x = libc::proc_listallpids(pids.as_mut_ptr() as *mut c_void, count);

        if x < 1 || x as usize >= pids.len() {
            None
        } else {
            pids.set_len(x as usize);
            Some(pids)
        }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_path() {
        let mut path = PathBuf::new();
        let mut check = true;

        do_get_env_path("PATH=tadam", &mut path, &mut check);

        assert!(!check);
        assert_eq!(path, PathBuf::from("tadam"));
    }
}
