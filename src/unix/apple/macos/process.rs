// Take a look at the license at the top of the repository in the LICENSE file.

use std::mem::{self, MaybeUninit};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use std::borrow::Borrow;

use libc::{c_int, c_void, kill};

use crate::{DiskUsage, Gid, Pid, Process, ProcessRefreshKind, ProcessStatus, Signal, Uid};

use crate::sys::process::ThreadStatus;
use crate::sys::system::Wrap;
use crate::unix::utils::cstr_to_rust_with_size;

pub(crate) struct ProcessInner {
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

impl ProcessInner {
    pub(crate) fn new_empty(
        pid: Pid,
        exe: PathBuf,
        name: String,
        cwd: PathBuf,
        root: PathBuf,
    ) -> Self {
        Self {
            name,
            pid,
            parent: None,
            cmd: Vec::new(),
            environ: Vec::new(),
            exe,
            cwd,
            root,
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

    pub(crate) fn new(
        pid: Pid,
        parent: Option<Pid>,
        start_time: u64,
        run_time: u64,
        cwd: PathBuf,
        root: PathBuf,
    ) -> Self {
        Self {
            name: String::new(),
            pid,
            parent,
            cmd: Vec::new(),
            environ: Vec::new(),
            exe: PathBuf::new(),
            cwd,
            root,
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

    pub(crate) fn kill_with(&self, signal: Signal) -> Option<bool> {
        let c_signal = crate::sys::convert_signal(signal)?;
        unsafe { Some(kill(self.pid.0, c_signal) == 0) }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn cmd(&self) -> &[String] {
        &self.cmd
    }

    pub(crate) fn exe(&self) -> &Path {
        self.exe.as_path()
    }

    pub(crate) fn pid(&self) -> Pid {
        self.pid
    }

    pub(crate) fn environ(&self) -> &[String] {
        &self.environ
    }

    pub(crate) fn cwd(&self) -> &Path {
        self.cwd.as_path()
    }

    pub(crate) fn root(&self) -> &Path {
        self.root.as_path()
    }

    pub(crate) fn memory(&self) -> u64 {
        self.memory
    }

    pub(crate) fn virtual_memory(&self) -> u64 {
        self.virtual_memory
    }

    pub(crate) fn parent(&self) -> Option<Pid> {
        self.parent
    }

    pub(crate) fn status(&self) -> ProcessStatus {
        // If the status is `Run`, then it's very likely wrong so we instead
        // return a `ProcessStatus` converted from the `ThreadStatus`.
        if self.process_status == ProcessStatus::Run {
            if let Some(thread_status) = self.status {
                return ProcessStatus::from(thread_status);
            }
        }
        self.process_status
    }

    pub(crate) fn start_time(&self) -> u64 {
        self.start_time
    }

    pub(crate) fn run_time(&self) -> u64 {
        self.run_time
    }

    pub(crate) fn cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    pub(crate) fn disk_usage(&self) -> DiskUsage {
        DiskUsage {
            read_bytes: self.read_bytes.saturating_sub(self.old_read_bytes),
            total_read_bytes: self.read_bytes,
            written_bytes: self.written_bytes.saturating_sub(self.old_written_bytes),
            total_written_bytes: self.written_bytes,
        }
    }

    pub(crate) fn user_id(&self) -> Option<&Uid> {
        self.user_id.as_ref()
    }

    pub(crate) fn effective_user_id(&self) -> Option<&Uid> {
        self.effective_user_id.as_ref()
    }

    pub(crate) fn group_id(&self) -> Option<Gid> {
        self.group_id
    }

    pub(crate) fn effective_group_id(&self) -> Option<Gid> {
        self.effective_group_id
    }

    pub(crate) fn wait(&self) {
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

    pub(crate) fn session_id(&self) -> Option<Pid> {
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
    p: &mut ProcessInner,
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
        let errno = crate::unix::libc_errno();
        // If errno is equal to ESCHR, it means the process is dead.
        !errno.is_null() && *errno != libc::ESRCH
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

unsafe fn convert_node_path_info(node: &libc::vnode_info_path) -> PathBuf {
    if node.vip_vi.vi_stat.vst_dev == 0 {
        return PathBuf::new();
    }
    cstr_to_rust_with_size(
        node.vip_path.as_ptr() as _,
        Some(node.vip_path.len() * node.vip_path[0].len()),
    )
    .map(PathBuf::from)
    .unwrap_or_default()
}

unsafe fn create_new_process(
    pid: Pid,
    now: u64,
    refresh_kind: ProcessRefreshKind,
    info: Option<libc::proc_bsdinfo>,
) -> Result<Option<Process>, ()> {
    let (cwd, root) = get_cwd_root(pid, refresh_kind);

    let info = match info {
        Some(info) => info,
        None => {
            if let Some((exe, name)) = get_exe_and_name_backup(pid, refresh_kind) {
                return Ok(Some(Process {
                    inner: ProcessInner::new_empty(pid, exe, name, cwd, root),
                }));
            }
            // If we can't even have the name, no point in keeping it.
            return Err(());
        }
    };
    let parent = match info.pbi_ppid as i32 {
        0 => None,
        p => Some(Pid(p)),
    };

    let start_time = info.pbi_start_tvsec;
    let run_time = now.saturating_sub(start_time);

    let mut p = ProcessInner::new(pid, parent, start_time, run_time, cwd, root);
    match get_process_infos(pid, refresh_kind) {
        Some((exe, name, cmd, environ)) => {
            p.exe = exe;
            p.name = name;
            p.cmd = cmd;
            p.environ = environ;
        }
        None => {
            if let Some((exe, name)) = get_exe_and_name_backup(pid, refresh_kind) {
                p.exe = exe;
                p.name = name;
            } else {
                // If we can't even have the name, no point in keeping it.
                return Err(());
            }
        }
    }

    if refresh_kind.memory() {
        let task_info = get_task_info(pid);
        p.memory = task_info.pti_resident_size;
        p.virtual_memory = task_info.pti_virtual_size;
    }

    p.user_id = Some(Uid(info.pbi_ruid));
    p.effective_user_id = Some(Uid(info.pbi_uid));
    p.group_id = Some(Gid(info.pbi_rgid));
    p.effective_group_id = Some(Gid(info.pbi_gid));
    p.process_status = ProcessStatus::from(info.pbi_status);
    if refresh_kind.disk_usage() {
        update_proc_disk_activity(&mut p);
    }
    Ok(Some(Process { inner: p }))
}

/// Less efficient way to retrieve `exe` and `name`.
fn get_exe_and_name_backup(
    pid: Pid,
    refresh_kind: ProcessRefreshKind,
) -> Option<(PathBuf, String)> {
    let mut buffer: Vec<u8> = Vec::with_capacity(libc::PROC_PIDPATHINFO_MAXSIZE as _);
    match libc::proc_pidpath(
        pid.0,
        buffer.as_mut_ptr() as *mut _,
        libc::PROC_PIDPATHINFO_MAXSIZE as _,
    ) {
        x if x > 0 => {
            buffer.set_len(x as _);
            let tmp = String::from_utf8_unchecked(buffer);
            let mut exe = PathBuf::from(tmp);
            let name = exe
                .file_name()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .to_owned();
            if !refresh_kind.exe() {
                exe = PathBuf::new();
            }
            Some((exe, name))
        }
        _ => Err(()),
    }
}

/// Returns `cwd` and `root`.
fn get_cwd_root(pid: Pid, refresh_kind: ProcessRefreshKind) -> (PathBuf, PathBuf) {
    if !refresh_kind.cwd() && !refresh_kind.root() {
        return (PathBuf::new(), PathBuf::new());
    }
    let mut vnodepathinfo = mem::zeroed::<libc::proc_vnodepathinfo>();
    let result = libc::proc_pidinfo(
        pid.0,
        libc::PROC_PIDVNODEPATHINFO,
        0,
        &mut vnodepathinfo as *mut _ as *mut _,
        mem::size_of::<libc::proc_vnodepathinfo>() as _,
    );
    if result < 1 {
        return (PathBuf::new(), PathBuf::new());
    }
    let cwd = if refresh_kind.cwd() {
        convert_node_path_info(&vnodepathinfo.pvi_cdir)
    } else {
        PathBuf::new()
    };
    let root = if refresh_kind.root() {
        convert_node_path_info(&vnodepathinfo.pvi_rdir)
    } else {
        PathBuf::new()
    };
    (cwd, root)
}

/// Returns (exe, name, cmd, environ)
fn get_process_infos(
    pid: Pid,
    refresh_kind: ProcessRefreshKind,
) -> Option<(PathBuf, String, Vec<String>, Vec<String>)> {
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
    let mut mib: [libc::c_int; 3] = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid.0 as _];
    let mut arg_max = 0;
    // First we retrieve the size we will need for our data (in `arg_max`).
    if libc::sysctl(
        mib.as_mut_ptr(),
        mib.len() as _,
        std::ptr::null_mut(),
        &mut arg_max,
        std::ptr::null_mut(),
        0,
    ) == -1
    {
        sysinfo_debug!(
            "couldn't get arguments and environment size for PID {}",
            pid.0
        );
        return None; // not enough rights I assume?
    }

    let mut proc_args: Vec<u8> = Vec::with_capacity(arg_max as _);
    if libc::sysctl(
        mib.as_mut_ptr(),
        mib.len() as _,
        proc_args.as_mut_slice().as_mut_ptr() as *mut _,
        &mut arg_max,
        std::ptr::null_mut(),
        0,
    ) == -1
    {
        sysinfo_debug!("couldn't get arguments and environment for PID {}", pid.0);
        return None; // What changed since the previous call? Dark magic!
    }

    proc_args.set_len(arg_max);

    if proc_args.is_empty() {
        return None;
    }
    // We copy the number of arguments (`argc`) to `n_args`.
    let mut n_args: c_int = 0;
    libc::memcpy(
        &mut n_args as *mut _ as *mut _,
        proc_args.as_slice().as_ptr() as *const _,
        mem::size_of::<c_int>(),
    );

    // We skip `argc`.
    let proc_args = &proc_args[mem::size_of::<c_int>()..];

    let (mut exe, proc_args) = get_exe(proc_args);
    let name = exe
        .file_name()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_owned();

    if !refresh_kind.exe() {
        exe = PathBuf::new();
    }

    let (cmd, proc_args) = if refresh_kind.environ() || refresh_kind.cmd() {
        get_arguments(proc_args, n_args);
    } else {
        (Vec::new(), &[])
    };
    let environ = if refresh_kind.environ() {
        get_environ(proc_args)
    } else {
        Vec::new()
    };
    Some((exe, name, parse_command_line(&cmd), environ))
}

fn get_exe(data: &[u8]) -> (PathBuf, &[u8]) {
    let pos = data.iter().position(|c| *c == 0).unwrap_or(data.len());
    unsafe {
        (
            Path::new(std::str::from_utf8_unchecked(&data[..pos])).to_path_buf(),
            &data[pos..],
        )
    }
}

fn get_arguments(mut data: &[u8], mut n_args: c_int) -> (Vec<String>, &[u8]) {
    if n_args < 1 {
        return (Vec::new(), data);
    }
    while data.first() == Some(&0) {
        data = &data[1..];
    }
    let mut cmd = Vec::with_capacity(n_args as _);

    unsafe {
        while n_args > 0 && !data.is_empty() {
            let pos = data.iter().position(|c| *c == 0).unwrap_or(data.len());
            let arg = std::str::from_utf8_unchecked(&data[..pos]);
            if !arg.is_empty() {
                cmd.push(arg.to_string());
            }
            data = &data[pos..];
            while data.first() == Some(&0) {
                data = &data[1..];
            }
            n_args -= 1;
        }
        (cmd, data)
    }
}

fn get_environ(mut data: &[u8]) -> Vec<String> {
    while data.first() == Some(&0) {
        data = &data[1..];
    }
    let mut environ = Vec::new();
    unsafe {
        while !data.is_empty() {
            let pos = data.iter().position(|c| *c == 0).unwrap_or(data.len());
            let arg = std::str::from_utf8_unchecked(&data[..pos]);
            if arg.is_empty() {
                return environ;
            }
            environ.push(arg.to_string());
            data = &data[pos..];
            while data.first() == Some(&0) {
                data = &data[1..];
            }
        }
        environ
    }
}

pub(crate) fn update_process(
    wrap: &Wrap,
    pid: Pid,
    time_interval: Option<f64>,
    now: u64,
    refresh_kind: ProcessRefreshKind,
    check_if_alive: bool,
) -> Result<Option<Process>, ()> {
    unsafe {
        if let Some(ref mut p) = (*wrap.0.get()).get_mut(&pid) {
            let p = &mut p.inner;
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
                    return create_new_process(pid, now, refresh_kind, Some(info));
                }
            }
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

            if refresh_kind.cpu() || refresh_kind.memory() {
                let task_info = get_task_info(pid);

                if refresh_kind.cpu() {
                    compute_cpu_usage(p, task_info, system_time, user_time, time_interval);
                }
                if refresh_kind.memory() {
                    p.memory = task_info.pti_resident_size;
                    p.virtual_memory = task_info.pti_virtual_size;
                }
            }

            if refresh_kind.disk_usage() {
                update_proc_disk_activity(p);
            }
            p.updated = true;
            return Ok(None);
        }
        create_new_process(pid, now, refresh_kind, get_bsd_info(pid))
    }
}

fn update_proc_disk_activity(p: &mut ProcessInner) {
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
