// Take a look at the license at the top of the repository in the LICENSE file.

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use libc::{gid_t, kill, uid_t};

use crate::sys::system::SystemInfo;
use crate::sys::utils::{
    get_all_data, get_all_data_from_file, realpath, FileCounter, PathHandler, PathPush,
};
use crate::utils::into_iter;
use crate::{DiskUsage, Gid, Pid, Process, ProcessRefreshKind, ProcessStatus, Signal, Uid};

#[doc(hidden)]
impl From<char> for ProcessStatus {
    fn from(status: char) -> ProcessStatus {
        match status {
            'R' => ProcessStatus::Run,
            'S' => ProcessStatus::Sleep,
            'I' => ProcessStatus::Idle,
            'D' => ProcessStatus::UninterruptibleDiskSleep,
            'Z' => ProcessStatus::Zombie,
            'T' => ProcessStatus::Stop,
            't' => ProcessStatus::Tracing,
            'X' | 'x' => ProcessStatus::Dead,
            'K' => ProcessStatus::Wakekill,
            'W' => ProcessStatus::Waking,
            'P' => ProcessStatus::Parked,
            x => ProcessStatus::Unknown(x as u32),
        }
    }
}

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            ProcessStatus::Idle => "Idle",
            ProcessStatus::Run => "Runnable",
            ProcessStatus::Sleep => "Sleeping",
            ProcessStatus::Stop => "Stopped",
            ProcessStatus::Zombie => "Zombie",
            ProcessStatus::Tracing => "Tracing",
            ProcessStatus::Dead => "Dead",
            ProcessStatus::Wakekill => "Wakekill",
            ProcessStatus::Waking => "Waking",
            ProcessStatus::Parked => "Parked",
            ProcessStatus::UninterruptibleDiskSleep => "UninterruptibleDiskSleep",
            _ => "Unknown",
        })
    }
}

pub(crate) struct ProcessInner {
    pub(crate) name: String,
    pub(crate) cmd: Vec<String>,
    pub(crate) exe: PathBuf,
    pub(crate) pid: Pid,
    parent: Option<Pid>,
    pub(crate) environ: Vec<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) memory: u64,
    pub(crate) virtual_memory: u64,
    utime: u64,
    stime: u64,
    old_utime: u64,
    old_stime: u64,
    start_time_without_boot_time: u64,
    start_time: u64,
    run_time: u64,
    pub(crate) updated: bool,
    cpu_usage: f32,
    user_id: Option<Uid>,
    effective_user_id: Option<Uid>,
    group_id: Option<Gid>,
    effective_group_id: Option<Gid>,
    pub(crate) status: ProcessStatus,
    pub(crate) tasks: HashMap<Pid, Process>,
    pub(crate) stat_file: Option<FileCounter>,
    old_read_bytes: u64,
    old_written_bytes: u64,
    read_bytes: u64,
    written_bytes: u64,
}

