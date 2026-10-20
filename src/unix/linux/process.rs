// Take a look at the license at the top of the repository in the LICENSE file.

use std::cell::UnsafeCell;
use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, CString, OsStr, OsString};
use std::fmt;
use std::fs::{self, DirEntry, File, read_dir};
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(feature = "gpu")]
use std::time::Instant;

use libc::{c_ulong, gid_t, pid_t, uid_t};

use crate::sys::system::SystemInfo;
use crate::sys::utils::{get_all_data_from_file, read_file_into, read_path_into};
use crate::{
    DiskUsage, Gid, Pid, Process, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, Signal,
    ThreadKind, Uid, UpdateKind,
};

#[doc(hidden)]
impl From<u8> for ProcessStatus {
    fn from(status: u8) -> ProcessStatus {
        match status {
            b'R' => ProcessStatus::Run,
            b'S' => ProcessStatus::Sleep,
            b'I' => ProcessStatus::Idle,
            b'D' => ProcessStatus::UninterruptibleDiskSleep,
            b'Z' => ProcessStatus::Zombie,
            b'T' => ProcessStatus::Stop,
            b't' => ProcessStatus::Tracing,
            b'X' | b'x' => ProcessStatus::Dead,
            b'K' => ProcessStatus::Wakekill,
            b'W' => ProcessStatus::Waking,
            b'P' => ProcessStatus::Parked,
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

#[allow(dead_code)]
#[repr(usize)]
enum ProcIndex {
    Pid = 0,
    State,
    ParentPid,
    GroupId,
    SessionId,
    Tty,
    ForegroundProcessGroupId,
    Flags,
    MinorFaults,
    ChildrenMinorFaults,
    MajorFaults,
    ChildrenMajorFaults,
    UserTime,
    SystemTime,
    ChildrenUserTime,
    ChildrenKernelTime,
    Priority,
    Nice,
    NumberOfThreads,
    IntervalTimerSigalarm,
    StartTime,
    VirtualSize,
    ResidentSetSize,
    // More exist but we only use the listed ones. For more, take a look at `man proc`.
}

#[cfg(feature = "gpu")]
#[derive(Default)]
struct GpuInfo {
    last_update: Option<Instant>,
    gpu_time: u64,
    gpu_usage: Option<f32>,
    memory: Option<u64>,
    /// `true` if this is a thread sharing the fd table of its parent process. In such cases, to
    /// prevent reading the `fd`/`fdinfo` folders to retrieve the same information, we instead skip
    /// threads with this field to `true` and copy the information from the parent process after
    /// we're done updating all processes.
    share_parent_fd_table: bool,
}

pub(crate) struct ProcessInner {
    pub(crate) name: OsString,
    pub(crate) cmd: Vec<OsString>,
    pub(crate) exe: Option<PathBuf>,
    pub(crate) pid: Pid,
    parent: Option<Pid>,
    pub(crate) environ: Vec<OsString>,
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) root: Option<PathBuf>,
    pub(crate) memory: u64,
    pub(crate) virtual_memory: u64,
    utime: u64,
    stime: u64,
    old_utime: u64,
    old_stime: u64,
    start_time_without_boot_time: u64,
    start_time: u64,
    start_time_raw: u64,
    run_time: u64,
    pub(crate) updated: bool,
    cpu_usage: f32,
    user_id: Option<Uid>,
    effective_user_id: Option<Uid>,
    group_id: Option<Gid>,
    effective_group_id: Option<Gid>,
    pub(crate) status: ProcessStatus,
    pub(crate) tasks: Option<HashSet<Pid>>,
    old_read_bytes: u64,
    old_written_bytes: u64,
    read_bytes: u64,
    written_bytes: u64,
    thread_kind: Option<ThreadKind>,
    proc_path: PathBuf,
    accumulated_cpu_time: u64,
    exists: bool,
    #[cfg(feature = "gpu")]
    gpu_info: GpuInfo,
}

impl ProcessInner {
    pub(crate) fn new(pid: Pid, proc_path: PathBuf) -> Self {
        Self {
            name: OsString::new(),
            pid,
            parent: None,
            cmd: Vec::new(),
            environ: Vec::new(),
            exe: None,
            cwd: None,
            root: None,
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
            start_time_raw: 0,
            run_time: 0,
            user_id: None,
            effective_user_id: None,
            group_id: None,
            effective_group_id: None,
            status: ProcessStatus::Unknown(0),
            tasks: None,
            old_read_bytes: 0,
            old_written_bytes: 0,
            read_bytes: 0,
            written_bytes: 0,
            thread_kind: None,
            proc_path,
            accumulated_cpu_time: 0,
            exists: true,
            #[cfg(feature = "gpu")]
            gpu_info: GpuInfo::default(),
        }
    }

    pub(crate) fn kill_with(&self, signal: Signal) -> Option<bool> {
        let c_signal = crate::sys::system::convert_signal(signal)?;
        unsafe { Some(libc::kill(self.pid.0, c_signal) == 0) }
    }

    pub(crate) fn name(&self) -> &OsStr {
        &self.name
    }

    pub(crate) fn cmd(&self) -> &[OsString] {
        &self.cmd
    }

    pub(crate) fn exe(&self) -> Option<&Path> {
        self.exe.as_deref()
    }

    pub(crate) fn pid(&self) -> Pid {
        self.pid
    }

    pub(crate) fn cgroup_limits(&self) -> Option<crate::CGroupLimits> {
        crate::sys::cgroup::limits_for_process(&self.proc_path)
    }

    pub(crate) fn environ(&self) -> &[OsString] {
        &self.environ
    }

    pub(crate) fn cwd(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }

    pub(crate) fn root(&self) -> Option<&Path> {
        self.root.as_deref()
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

    pub(crate) fn accumulated_cpu_time(&self) -> u64 {
        self.accumulated_cpu_time
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

    pub(crate) fn wait(&self) -> Option<ExitStatus> {
        // If anything fails when trying to retrieve the start time, better to return `None`.
        let mut file = File::open(self.proc_path.join("stat")).ok()?;
        let data = get_all_data_from_file(&mut file, 1024).ok()?;

        let parts = parse_stat_file(&data, Some(self.start_time_raw))?;

        if parts.start_time != self.start_time_raw {
            sysinfo_debug!("Seems to not be the same process anymore");
            return None;
        }

        crate::unix::utils::wait_process(self.pid)
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

    pub(crate) fn thread_kind(&self) -> Option<ThreadKind> {
        self.thread_kind
    }

    pub(crate) fn switch_updated(&mut self) -> bool {
        std::mem::replace(&mut self.updated, false)
    }

    pub(crate) fn set_nonexistent(&mut self) {
        self.exists = false;
    }

    pub(crate) fn exists(&self) -> bool {
        self.exists
    }

    pub(crate) fn open_files(&self) -> Option<usize> {
        let open_files_dir = self.proc_path.as_path().join("fd");
        match fs::read_dir(&open_files_dir) {
            Ok(entries) => Some(entries.count() as _),
            Err(_error) => {
                sysinfo_debug!(
                    "Failed to get open files in `{}`: {_error:?}",
                    open_files_dir.display(),
                );
                None
            }
        }
    }

    pub(crate) fn open_files_limit(&self) -> Option<usize> {
        let limits_files = self.proc_path.as_path().join("limits");
        match fs::read(&limits_files) {
            Ok(content) => {
                for line in content.split(|c| *c == b'\n') {
                    if let Some(line) = line.strip_prefix(b"Max open files ")
                        && let Some(nb) = line.split(|c| *c == b' ').find(|p| !p.is_empty())
                    {
                        return parse_ascii_checked_usize(nb);
                    }
                }
                None
            }
            Err(_error) => {
                sysinfo_debug!(
                    "Failed to get limits in `{}`: {_error:?}",
                    limits_files.display()
                );
                None
            }
        }
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn gpu_usage(&self) -> Option<f32> {
        self.gpu_info.gpu_usage
    }

    #[cfg(feature = "gpu")]
    pub fn gpu_memory(&self) -> Option<u64> {
        self.gpu_info.memory
    }
}

macro_rules! parse_integers {
    ($($ty:ident: $name:ident),+) => {
        $(
            fn $name(bytes: &[u8]) -> Option<$ty> {
                let mut num: $ty = 0;
                for &b in bytes {
                    if !b.is_ascii_digit() {
                        return None;
                    }
                    num = num.checked_mul(10)?.checked_add((b - b'0') as $ty)?;
                }
                Some(num)
            }
        )+
    }
}

parse_integers!(
    u64: parse_ascii_checked_u64,
    c_ulong: parse_ascii_checked_culong,
    usize: parse_ascii_checked_usize,
    pid_t: parse_ascii_checked_pid_t,
    uid_t: parse_ascii_checked_uid_t
);

#[cfg(feature = "gpu")]
mod gpu {
    use super::*;
    use std::mem::MaybeUninit;

    const DRM_MAJOR: u32 = 226; // `/dev/dri/*`
    const ACCEL_MAJOR: u32 = 261; // `/dev/accel/*`

    // It's possible, using the `unshare` function, to make a thread not use the same `fd` table
    // as its parent process. So returns `true` if both `pid` share the same fd table.
    pub(super) fn same_fd_table(pid1: Pid, pid2: Pid) -> bool {
        // FIXME: Remove it once `libc` defines this constant.
        const KCMP_FILES: libc::c_int = 2;

        unsafe { libc::syscall(libc::SYS_kcmp, pid1.0, pid2.0, KCMP_FILES, 0, 0) == 0 }
    }

    // Threads which share the fd table of their parent process aren't refreshed as it would be a
    // duplicate. Instead, we go through all updated processes and for thread ones, we copy the
    // GPU information from their parent. If the parent wasn't updated, the thread's own `fd` folder
    // is read instead.
    pub(super) fn update_threads_gpu_info(
        proc_list: &mut HashMap<Pid, Process>,
        refresh_kind: ProcessRefreshKind,
        now: Instant,
        buffers: &mut ProcessBuffers,
    ) {
        // Same issue as `refresh_procs`: `proc_list` needs to be accessed from two places at once.
        // It's fine since the map itself isn't modified (and the thread and its parent are
        // different entries).
        let proc_list = Wrap(UnsafeCell::new(proc_list));
        for p in proc_list.get().values_mut() {
            let p = &mut p.inner;
            if !p.updated || !p.gpu_info.share_parent_fd_table {
                continue;
            }
            let parent_info = p
                .parent
                .filter(|parent| *parent != p.pid)
                .and_then(|parent| proc_list.get().get(&parent))
                .filter(|parent| parent.inner.updated)
                .map(|parent| {
                    (
                        parent.inner.gpu_info.gpu_usage,
                        parent.inner.gpu_info.memory,
                    )
                });
            let gpu_info = &mut p.gpu_info;
            if let Some((gpu_usage, memory)) = parent_info {
                if refresh_kind.gpu_usage() {
                    gpu_info.gpu_usage = gpu_usage;
                }
                if refresh_kind.gpu_memory() {
                    gpu_info.memory = memory;
                }
            } else {
                let path = super::to_c_string(p.proc_path.clone());
                compute_gpu_usage(
                    &mut ProcDir::new(&path),
                    gpu_info,
                    now,
                    refresh_kind,
                    buffers,
                );
            }
        }
    }

    #[inline(always)]
    #[allow(clippy::unnecessary_cast)]
    fn is_gpu_device(
        dir: &Dir,
        file_name: &[libc::c_char],
        stat: &mut MaybeUninit<libc::statx>,
    ) -> bool {
        unsafe {
            if libc::statx(
                dir.dir_fd,
                file_name.as_ptr(),
                libc::AT_STATX_DONT_SYNC | libc::AT_NO_AUTOMOUNT,
                libc::STATX_TYPE,
                stat.as_mut_ptr(),
            ) != 0
            {
                return false;
            }

            let stat = stat.assume_init_ref();
            (stat.stx_mode & libc::S_IFMT as libc::__u16) == libc::S_IFCHR as libc::__u16
                && matches!(stat.stx_rdev_major, DRM_MAJOR | ACCEL_MAJOR)
        }
    }

    struct Dir {
        dir_fd: libc::c_int,
    }

    impl Dir {
        fn open(proc_dir: &mut ProcDir<'_>, name: &CStr) -> Option<Self> {
            let proc_dir_fd = proc_dir.dir_fd()?;
            unsafe {
                let dir_fd = retry_eintr!(libc::openat(
                    proc_dir_fd.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC,
                ));
                if dir_fd < 0 {
                    None
                } else {
                    Some(Self { dir_fd })
                }
            }
        }

        fn update_dents_buf(&self, buf: &mut [u8]) -> Result<Option<usize>, ()> {
            unsafe {
                let read = libc::syscall(
                    libc::SYS_getdents64,
                    self.dir_fd,
                    buf.as_mut_ptr() as *mut _,
                    buf.len() as libc::c_int,
                );
                if read < 0 {
                    sysinfo_debug!("getdents64 failed");
                    Err(())
                } else if read == 0 {
                    Ok(None)
                } else {
                    Ok(Some(read as usize))
                }
            }
        }

        fn iter<'a>(&'a self, buf: &'a mut Vec<u8>) -> Result<Option<DirIter<'a>>, ()> {
            if let Some(read) = self.update_dents_buf(buf)? {
                Ok(Some(DirIter {
                    read,
                    pos: 0,
                    dir: self,
                    buf,
                }))
            } else {
                Ok(None)
            }
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            unsafe {
                libc::close(self.dir_fd);
            }
        }
    }

    struct DirIter<'a> {
        pos: usize,
        read: usize,
        dir: &'a Dir,
        // If we use an array instead of a Vec here, we get unaligned memory errors when we go
        // through the `dirent64` entries. The `Vec` is reused across processes.
        buf: &'a mut Vec<u8>,
    }

    impl<'a> Iterator for DirIter<'a> {
        type Item = &'a [libc::c_char];

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                unsafe {
                    if self.pos >= self.read {
                        if let Ok(Some(read)) = self.dir.update_dents_buf(self.buf) {
                            self.read = read;
                            self.pos = 0;
                        } else {
                            // Reached the end!
                            return None;
                        }
                    }
                    let dir_entry = self.buf.as_ptr().add(self.pos) as *const libc::dirent64;
                    let dir_entry = &*dir_entry;
                    self.pos += dir_entry.d_reclen as usize;

                    // It's only supposed to contain digits in any case, but that filters very well too.
                    if dir_entry.d_name[0] != b'.' as libc::c_char {
                        return Some(&dir_entry.d_name);
                    }
                }
            }
        }
    }

    // Maybe something to do in the future: if it's not an AMD or NVIDIA GPU, we can still compute the
    // total % usage by adding all processes GPU time. However: we need to have access to `Gpus` all the
    // time, so likely needs to be part of `System`, just like `Cpu`. Not really worth it since it comes
    // with limitations (such as: only gives information for current user, unless it's an admin), so for
    // now ignoring it.
    pub(super) fn compute_gpu_usage(
        proc_dir: &mut ProcDir<'_>,
        gpu_info: &mut GpuInfo,
        now: Instant,
        refresh_kind: ProcessRefreshKind,
        buffers: &mut ProcessBuffers,
    ) {
        use std::fs::File;

        let Some(fd_dir) = Dir::open(proc_dir, c"fd") else {
            return;
        };
        // Most processes don't have any GPU file descriptor, so we only open `fdinfo` once we
        // found one.
        let mut fdinfo_dir: Option<Dir> = None;

        let mut total_time: u64 = 0;
        let mut total_memory: u64 = 0;
        let mut found_memory = false;
        if let Ok(Some(dir_iter)) = fd_dir.iter(&mut buffers.dents) {
            let mut stat = MaybeUninit::<libc::statx>::uninit();
            let mut gpus: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

            for file_name in dir_iter {
                if !is_gpu_device(&fd_dir, file_name, &mut stat) {
                    continue;
                }
                let fdinfo_dir_fd = match fdinfo_dir {
                    Some(ref dir) => dir.dir_fd,
                    None => {
                        let Some(dir) = Dir::open(proc_dir, c"fdinfo") else {
                            return;
                        };
                        fdinfo_dir.insert(dir).dir_fd
                    }
                };
                let buf = unsafe {
                    let fd = retry_eintr!(libc::openat(
                        fdinfo_dir_fd,
                        file_name.as_ptr(),
                        libc::O_RDONLY,
                    ));
                    if fd < 0 {
                        continue;
                    }
                    // 4096 is the limit used in htop so why not.
                    const BUF_SIZE: usize = 4096;
                    let buf = &mut buffers.buf;
                    if buf.len() < BUF_SIZE {
                        assert!(buf.capacity() > BUF_SIZE);
                        // SAFETY: we create `ProcessBuffers::buf` with a capacity of `16_384`, so
                        // it's fine.
                        buf.set_len(BUF_SIZE);
                    }
                    if let mut file = File::from_raw_fd(fd)
                        // We pass a slice of maximum `BUF_SIZE` to never read more than
                        // "necessary".
                        && let Ok(read) = file.read(&mut buf[..BUF_SIZE])
                    {
                        &buf[..read]
                    } else {
                        continue;
                    }
                };
                let mut gpu_id = None;
                let mut pci = None;
                let mut gpu_time: Option<u64> = None;
                let mut gpu_memory: Option<u64> = None;
                // All the keys are listed in
                // <https://www.kernel.org/doc/html/latest/gpu/drm-usage-stats.html>.
                for line in buf
                    .split(|c| *c == b'\n')
                    .filter_map(|line| line.strip_prefix(b"drm-"))
                {
                    if line.starts_with(b"client-id:") {
                        if let Some(id) = line.splitn(2, |c| *c == b':').nth(1) {
                            gpu_id = Some(id.trim_ascii().to_vec());
                        }
                    } else if line.starts_with(b"pdev:") {
                        if let Some(dev) = line.splitn(2, |c| *c == b':').nth(1) {
                            pci = Some(dev.trim_ascii().to_vec());
                        }
                    } else if let Some(line) = line.strip_prefix(b"engine-") {
                        if refresh_kind.gpu_usage()
                            && !line.starts_with(b"capacity-")
                            && let Some(line) = line.strip_suffix(b" ns")
                            && let Some(nb) = line.split(|c| *c == b':').nth(1)
                            && let Some(nb) = parse_ascii_checked_u64(nb.trim_ascii())
                        {
                            *gpu_time.get_or_insert(0) += nb;
                        }
                    } else if let Some(line) = line.strip_prefix(b"total-") {
                        #[allow(clippy::collapsible_if)]
                        if refresh_kind.gpu_memory()
                            && let Some(nb) = line.splitn(2, |c| *c == b':').nth(1)
                            && let mut nb = nb
                                .split(|c| *c == b' ' || *c == b'\t')
                                .filter(|s| !s.is_empty())
                            && let Some(value) = nb.next()
                            && let Some(unit) = nb.next()
                            && !unit.is_empty()
                            && let Some(value) = parse_ascii_checked_u64(value.trim_ascii())
                        {
                            gpu_memory = match unit[0] {
                                b'K' | b'k' => Some(value * 1024),
                                b'M' | b'm' => Some(value * 1024 * 1024),
                                b'G' | b'g' => Some(value * 1024 * 1024 * 1024),
                                _ => {
                                    eprintln!(
                                        "Unknown GPU memory unit {unit:?} in {:?}",
                                        proc_dir.path
                                    );
                                    None
                                }
                            };
                        }
                    }
                }
                // This is the fallback in case the gpu memory or time wasn't retrieved.
                if let Some(gpu_id) = gpu_id
                    && let Some(pci) = pci
                    && !gpus
                        .iter()
                        .any(|(s_id, s_pci)| *s_id == gpu_id && *s_pci == pci)
                {
                    gpus.push((gpu_id, pci));
                    if let Some(gpu_time) = gpu_time {
                        total_time = total_time.saturating_add(gpu_time);
                    }
                    if let Some(gpu_memory) = gpu_memory {
                        total_memory = total_memory.saturating_add(gpu_memory);
                        found_memory = true;
                    }
                }
            }
        }
        if found_memory {
            gpu_info.memory = Some(total_memory);
        } else {
            gpu_info.memory = None;
        }
        if total_time != 0 {
            let elapsed_time = if let Some(last_update) = gpu_info.last_update {
                now.duration_since(last_update).as_millis()
            } else {
                0
            };
            let gpu_time_delta = total_time.saturating_sub(gpu_info.gpu_time);

            if gpu_time_delta == 0 || elapsed_time == 0 {
                gpu_info.gpu_usage = None;
            } else {
                // We need to convert from nanos to millis, hence the `/ 1_000_000.`.
                gpu_info.gpu_usage =
                    Some(100. * (gpu_time_delta as f32) / 1_000_000. / (elapsed_time as f32));
            }
            gpu_info.last_update = Some(now);
            gpu_info.gpu_time = total_time;
        } else {
            gpu_info.gpu_usage = Some(0.);
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
}

pub(crate) fn set_time(p: &mut ProcessInner, utime: u64, stime: u64) {
    p.old_utime = p.utime;
    p.old_stime = p.stime;
    p.utime = utime;
    p.stime = stime;
}

fn update_process_disk_activity(
    p: &mut ProcessInner,
    proc_dir: &mut ProcDir<'_>,
    buf: &mut Vec<u8>,
) {
    if !proc_dir.read_into(c"io", buf) {
        return;
    }
    let Ok(data) = std::str::from_utf8(buf) else {
        return;
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

/// Buffers reused across processes during a refresh, to avoid allocating for each of them.
struct ProcessBuffers {
    buf: Vec<u8>,
    /// Used to build the paths of the files to read.
    path: PathBuf,
    #[cfg(feature = "gpu")]
    dents: Vec<u8>,
}

impl ProcessBuffers {
    #[allow(clippy::uninit_vec)]
    fn new(
        #[cfg_attr(not(feature = "gpu"), allow(unused_variables))] refresh_kind: ProcessRefreshKind,
    ) -> Self {
        Self {
            buf: Vec::with_capacity(16_384),
            path: PathBuf::new(),
            #[cfg(feature = "gpu")]
            dents: {
                if refresh_kind.gpu_usage() || refresh_kind.gpu_memory() {
                    let mut data = Vec::with_capacity(std::mem::size_of::<libc::dirent64>() * 20);
                    // SAFETY: `data` is set by syscalls, so no need to initialize it ourselves.
                    unsafe {
                        data.set_len(data.capacity());
                    }
                    data
                } else {
                    Vec::new()
                }
            },
        }
    }
}

/// Gives access to the files of a `/proc/<pid>` folder. The folder is opened (lazily) once and
/// its files are then accessed relative to it, which avoids the kernel resolving the whole path
/// every time.
struct ProcDir<'a> {
    path: &'a CStr,
    dir_fd: Option<OwnedFd>,
}

impl<'a> ProcDir<'a> {
    fn new(path: &'a CStr) -> Self {
        Self { path, dir_fd: None }
    }

    fn dir_fd(&mut self) -> Option<&OwnedFd> {
        if let Some(ref dir_fd) = self.dir_fd {
            return Some(dir_fd);
        }
        let fd = retry_eintr!(unsafe {
            libc::open(
                self.path.as_ptr(),
                libc::O_RDONLY | libc::O_PATH | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        });
        if fd < 0 {
            sysinfo_debug!(
                "failed to open {:?}: {:?}",
                self.path,
                std::io::Error::last_os_error()
            );
            return None;
        }
        // SAFETY: `fd` was just opened and isn't owned by anything else.
        self.dir_fd = unsafe { Some(OwnedFd::from_raw_fd(fd)) };
        self.dir_fd.as_ref()
    }

    fn open(&mut self, name: &CStr) -> Option<File> {
        let dir_fd = self.dir_fd()?;
        let fd = retry_eintr!(unsafe {
            libc::openat(
                dir_fd.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC,
            )
        });
        if fd < 0 {
            sysinfo_debug!(
                "Failed to open file {name:?}: {:?}",
                std::io::Error::last_os_error()
            );
            None
        } else {
            // SAFETY: `fd` was just opened and isn't owned by anything else.
            Some(unsafe { File::from_raw_fd(fd) })
        }
    }

    /// Reads the whole content of `name` into `buf`, replacing its previous content. Returns
    /// `false` if an error occurred.
    fn read_into(&mut self, name: &CStr, buf: &mut Vec<u8>) -> bool {
        let Some(mut file) = self.open(name) else {
            return false;
        };
        buf.clear();
        if let Err(_err) = file.read_to_end(buf) {
            sysinfo_debug!("Failed to read file: {_err:?}");
            false
        } else {
            true
        }
    }

    /// Reads the target of the `name` link into `buf`, replacing its previous content.
    fn read_link<'b>(&mut self, name: &CStr, buf: &'b mut Vec<u8>) -> Option<&'b [u8]> {
        let dir_fd = self.dir_fd()?;
        buf.clear();
        if buf.len() < 256 {
            buf.reserve(256);
        }
        loop {
            let read = unsafe {
                libc::readlinkat(
                    dir_fd.as_raw_fd(),
                    name.as_ptr(),
                    buf.as_mut_ptr() as *mut _,
                    buf.capacity(),
                )
            };
            if read < 0 {
                sysinfo_debug!(
                    "failed to read link {name:?} in {:?}: {:?}",
                    self.path,
                    std::io::Error::last_os_error(),
                );
                return None;
            }
            let read = read as usize;
            if read < buf.capacity() {
                // SAFETY: `readlinkat` initialized the first `read` bytes.
                unsafe { buf.set_len(read) };
                return Some(buf);
            }
            // The link might have been truncated, so we try again with a bigger buffer.
            buf.reserve(buf.capacity() * 2);
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
unsafe impl<T> Send for Wrap<'_, T> {}
unsafe impl<T> Sync for Wrap<'_, T> {}

/// Reads the `stat` file into `buffers.buf`.
fn _get_stat_data(proc_dir: &mut ProcDir<'_>, buffers: &mut ProcessBuffers) -> Result<(), ()> {
    let Some(mut file) = proc_dir.open(c"stat") else {
        return Err(());
    };
    read_file_into(&mut file, &mut buffers.buf).map_err(|_| ())?;
    Ok(())
}

fn refresh_user_group_ids(
    p: &mut ProcessInner,
    ids: Option<Ids>,
    refresh_kind: ProcessRefreshKind,
) {
    if !refresh_kind.user().needs_update(|| p.user_id.is_none()) {
        return;
    }

    if let Some(Ids {
        user_id,
        effective_user_id,
        group_id,
        effective_group_id,
    }) = ids
    {
        p.user_id = Some(Uid(user_id));
        p.effective_user_id = Some(Uid(effective_user_id));
        p.group_id = Some(Gid(group_id));
        p.effective_group_id = Some(Gid(effective_group_id));
    }
}

fn split_content(data: &[u8]) -> Vec<OsString> {
    let mut out = Vec::with_capacity(10);
    for part in data.split(|c| *c == 0) {
        if !part.is_empty() {
            out.push(OsStr::from_bytes(part).to_os_string());
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn update_proc_info(
    p: &mut ProcessInner,
    parent_pid: Option<Pid>,
    refresh_kind: ProcessRefreshKind,
    proc_dir: &mut ProcDir<'_>,
    parts: &Parts,
    uptime: u64,
    info: &SystemInfo,
    ids: Option<Ids>,
    buffers: &mut ProcessBuffers,
    #[cfg(feature = "gpu")] now: Instant,
) {
    fn copy_from_file(proc_dir: &mut ProcDir<'_>, name: &CStr, buf: &mut Vec<u8>) -> Vec<OsString> {
        if proc_dir.read_into(name, buf) {
            split_content(buf)
        } else {
            Vec::new()
        }
    }

    fn update_optional_path(target: &mut Option<PathBuf>, new_val: Option<&[u8]>) {
        if let Some(new_val) = new_val
            && target
                .as_ref()
                .is_none_or(|target| target.as_os_str().as_bytes() != new_val)
        {
            *target = Some(PathBuf::from(OsStr::from_bytes(new_val)));
        }
    }

    update_parent_pid(p, parent_pid, parts);

    p.status = parts.status;
    refresh_user_group_ids(p, ids, refresh_kind);

    if refresh_kind.exe().needs_update(|| p.exe.is_none()) {
        // Do not use cmd[0] because it is not the same thing.
        // See https://github.com/GuillaumeGomez/sysinfo/issues/697.
        let new_exe = proc_dir.read_link(c"exe", &mut buffers.buf).map(|exe| {
            // If the target executable file was modified or removed, linux appends ` (deleted)`
            // at the end. We need to remove it.
            // See https://github.com/GuillaumeGomez/sysinfo/issues/1585.
            exe.strip_suffix(b" (deleted)").unwrap_or(exe)
        });
        update_optional_path(&mut p.exe, new_exe);
    }

    if refresh_kind.cmd().needs_update(|| p.cmd.is_empty()) {
        let new_cmd = copy_from_file(proc_dir, c"cmdline", &mut buffers.buf);
        if !new_cmd.is_empty() || p.cmd.is_empty() {
            p.cmd = new_cmd;
        }
    }
    if refresh_kind.environ().needs_update(|| p.environ.is_empty()) {
        let new_environ = copy_from_file(proc_dir, c"environ", &mut buffers.buf);
        if !new_environ.is_empty() || p.environ.is_empty() {
            p.environ = new_environ;
        }
    }
    if refresh_kind.cwd().needs_update(|| p.cwd.is_none()) {
        update_optional_path(&mut p.cwd, proc_dir.read_link(c"cwd", &mut buffers.buf));
    }
    if refresh_kind.root().needs_update(|| p.root.is_none()) {
        update_optional_path(&mut p.root, proc_dir.read_link(c"root", &mut buffers.buf));
    }

    update_time_and_memory(
        proc_dir,
        p,
        parts,
        uptime,
        info,
        refresh_kind,
        &mut buffers.buf,
    );
    if refresh_kind.disk_usage() {
        update_process_disk_activity(p, proc_dir, &mut buffers.buf);
    }
    // Needs to be after `update_time_and_memory`.
    if refresh_kind.cpu() {
        // The external values for CPU times are in "ticks", which are
        // scaled by "HZ", which is pegged externally at 100 ticks/second.
        p.accumulated_cpu_time =
            p.utime.saturating_add(p.stime).saturating_mul(1_000) / info.clock_cycle;
    }
    #[cfg(feature = "gpu")]
    if (refresh_kind.gpu_usage() || refresh_kind.gpu_memory())
        // Kernel threads don't have file descriptors, so no need to look for GPU ones.
        && p.thread_kind != Some(ThreadKind::Kernel)
    {
        p.gpu_info.share_parent_fd_table = p.thread_kind == Some(ThreadKind::Userland)
            && p.parent
                .is_some_and(|parent| self::gpu::same_fd_table(parent, p.pid));
        // If it's a thread sharing its parent process's file descriptor table, its GPU information
        // will be copied from it once all processes have been updated.
        if !p.gpu_info.share_parent_fd_table {
            self::gpu::compute_gpu_usage(proc_dir, &mut p.gpu_info, now, refresh_kind, buffers);
        }
    }
    p.updated = true;
    // A process can be marked as nonexistent (e.g. while hidden by `hidepid=invisible`) and
    // become readable again later.
    p.exists = true;
}

fn update_parent_pid(p: &mut ProcessInner, parent_pid: Option<Pid>, parts: &Parts) {
    p.parent = match parent_pid {
        Some(parent_pid) if parent_pid.0 != 0 => Some(parent_pid),
        _ => match parts.parent_pid {
            Some(p) if p != 0 => Some(Pid(p)),
            _ => None,
        },
    };
}

#[allow(clippy::too_many_arguments)]
fn retrieve_all_new_process_info(
    is_thread: bool,
    pid: Pid,
    parent_pid: Option<Pid>,
    parts: &mut Parts,
    proc_dir: &mut ProcDir<'_>,
    info: &SystemInfo,
    refresh_kind: ProcessRefreshKind,
    uptime: u64,
    ids: Option<Ids>,
    buffers: &mut ProcessBuffers,
    #[cfg(feature = "gpu")] now: Instant,
) -> Process {
    let mut p = ProcessInner::new(
        pid,
        PathBuf::from(OsStr::from_bytes(proc_dir.path.to_bytes())),
    );

    // To be noted that the start time is invalid here, it still needs to be converted into
    // "real" time.
    let start_time_without_boot_time = parts.start_time / info.clock_cycle;
    p.start_time_raw = parts.start_time;
    p.start_time_without_boot_time = start_time_without_boot_time;
    p.start_time = p
        .start_time_without_boot_time
        .saturating_add(info.boot_time);

    p.name = std::mem::take(&mut parts.short_exe);
    if parts
        .flags
        .is_some_and(|flags| flags & libc::PF_KTHREAD as c_ulong != 0)
    {
        p.thread_kind = Some(ThreadKind::Kernel);
    } else if is_thread {
        p.thread_kind = Some(ThreadKind::Userland);
    }

    update_proc_info(
        &mut p,
        parent_pid,
        refresh_kind,
        proc_dir,
        parts,
        uptime,
        info,
        ids,
        buffers,
        #[cfg(feature = "gpu")]
        now,
    );

    Process { inner: p }
}

#[allow(clippy::too_many_arguments)]
fn update_existing_process(
    is_thread: bool,
    proc: &mut Process,
    proc_dir: &mut ProcDir<'_>,
    parent_pid: Option<Pid>,
    uptime: u64,
    info: &SystemInfo,
    refresh_kind: ProcessRefreshKind,
    tasks: Option<HashSet<Pid>>,
    ids: Option<Ids>,
    buffers: &mut ProcessBuffers,
    #[cfg(feature = "gpu")] now: Instant,
) -> Result<Option<Process>, ()> {
    let entry = &mut proc.inner;
    // It's possible that the file descriptor is no longer valid in case the
    // original process was terminated and another one took its place.
    _get_stat_data(proc_dir, buffers)?;
    entry.tasks = tasks;

    // `parts` borrows the `stat` buffer while `buffers` is still needed, so we take it out and put
    // it back once done.
    let Some(mut parts) = parse_stat_file(&buffers.buf, Some(entry.start_time_raw)) else {
        return Err(());
    };

    // It's possible that a new process took this same PID when the "original one" terminated.
    // If the start time differs, then it means it's not the same process anymore and that we
    // need to get all its information, hence why we check it here.
    if parts.start_time == entry.start_time_raw {
        // If the entry was first discovered without thread info
        // (e.g. in ProcessesToUpdate::All mode), fix its thread_kind now.
        if is_thread && entry.thread_kind.is_none() {
            entry.thread_kind = Some(ThreadKind::Userland);
        }

        update_proc_info(
            entry,
            parent_pid,
            refresh_kind,
            proc_dir,
            &parts,
            uptime,
            info,
            ids,
            buffers,
            #[cfg(feature = "gpu")]
            now,
        );
        return Ok(None);
    }
    // If we're here, it means that the PID still exists but it's a different process.
    let p = retrieve_all_new_process_info(
        is_thread,
        entry.pid,
        parent_pid,
        &mut parts,
        proc_dir,
        info,
        refresh_kind,
        uptime,
        ids,
        buffers,
        #[cfg(feature = "gpu")]
        now,
    );
    *proc = p;
    // Since this PID is already in the HashMap, no need to add it again.
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
fn _get_process_data(
    path: &CStr,
    proc_list: &mut HashMap<Pid, Process>,
    pid: Pid,
    is_thread: bool,
    parent_pid: Option<Pid>,
    uptime: u64,
    info: &SystemInfo,
    refresh_kind: ProcessRefreshKind,
    tasks: Option<HashSet<Pid>>,
    ids: Option<Ids>,
    buffers: &mut ProcessBuffers,
    #[cfg(feature = "gpu")] now: Instant,
) -> Result<Option<Process>, ()> {
    let mut proc_dir = ProcDir::new(path);
    if let Some(ref mut entry) = proc_list.get_mut(&pid) {
        return update_existing_process(
            is_thread,
            entry,
            &mut proc_dir,
            parent_pid,
            uptime,
            info,
            refresh_kind,
            tasks,
            ids,
            buffers,
            #[cfg(feature = "gpu")]
            now,
        );
    }
    _get_stat_data(&mut proc_dir, buffers)?;
    // `parts` borrows the `stat` buffer while `buffers` is still needed, so we take it out and put
    // it back once done.
    let Some(mut parts) = parse_stat_file(&buffers.buf, None) else {
        return Err(());
    };

    let mut new_process = retrieve_all_new_process_info(
        is_thread,
        pid,
        parent_pid,
        &mut parts,
        &mut proc_dir,
        info,
        refresh_kind,
        uptime,
        ids,
        buffers,
        #[cfg(feature = "gpu")]
        now,
    );
    new_process.inner.tasks = tasks;
    Ok(Some(new_process))
}

fn old_get_memory(entry: &mut ProcessInner, parts: &Parts, info: &SystemInfo) {
    // rss
    entry.memory = parts.resident_set_size.saturating_mul(info.page_size_b);
    // vsz correspond to the Virtual memory size in bytes.
    // see: https://man7.org/linux/man-pages/man5/proc.5.html
    entry.virtual_memory = parts.virtual_size
}

fn slice_to_nb(s: &[u8]) -> u64 {
    let mut nb: u64 = 0;

    for c in s {
        nb = nb * 10 + (c - b'0') as u64;
    }
    nb
}

fn get_memory(
    proc_dir: &mut ProcDir<'_>,
    entry: &mut ProcessInner,
    info: &SystemInfo,
    buf: &mut Vec<u8>,
) -> bool {
    if !proc_dir.read_into(c"statm", buf) {
        return false;
    }
    let mut parts = buf.split(|c| *c == b' ');
    entry.virtual_memory = parts
        .next()
        .map(slice_to_nb)
        .unwrap_or(0)
        .saturating_mul(info.page_size_b);
    entry.memory = parts
        .next()
        .map(slice_to_nb)
        .unwrap_or(0)
        .saturating_mul(info.page_size_b);
    true
}

#[allow(clippy::too_many_arguments)]
fn update_time_and_memory(
    proc_dir: &mut ProcDir<'_>,
    entry: &mut ProcessInner,
    parts: &Parts,
    uptime: u64,
    info: &SystemInfo,
    refresh_kind: ProcessRefreshKind,
    buf: &mut Vec<u8>,
) {
    {
        #[allow(clippy::collapsible_if)]
        if refresh_kind.memory() {
            // Keeping this nested level for readability reasons.
            if !get_memory(proc_dir, entry, info, buf) {
                old_get_memory(entry, parts, info);
            }
        }
        set_time(entry, parts.user_time, parts.system_time);
        entry.run_time = uptime.saturating_sub(entry.start_time_without_boot_time);
    }
}

struct ProcAndTasks {
    pid: Pid,
    parent_pid: Option<Pid>,
    path: CString,
    tasks: Option<HashSet<Pid>>,
    is_thread: bool,
    ids: Option<Ids>,
}

#[cfg(feature = "multithread")]
#[inline]
pub(crate) fn iter<T>(val: T) -> rayon::iter::IterBridge<T>
where
    T: rayon::iter::ParallelBridge,
{
    val.par_bridge()
}

#[cfg(not(feature = "multithread"))]
#[inline]
pub(crate) fn iter<T>(val: T) -> T
where
    T: Iterator,
{
    val
}

/// We're forced to read the whole `/proc` folder because if a process died and another took its
/// place, we need to get the task parent (if it's a task).
pub(crate) fn refresh_procs(
    proc_list: &mut HashMap<Pid, Process>,
    proc_path: &Path,
    uptime: u64,
    info: &SystemInfo,
    processes_to_update: ProcessesToUpdate<'_>,
    refresh_kind: ProcessRefreshKind,
) -> usize {
    #[cfg(feature = "multithread")]
    use rayon::iter::ParallelIterator;

    let nb_updated = AtomicUsize::new(0);
    #[cfg(feature = "gpu")]
    let now = Instant::now();
    let mut buffers = ProcessBuffers::new(refresh_kind);

    // This code goes through processes (listed in `/proc`) and through tasks (listed in
    // `/proc/[PID]/task`). However, the stored tasks information is supposed to be already present
    // in the PIDs listed from `/proc` so there will be no duplicates between PIDs and tasks PID.
    //
    // If a task is not listed in `/proc`, then we don't retrieve its information.
    //
    // So in short: since we update the `HashMap` itself by adding/removing entries outside of the
    // parallel iterator, we can safely use it inside the parallel iterator and update its entries
    // concurrently.
    let procs = {
        let pid_iter: Box<dyn Iterator<Item = (PathBuf, Pid)> + Send> = match processes_to_update {
            ProcessesToUpdate::All => match read_dir(proc_path) {
                Ok(proc_entries) => Box::new(proc_entries.filter_map(filter_pid_entries)),
                Err(_err) => {
                    sysinfo_debug!("Failed to read folder {proc_path:?}: {_err:?}");
                    return 0;
                }
            },
            ProcessesToUpdate::Some(pids) => Box::new(
                pids.iter()
                    .map(|pid| (proc_path.join(pid.to_string()), *pid)),
            ),
        };

        let proc_list = Wrap(UnsafeCell::new(proc_list));

        let get_process_data = |buffers: &mut ProcessBuffers, e: ProcAndTasks| {
            let proc_list = proc_list.get();
            let new_process = _get_process_data(
                &e.path,
                proc_list,
                e.pid,
                e.is_thread,
                e.parent_pid,
                uptime,
                info,
                refresh_kind,
                e.tasks,
                e.ids,
                buffers,
                #[cfg(feature = "gpu")]
                now,
            )
            .ok()?;
            nb_updated.fetch_add(1, Ordering::Relaxed);
            new_process
        };
        let get_procs = |buffers: &mut ProcessBuffers, (path, pid): (PathBuf, Pid)| {
            get_proc_and_tasks(path, pid, refresh_kind, processes_to_update, buffers)
        };

        // Each rayon job gets its own buffers.
        #[cfg(feature = "multithread")]
        let procs = iter(pid_iter)
            .map_init(|| ProcessBuffers::new(refresh_kind), get_procs)
            .flatten()
            .map_init(|| ProcessBuffers::new(refresh_kind), get_process_data)
            .flatten();
        #[cfg(not(feature = "multithread"))]
        let procs = {
            let mut procs_buffers = ProcessBuffers::new(refresh_kind);
            let buffers = &mut buffers;
            iter(pid_iter)
                .flat_map(move |e| get_procs(&mut procs_buffers, e))
                .filter_map(move |e| get_process_data(buffers, e))
        };
        procs.collect::<Vec<_>>()
    };
    for proc_ in procs {
        proc_list.insert(proc_.pid(), proc_);
    }
    #[cfg(feature = "gpu")]
    if refresh_kind.gpu_usage() || refresh_kind.gpu_memory() {
        gpu::update_threads_gpu_info(proc_list, refresh_kind, now, &mut buffers);
    }
    nb_updated.into_inner()
}

fn filter_pid_entries(entry: Result<DirEntry, std::io::Error>) -> Option<(PathBuf, Pid)> {
    if let Ok(entry) = entry
        && entry.file_type().is_ok_and(|file_type| file_type.is_dir())
        && let Some(pid) = parse_ascii_checked_usize(entry.file_name().as_bytes())
    {
        Some((entry.path(), Pid::from(pid)))
    } else {
        None
    }
}

fn get_proc_and_tasks(
    path: PathBuf,
    pid: Pid,
    refresh_kind: ProcessRefreshKind,
    processes_to_update: ProcessesToUpdate<'_>,
    buffers: &mut ProcessBuffers,
) -> Vec<ProcAndTasks> {
    let mut parent_pid = None;
    let mut is_thread = false;

    let (ids, tgid) = match get_uid_and_gid_and_tgid(&path, buffers) {
        Some((ids, tgid)) => (Some(ids), Some(tgid)),
        None => (None, None),
    };
    let is_a_task = tgid.is_some_and(|tgid| tgid != pid);
    // Threads don't have meaningful tasks, so no need to fetch `procs` and `tasks`.
    let (mut procs, tasks) = if refresh_kind.tasks() && !is_a_task {
        let update_processes_list = processes_to_update == ProcessesToUpdate::All;
        // The `status` file allows to retrieve user info and tgid. If we're not interested in any,
        // then we don't read it.
        let retrieve_ids = update_processes_list && refresh_kind.user() != UpdateKind::Never;
        let mut procs = get_proc_tasks(&path, pid, retrieve_ids, buffers);
        let tasks = procs.iter().map(|ProcAndTasks { pid, .. }| *pid).collect();

        if !update_processes_list {
            // Don't add the tasks to the list of processes to update.
            procs.clear();
        }
        (procs, Some(tasks))
    } else {
        (Vec::new(), None)
    };

    // If the process' tgid doesn't match its pid, it is a task (thread).
    // This check must apply in ALL modes, not just `Some`.
    if is_a_task {
        parent_pid = tgid;
        is_thread = true;
    }

    procs.push(ProcAndTasks {
        is_thread,
        pid,
        parent_pid,
        path: to_c_string(path),
        tasks,
        ids,
    });

    procs
}

fn to_c_string(path: PathBuf) -> CString {
    let mut path = path.into_os_string().into_vec();
    path.push(0);
    // SAFETY: Paths cannot contain NUL bytes and we added one at the end.
    unsafe { CString::from_vec_with_nul_unchecked(path) }
}

fn get_proc_tasks(
    path: &Path,
    parent_pid: Pid,
    retrieve_ids: bool,
    buffers: &mut ProcessBuffers,
) -> Vec<ProcAndTasks> {
    let task_path = path.join("task");

    read_dir(&task_path)
        .ok()
        .map(|task_entries| {
            task_entries
                .filter_map(filter_pid_entries)
                // Needed because tasks have their own PID listed in the "task" folder.
                .filter(|(_, pid)| *pid != parent_pid)
                .map(|(path, pid)| {
                    let ids = if retrieve_ids {
                        get_uid_and_gid_and_tgid(&path, buffers).map(|(ids, _)| ids)
                    } else {
                        None
                    };
                    ProcAndTasks {
                        pid,
                        is_thread: true,
                        path: to_c_string(path),
                        parent_pid: Some(parent_pid),
                        tasks: None,
                        ids,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) struct Ids {
    user_id: uid_t,
    effective_user_id: uid_t,
    group_id: gid_t,
    effective_group_id: gid_t,
}

// Fetch real and effective UID and GID and store them into `Pids`. Returns the "thread group id"
// (tgid) as second value of the tuple.
fn get_uid_and_gid_and_tgid(proc_path: &Path, buffers: &mut ProcessBuffers) -> Option<(Ids, Pid)> {
    /// Writes `{base}/{file_name}` into `buf` and returns a reference to it.
    fn build_path<'a>(buf: &'a mut PathBuf, base: &Path, file_name: &str) -> &'a Path {
        buf.as_mut_os_string().clear();
        buf.push(base);
        buf.push(file_name);
        buf
    }

    let status_path = build_path(&mut buffers.path, proc_path, "status");
    read_path_into(status_path, &mut buffers.buf).ok()?;
    let status_data = &buffers.buf;

    // We're only interested in the lines starting with Uid: and Gid:
    // here. From these lines, we're looking at the first and second entries to get
    // the real u/gid.

    let f = |h: &[u8], n: &[u8]| -> (Option<uid_t>, Option<uid_t>) {
        if let Some(h) = h.strip_prefix(n) {
            let mut ids = h
                .split(|c| *c == b'\t' || *c == b' ')
                .filter(|s| !s.is_empty());
            let real = ids.next().and_then(parse_ascii_checked_uid_t);
            let effective = ids.next().and_then(parse_ascii_checked_uid_t);

            (real, effective)
        } else {
            (None, None)
        }
    };
    let mut uid = None;
    let mut effective_uid = None;
    let mut gid = None;
    let mut effective_gid = None;
    let mut tgid = None;
    for line in status_data.split(|c| *c == b'\n') {
        if let (Some(real), Some(effective)) = f(line, b"Uid:") {
            debug_assert!(uid.is_none() && effective_uid.is_none());
            uid = Some(real);
            effective_uid = Some(effective);
        } else if let (Some(real), Some(effective)) = f(line, b"Gid:") {
            debug_assert!(gid.is_none() && effective_gid.is_none());
            gid = Some(real);
            effective_gid = Some(effective);
        } else if let Some(line) = line.strip_prefix(b"Tgid:")
            && let Some(real) = parse_ascii_checked_pid_t(line.trim_ascii())
        {
            debug_assert!(tgid.is_none());
            tgid = Some(real);
        } else {
            continue;
        }
        if uid.is_some() && gid.is_some() && tgid.is_some() {
            break;
        }
    }
    match (uid, effective_uid, gid, effective_gid, tgid) {
        (
            Some(user_id),
            Some(effective_user_id),
            Some(group_id),
            Some(effective_group_id),
            Some(thread_group_id),
        ) => Some((
            Ids {
                user_id,
                effective_user_id,
                group_id,
                effective_group_id,
            },
            Pid(thread_group_id),
        )),
        _ => None,
    }
}

#[cfg_attr(test, derive(PartialEq, Debug))]
struct Parts {
    short_exe: OsString,
    status: ProcessStatus,
    parent_pid: Option<pid_t>,
    flags: Option<c_ulong>,
    user_time: u64,
    system_time: u64,
    start_time: u64,
    virtual_size: u64,
    resident_set_size: u64,
}

fn parse_stat_file(data: &[u8], start_time_raw: Option<u64>) -> Option<Parts> {
    // The stat file is "interesting" to parse, because spaces cannot
    // be used as delimiters. The second field stores the command name
    // surrounded by parentheses. Unfortunately, whitespace and
    // parentheses are legal parts of the command, so parsing has to
    // proceed like this: The first field is delimited by the first
    // whitespace, the second field is everything until the last ')'
    // in the entire string. All other fields are delimited by
    // whitespace.

    // We ignore the first field (`pid`).
    let data = data.splitn(2, |&b| b == b' ').nth(1)?;
    let pos = data
        .iter()
        .rposition(|&b| b == b')')
        .or_else(|| data.iter().rposition(|&b| b == b')'))?;
    let short_exe = &data[1..pos];

    let mut data = data[pos + 1..]
        .split(|c| *c == b' ')
        .filter(|p| !p.is_empty());

    // This code is awful, but couldn't find a better way. We ensure that the parsing is done
    // correctly with the `test_parse_stat_file` test below.
    let status = data
        .next()
        .and_then(|part| part.first().copied().map(ProcessStatus::from))
        .unwrap_or(ProcessStatus::Unknown(0));
    let parent_pid = data.next().and_then(parse_ascii_checked_pid_t);

    let flags = data
        .nth(ProcIndex::Flags as usize - ProcIndex::ParentPid as usize - 1)
        .and_then(parse_ascii_checked_culong);
    let user_time = data
        .nth(ProcIndex::UserTime as usize - ProcIndex::Flags as usize - 1)
        .and_then(parse_ascii_checked_u64)
        .unwrap_or(0);
    let system_time = data.next().and_then(parse_ascii_checked_u64).unwrap_or(0);

    let start_time = data
        .nth(ProcIndex::StartTime as usize - ProcIndex::SystemTime as usize - 1)
        .and_then(parse_ascii_checked_u64)
        .unwrap_or(0);
    let virtual_size = data.next().and_then(parse_ascii_checked_u64).unwrap_or(0);
    let resident_set_size = data.next().and_then(parse_ascii_checked_u64).unwrap_or(0);

    Some(Parts {
        status,
        parent_pid,
        flags,
        user_time,
        system_time,
        start_time,
        virtual_size,
        resident_set_size,
        short_exe: if start_time_raw.is_some_and(|raw| start_time == raw) {
            OsString::new()
        } else {
            OsStr::from_bytes(short_exe).to_os_string()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::{Parts, parse_stat_file, split_content};
    use std::ffi::OsString;

    // A process marked as nonexistent (e.g. hidden by `hidepid=invisible` for one refresh) must
    // be reported as existing again once it can be read.
    #[test]
    fn test_exists_after_being_nonexistent() {
        use crate::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

        let pid = Pid::from_u32(std::process::id());
        let mut s = System::new().unwrap();
        s.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);
        s.process(pid).unwrap();
        s.inner
            .processes_mut()
            .get_mut(&pid)
            .unwrap()
            .inner
            .set_nonexistent();
        assert!(!s.process(pid).unwrap().exists());
        s.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            false,
            ProcessRefreshKind::nothing(),
        );
        assert!(s.process(pid).unwrap().exists());
    }

    // This test ensures that all the parts of the data are split.
    #[test]
    fn test_copy_file() {
        assert_eq!(split_content(b"hello\0"), vec![OsString::from("hello")]);
        assert_eq!(split_content(b"hello"), vec![OsString::from("hello")]);
        assert_eq!(
            split_content(b"hello\0b"),
            vec![OsString::from("hello"), "b".into()]
        );
        assert_eq!(
            split_content(b"hello\0\0\0\0b"),
            vec![OsString::from("hello"), "b".into()]
        );
    }

    #[test]
    fn test_parse_stat_file() {
        // The (trimmed) content of a stat file.
        let content = b"1 (blob) S 2 0 0 0 -1 2129984 0 0 0 0 14 28 0 0 20 0 1 0 21 66 77";
        let data = parse_stat_file(content, None).unwrap();
        assert_eq!(
            data,
            Parts {
                short_exe: "blob".into(),
                status: crate::ProcessStatus::Sleep,
                parent_pid: Some(2),
                flags: Some(2129984),
                user_time: 14,
                start_time: 21,
                system_time: 28,
                virtual_size: 66,
                resident_set_size: 77,
            }
        );
    }

    // This test ensures that even if we have a `(`/`)` char in the short exe name, we still make
    // it works.
    #[test]
    fn test_parse_stat_file_short_exe() {
        // The (trimmed) content of a stat file.
        let content = b"1 (bl()ob) S 2 0 0 0 -1 2129984 0 0 0 0 14 28 0 0 20 0 1 0 21 66 77";
        let data = parse_stat_file(content, None).unwrap();
        assert_eq!(
            data,
            Parts {
                short_exe: "bl()ob".into(),
                status: crate::ProcessStatus::Sleep,
                parent_pid: Some(2),
                flags: Some(2129984),
                user_time: 14,
                start_time: 21,
                system_time: 28,
                virtual_size: 66,
                resident_set_size: 77,
            }
        );
    }

    #[test]
    fn test_parse_stat_file_long_exe() {
        // In case you wonder: yes, it's a real "short" exe name.
        let content = b"1 (nvidia-modeset/deferred_close_kthread_q) S 2 0 0 0 -1 2129984 0 0 0 0 14 28 0 0 20 0 1 0 21 66 77";
        let data = parse_stat_file(content, None).unwrap();
        assert_eq!(
            data,
            Parts {
                short_exe: "nvidia-modeset/deferred_close_kthread_q".into(),
                status: crate::ProcessStatus::Sleep,
                parent_pid: Some(2),
                flags: Some(2129984),
                user_time: 14,
                start_time: 21,
                system_time: 28,
                virtual_size: 66,
                resident_set_size: 77,
            }
        );
    }
}