impl ProcessInner {
    pub(crate) fn new(pid: Pid) -> Self {
        Self {
            name: String::new(),
            pid,
            parent: None,
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
            start_time_without_boot_time: 0,
            start_time: 0,
            run_time: 0,
            user_id: None,
            effective_user_id: None,
            group_id: None,
            effective_group_id: None,
            status: ProcessStatus::Unknown(0),
            tasks: if pid.0 == 0 {
                HashMap::with_capacity(1000)
            } else {
                HashMap::new()
            },
            stat_file: None,
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
        self.status
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
            written_bytes: self.written_bytes.saturating_sub(self.old_written_bytes),
            total_written_bytes: self.written_bytes,
            read_bytes: self.read_bytes.saturating_sub(self.old_read_bytes),
            total_read_bytes: self.read_bytes,
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

pub(crate) fn compute_cpu_usage(p: &mut ProcessInner, total_time: f32, max_value: f32) {
    // First time updating the values without reference, wait for a second cycle to update cpu_usage
    if p.old_utime == 0 && p.old_stime == 0 {
        return;
    }

    // We use `max_value` to ensure that the process CPU usage will never get bigger than:
    // `"number of CPUs" * 100.`
    p.cpu_usage = (p
        .utime
        .saturating_sub(p.old_utime)
        .saturating_add(p.stime.saturating_sub(p.old_stime)) as f32
        / total_time
        * 100.)
        .min(max_value);

    for task in p.tasks.values_mut() {
        compute_cpu_usage(&mut task.inner, total_time, max_value);
    }
}

pub(crate) fn unset_updated(p: &mut ProcessInner) {
    p.updated = false;
    for task in p.tasks.values_mut() {
        unset_updated(&mut task.inner);
    }
}

pub(crate) fn set_time(p: &mut ProcessInner, utime: u64, stime: u64) {
    p.old_utime = p.utime;
    p.old_stime = p.stime;
    p.utime = utime;
    p.stime = stime;
    p.updated = true;
}

pub(crate) fn update_process_disk_activity(p: &mut ProcessInner, path: &mut PathHandler) {
    let data = match get_all_data(path.join("io"), 16_384) {
        Ok(d) => d,
        Err(_) => return,
    };
    let mut done = 0;
    for line in data.split('\n') {
        let mut parts = line.split(": ");
        match parts.next() {
            Some("read_bytes") => {
                p.old_read_bytes = p.read_bytes;
                p.read_bytes = parts
                    .next()
                    .and_then(|x| x.parse::<u64>().ok())
                    .unwrap_or(p.old_read_bytes);
            }
            Some("write_bytes") => {
                p.old_written_bytes = p.written_bytes;
                p.written_bytes = parts
                    .next()
                    .and_then(|x| x.parse::<u64>().ok())
                    .unwrap_or(p.old_written_bytes);
            }
            _ => continue,
        }
        done += 1;
        if done > 1 {
            // No need to continue the reading.
            break;
        }
    }
}

struct Wrap<'a, T>(UnsafeCell<&'a mut T>);

impl<'a, T> Wrap<'a, T> {
    fn get(&self) -> &'a mut T {
        unsafe { *(self.0.get()) }
    }
}

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl<'a, T> Send for Wrap<'a, T> {}
unsafe impl<'a, T> Sync for Wrap<'a, T> {}

#[inline(always)]
fn compute_start_time_without_boot_time(parts: &[&str], info: &SystemInfo) -> u64 {
    // To be noted that the start time is invalid here, it still needs to be converted into
    // "real" time.
    u64::from_str(parts[21]).unwrap_or(0) / info.clock_cycle
}

fn _get_stat_data(path: &Path, stat_file: &mut Option<FileCounter>) -> Result<String, ()> {
    let mut file = File::open(path.join("stat")).map_err(|_| ())?;
    let data = get_all_data_from_file(&mut file, 1024).map_err(|_| ())?;
    *stat_file = FileCounter::new(file);
    Ok(data)
}

#[inline(always)]
fn get_status(p: &mut ProcessInner, part: &str) {
    p.status = part
        .chars()
        .next()
        .map(ProcessStatus::from)
        .unwrap_or_else(|| ProcessStatus::Unknown(0));
}

fn refresh_user_group_ids(p: &mut ProcessInner, path: &mut PathHandler) {
    if let Some(((user_id, effective_user_id), (group_id, effective_group_id))) =
        get_uid_and_gid(path.join("status"))
    {
        p.user_id = Some(Uid(user_id));
        p.effective_user_id = Some(Uid(effective_user_id));
        p.group_id = Some(Gid(group_id));
        p.effective_group_id = Some(Gid(effective_group_id));
    }
}

#[allow(clippy::too_many_arguments)]
fn update_proc_info(
    p: &mut ProcessInner,
    refresh_kind: ProcessRefreshKind,
    proc_path: &mut PathHandler,
    parts: &[&str],
    memory: u64,
    virtual_memory: u64,
    uptime: u64,
    info: &SystemInfo,
) {
    get_status(p, parts[2]);

    if refresh_kind.user() && p.user_id.is_none() {
        refresh_user_group_ids(p, proc_path);
    }

    if refresh_kind.exe() && p.exe.as_os_str().is_empty() {
        match proc_path.join("exe").read_link() {
            Ok(exe_path) => p.exe = exe_path,
            Err(_error) => {
                sysinfo_debug!("Failed to retrieve exe for {}: {_error:?}", p.pid().0);
                // Do not use cmd[0] because it is not the same thing.
                // See https://github.com/GuillaumeGomez/sysinfo/issues/697.
                p.exe = PathBuf::new();
            }
        }
    }

    if refresh_kind.cmd() && p.cmd.is_empty() {
        p.cmd = copy_from_file(proc_path.join("cmdline"));
    }
    if refresh_kind.environ() {
        p.environ = copy_from_file(proc_path.join("environ"));
    }
    if refresh_kind.cwd() {
        p.cwd = realpath(proc_path.join("cwd"));
    }
    if refresh_kind.root() {
        p.root = realpath(proc_path.join("root"));
    }

    update_time_and_memory(
        proc_path,
        p,
        parts,
        memory,
        virtual_memory,
        uptime,
        info,
        refresh_kind,
    );
    if refresh_kind.disk_usage() {
        update_process_disk_activity(p, proc_path);
    }
}

fn retrieve_all_new_process_info(
    pid: Pid,
    proc_list: &ProcessInner,
    parts: &[&str],
    path: &Path,
    info: &SystemInfo,
    refresh_kind: ProcessRefreshKind,
    uptime: u64,
) -> Process {
    let mut p = ProcessInner::new(pid);
    let mut proc_path = PathHandler::new(path);
    let name = parts[1];

    p.parent = if proc_list.pid.0 != 0 {
        Some(proc_list.pid)
    } else {
        match Pid::from_str(parts[3]) {
            Ok(p) if p.0 != 0 => Some(p),
            _ => None,
        }
    };

    p.start_time_without_boot_time = compute_start_time_without_boot_time(parts, info);
    p.start_time = p
        .start_time_without_boot_time
        .saturating_add(info.boot_time);

    p.name = name.into();

    update_proc_info(
        &mut p,
        refresh_kind,
        &mut proc_path,
        parts,
        proc_list.memory,
        proc_list.virtual_memory,
        uptime,
        info,
    );

    Process { inner: p }
}

pub(crate) fn _get_process_data(
    path: &Path,
    proc_list: &mut ProcessInner,
    pid: Pid,
    uptime: u64,
    info: &SystemInfo,
    refresh_kind: ProcessRefreshKind,
) -> Result<(Option<Process>, Pid), ()> {
    let pid = match path.file_name().and_then(|x| x.to_str()).map(Pid::from_str) {
        // If `pid` and `nb` are the same, it means the file is linking to itself so we skip it.
        //
        // It's because when reading `/proc/[PID]` folder, we then go through the folders inside it.
        // Then, if we encounter a sub-folder with the same PID as the parent, then it's a link to
        // the current folder we already did read so no need to do anything.
        Some(Ok(nb)) if nb != pid => nb,
        _ => return Err(()),
    };

    let data;
    let parts = if let Some(ref mut entry) = proc_list.tasks.get_mut(&pid) {
        let entry = &mut entry.inner;
        data = if let Some(mut f) = entry.stat_file.take() {
            match get_all_data_from_file(&mut f, 1024) {
                Ok(data) => {
                    // Everything went fine, we put back the file descriptor.
                    entry.stat_file = Some(f);
                    data
                }
                Err(_) => {
                    // It's possible that the file descriptor is no longer valid in case the
                    // original process was terminated and another one took its place.
                    _get_stat_data(path, &mut entry.stat_file)?
                }
            }
        } else {
            _get_stat_data(path, &mut entry.stat_file)?
        };
        let parts = parse_stat_file(&data).ok_or(())?;
        let start_time_without_boot_time = compute_start_time_without_boot_time(&parts, info);

        // It's possible that a new process took this same PID when the "original one" terminated.
        // If the start time differs, then it means it's not the same process anymore and that we
        // need to get all its information, hence why we check it here.
        if start_time_without_boot_time == entry.start_time_without_boot_time {
            let mut proc_path = PathHandler::new(path);

            update_proc_info(
                entry,
                refresh_kind,
                &mut proc_path,
                &parts,
                proc_list.memory,
                proc_list.virtual_memory,
                uptime,
                info,
            );

            if refresh_kind.user() && entry.user_id.is_none() {
                refresh_user_group_ids(entry, &mut proc_path);
            }
            return Ok((None, pid));
        }
        parts
    } else {
        let mut stat_file = None;
        let data = _get_stat_data(path, &mut stat_file)?;
        let parts = parse_stat_file(&data).ok_or(())?;

        let mut p =
            retrieve_all_new_process_info(pid, proc_list, &parts, path, info, refresh_kind, uptime);
        p.inner.stat_file = stat_file;
        return Ok((Some(p), pid));
    };

    // If we're here, it means that the PID still exists but it's a different process.
    let p = retrieve_all_new_process_info(pid, proc_list, &parts, path, info, refresh_kind, uptime);
    match proc_list.tasks.get_mut(&pid) {
        Some(ref mut entry) => **entry = p,
        // If it ever enters this case, it means that the process was removed from the HashMap
        // in-between with the usage of dark magic.
        None => unreachable!(),
    }
    // Since this PID is already in the HashMap, no need to add it again.
    Ok((None, pid))
}

#[allow(clippy::too_many_arguments)]
fn update_time_and_memory(
    path: &mut PathHandler,
    entry: &mut ProcessInner,
    parts: &[&str],
    parent_memory: u64,
    parent_virtual_memory: u64,
    uptime: u64,
    info: &SystemInfo,
    refresh_kind: ProcessRefreshKind,
) {
    {
        if refresh_kind.memory() {
            // rss
            entry.memory = u64::from_str(parts[23])
                .unwrap_or(0)
                .saturating_mul(info.page_size_b);
            if entry.memory >= parent_memory {
                entry.memory -= parent_memory;
            }
            // vsz correspond to the Virtual memory size in bytes.
            // see: https://man7.org/linux/man-pages/man5/proc.5.html
            entry.virtual_memory = u64::from_str(parts[22]).unwrap_or(0);
            if entry.virtual_memory >= parent_virtual_memory {
                entry.virtual_memory -= parent_virtual_memory;
            }
        }
        set_time(
            entry,
            u64::from_str(parts[13]).unwrap_or(0),
            u64::from_str(parts[14]).unwrap_or(0),
        );
        entry.run_time = uptime.saturating_sub(entry.start_time_without_boot_time);
    }
    refresh_procs(
        entry,
        path.join("task"),
        entry.pid,
        uptime,
        info,
        refresh_kind,
    );
}

pub(crate) fn refresh_procs(
    proc_list: &mut ProcessInner,
    path: &Path,
    pid: Pid,
    uptime: u64,
    info: &SystemInfo,
    refresh_kind: ProcessRefreshKind,
) -> bool {
    let d = match fs::read_dir(path) {
        Ok(d) => d,
        Err(_) => return false,
    };
    let folders = d
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let entry = entry.path();

            if entry.is_dir() {
                Some(entry)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if pid.0 == 0 {
        let proc_list = Wrap(UnsafeCell::new(proc_list));

        #[cfg(feature = "multithread")]
        use rayon::iter::ParallelIterator;

        into_iter(folders)
            .filter_map(|e| {
                let (p, _) = _get_process_data(
                    e.as_path(),
                    proc_list.get(),
                    pid,
                    uptime,
                    info,
                    refresh_kind,
                )
                .ok()?;
                p
            })
            .collect::<Vec<_>>()
    } else {
        let mut updated_pids = Vec::with_capacity(folders.len());
        let new_tasks = folders
            .iter()
            .filter_map(|e| {
                let (p, pid) =
                    _get_process_data(e.as_path(), proc_list, pid, uptime, info, refresh_kind)
                        .ok()?;
                updated_pids.push(pid);
                p
            })
            .collect::<Vec<_>>();
        // Sub-tasks are not cleaned up outside so we do it here directly.
        proc_list
            .tasks
            .retain(|&pid, _| updated_pids.iter().any(|&x| x == pid));
        new_tasks
    }
    .into_iter()
    .for_each(|e| {
        proc_list.tasks.insert(e.pid(), e);
    });
    true
}

fn copy_from_file(entry: &Path) -> Vec<String> {
    match File::open(entry) {
        Ok(mut f) => {
            let mut data = Vec::with_capacity(16_384);

            if let Err(_e) = f.read_to_end(&mut data) {
                sysinfo_debug!("Failed to read file in `copy_from_file`: {:?}", _e);
                Vec::new()
            } else {
                let mut out = Vec::with_capacity(10);
                let mut data = data.as_slice();
                while let Some(pos) = data.iter().position(|c| *c == 0) {
                    match std::str::from_utf8(&data[..pos]).map(|s| s.trim()) {
                        Ok(s) if !s.is_empty() => out.push(s.to_string()),
                        _ => {}
                    }
                    data = &data[pos + 1..];
                }
                out
            }
        }
        Err(_e) => {
            sysinfo_debug!("Failed to open file in `copy_from_file`: {:?}", _e);
            Vec::new()
        }
    }
}

// Fetch tuples of real and effective UID and GID.
fn get_uid_and_gid(file_path: &Path) -> Option<((uid_t, uid_t), (gid_t, gid_t))> {
    let status_data = get_all_data(file_path, 16_385).ok()?;

    // We're only interested in the lines starting with Uid: and Gid:
    // here. From these lines, we're looking at the first and second entries to get
    // the real u/gid.

    let f = |h: &str, n: &str| -> (Option<uid_t>, Option<uid_t>) {
        if h.starts_with(n) {
            let mut ids = h.split_whitespace();
            let real = ids.nth(1).unwrap_or("0").parse().ok();
            let effective = ids.next().unwrap_or("0").parse().ok();

            (real, effective)
        } else {
            (None, None)
        }
    };
    let mut uid = None;
    let mut effective_uid = None;
    let mut gid = None;
    let mut effective_gid = None;
    for line in status_data.lines() {
        if let (Some(real), Some(effective)) = f(line, "Uid:") {
            debug_assert!(uid.is_none() && effective_uid.is_none());
            uid = Some(real);
            effective_uid = Some(effective);
        } else if let (Some(real), Some(effective)) = f(line, "Gid:") {
            debug_assert!(gid.is_none() && effective_gid.is_none());
            gid = Some(real);
            effective_gid = Some(effective);
        } else {
            continue;
        }
        if uid.is_some() && gid.is_some() {
            break;
        }
    }
    match (uid, effective_uid, gid, effective_gid) {
        (Some(uid), Some(effective_uid), Some(gid), Some(effective_gid)) => {
            Some(((uid, effective_uid), (gid, effective_gid)))
        }
        _ => None,
    }
}

fn parse_stat_file(data: &str) -> Option<Vec<&str>> {
    // The stat file is "interesting" to parse, because spaces cannot
    // be used as delimiters. The second field stores the command name
    // surrounded by parentheses. Unfortunately, whitespace and
    // parentheses are legal parts of the command, so parsing has to
    // proceed like this: The first field is delimited by the first
    // whitespace, the second field is everything until the last ')'
    // in the entire string. All other fields are delimited by
    // whitespace.

    let mut parts = Vec::with_capacity(52);
    let mut data_it = data.splitn(2, ' ');
    parts.push(data_it.next()?);
    let mut data_it = data_it.next()?.rsplitn(2, ')');
    let data = data_it.next()?;
    parts.push(data_it.next()?);
    parts.extend(data.split_whitespace());
    // Remove command name '('
    if let Some(name) = parts[1].strip_prefix('(') {
        parts[1] = name;
    }
    Some(parts)
}
