// Take a look at the license at the top of the repository in the LICENSE file.

use std::cell::UnsafeCell;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, DirEntry, File, read_dir};
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(feature = "gpu")]
use std::time::Instant;

use libc::{c_ulong, gid_t, uid_t};

use crate::sys::system::SystemInfo;
use crate::sys::utils::{PathHandler, PathPush, get_all_data_from_file, get_all_utf8_data};
use crate::unix::utils::realpath;
use crate::{
    DiskUsage, Gid, Pid, Process, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, Signal,
    ThreadKind, Uid,
};

use crate::sys::system::remaining_files;

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
    stat_file: Option<FileCounter>,
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
            stat_file: None,
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
        let (data, _) = _get_stat_data_and_file(&self.proc_path).ok()?;
        let parts = parse_stat_file(&data)?;

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

fn parse_ascii_checked_u64(bytes: &[u8]) -> Option<u64> {
    let mut num: u64 = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        num = num.checked_mul(10)?.checked_add((b - b'0') as u64)?;
    }
    Some(num)
}

// Yes, it's ugly to duplicate this code and makes me very sad... I could implement a trait for
// both `c_ulong` and `u64`. However, it's possible on some platforms that `c_ulong` and `u64` are
// the same type, so implementing this trait would fail compilation. Would be much simpler if all
// integers implemented `checked_` into a common trait instead...
fn parse_ascii_checked_culong(bytes: &[u8]) -> Option<c_ulong> {
    let mut num: c_ulong = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        num = num.checked_mul(10)?.checked_add((b - b'0') as c_ulong)?;
    }
    Some(num)
}

fn parse_ascii_checked_usize(bytes: &[u8]) -> Option<usize> {
    let mut num: usize = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        num = num.checked_mul(10)?.checked_add((b - b'0') as usize)?;
    }
    Some(num)
}

fn parse_ascii_checked_pid_t(bytes: &[u8]) -> Option<Pid> {
    let mut num: libc::pid_t = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        num = num
            .checked_mul(10)?
            .checked_add((b - b'0') as libc::pid_t)?;
    }
    Some(Pid(num))
}

#[cfg(feature = "gpu")]
mod gpu {
    use super::*;

    // Faster `readlink` implementation which skips allocations by reusing a same buffer.
    #[inline(always)]
    fn read_link(dir: &Dir, file_name: &[libc::c_char], buf: &mut [u8]) -> Option<usize> {
        unsafe {
            let res = libc::readlinkat(
                dir.dir_fd,
                file_name.as_ptr(),
                buf.as_mut_ptr() as *mut libc::c_char,
                buf.len(),
            );

            if res < 1 { None } else { Some(res as usize) }
        }
    }

    struct Dir {
        dir_fd: libc::c_int,
    }

    impl Dir {
        fn new(path: &[u8]) -> Option<Self> {
            unsafe {
                let dir_fd = retry_eintr!(libc::open(
                    path.as_ptr() as *const _,
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

        #[allow(clippy::uninit_vec)]
        fn iter(&self) -> Result<Option<DirIter<'_>>, ()> {
            // 20 dir entries at once should be enough.
            let mut buf = Vec::with_capacity(std::mem::size_of::<libc::dirent64>() * 20);
            // SAFETY: Data is set by syscalls, so no need to initialize it ourselves.
            unsafe {
                buf.set_len(buf.capacity());
            }
            if let Some(read) = self.update_dents_buf(&mut buf)? {
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
        // through the `dirent64` entries. So sadly, we need to go through the allocation...
        buf: Vec<u8>,
    }

    impl<'a> Iterator for DirIter<'a> {
        type Item = &'a [libc::c_char];

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                unsafe {
                    if self.pos >= self.read {
                        if let Ok(Some(read)) = self.dir.update_dents_buf(&mut self.buf) {
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
    pub fn compute_gpu_usage(
        proc_path: &mut PathHandler,
        gpu_info: &mut GpuInfo,
        now: Instant,
        refresh_kind: ProcessRefreshKind,
    ) {
        use std::fs::File;
        use std::mem::MaybeUninit;
        use std::os::fd::FromRawFd;

        // CString is apparently expensive, so we do our own...
        let path = proc_path.replace_and_join("fdinfo").as_os_str().as_bytes();
        let mut c_path = Vec::with_capacity(path.len() + 1);
        c_path.extend_from_slice(path);
        c_path.push(0);
        let Some(dir) = Dir::new(&c_path) else { return };

        let mut total_time: u64 = 0;
        let mut total_memory: u64 = 0;
        let mut found_memory = false;
        let dir_fd = dir.dir_fd;
        if let Ok(Some(dir_iter)) = dir.iter()
            && let Some(fd_dir) = {
                // We replace `/fdinfo\0` with `/fd\0nfo\0` to the folder name becomes `fd`.
                // So 4 characters for `info` and 1 for the `\0`.
                let index = c_path.len() - 5;
                c_path[index] = 0;
                Dir::new(&c_path)
            }
        {
            // 4096 is the limit used in htop so why not.
            let buf: MaybeUninit<[u8; 4096]> = MaybeUninit::uninit();
            // SAFETY: `read_link` and `openat` will initialize the values.
            let mut buf: [u8; 4096] = unsafe { buf.assume_init() };
            let mut gpus: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

            for file_name in dir_iter {
                // SAFETY: `d_name` is always valid UTF8 if it comes from `getdents64`/`readdir`
                // otherwise rust always provide valid UTF8 strings.

                let Some(size) = read_link(&fd_dir, file_name, &mut buf) else {
                    continue;
                };
                let target_bytes = &buf[..size];
                if !matches!(
                    target_bytes.strip_prefix(b"/dev/"),
                    Some(part) if part.starts_with(b"dri/") || part.starts_with(b"accel/")
                ) {
                    continue;
                }
                let buf = unsafe {
                    let fd = retry_eintr!(libc::openat(dir_fd, file_name.as_ptr(), libc::O_RDONLY));
                    if fd < 0 {
                        continue;
                    }
                    if let mut file = File::from_raw_fd(fd)
                        && let Ok(read) = file.read(&mut buf)
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
                                        proc_path.as_path()
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

pub(crate) fn update_process_disk_activity(p: &mut ProcessInner, path: &mut PathHandler) {
    let data = match get_all_utf8_data(path.replace_and_join("io"), 16_384) {
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
unsafe impl<T> Send for Wrap<'_, T> {}
unsafe impl<T> Sync for Wrap<'_, T> {}

fn _get_stat_data_and_file(path: &Path) -> Result<(Vec<u8>, File), ()> {
    let mut file = File::open(path.join("stat")).map_err(|_| ())?;
    let data = get_all_data_from_file(&mut file, 1024).map_err(|_| ())?;
    Ok((data, file))
}

fn _get_stat_data(path: &Path, stat_file: &mut Option<FileCounter>) -> Result<Vec<u8>, ()> {
    let (data, file) = _get_stat_data_and_file(path)?;
    *stat_file = FileCounter::new(file);
    Ok(data)
}

fn refresh_user_group_ids(
    p: &mut ProcessInner,
    path: &mut PathHandler,
    refresh_kind: ProcessRefreshKind,
) {
    if !refresh_kind.user().needs_update(|| p.user_id.is_none()) {
        return;
    }

    if let Some(((user_id, effective_user_id), (group_id, effective_group_id))) =
        get_uid_and_gid(path.replace_and_join("status"))
    {
        p.user_id = Some(Uid(user_id));
        p.effective_user_id = Some(Uid(effective_user_id));
        p.group_id = Some(Gid(group_id));
        p.effective_group_id = Some(Gid(effective_group_id));
    }
}

/// Only overwrite if the new value is `Some` or the old value was `None`,
/// to avoid wiping previously-read data if the process terminated mid-refresh.
fn update_optional_path(target: &mut Option<PathBuf>, path: &std::path::Path) {
    let new_val = realpath(path);
    if new_val.is_some() || target.is_none() {
        *target = new_val;
    }
}

#[allow(clippy::too_many_arguments)]
fn update_proc_info(
    p: &mut ProcessInner,
    parent_pid: Option<Pid>,
    refresh_kind: ProcessRefreshKind,
    proc_path: &mut PathHandler,
    parts: &Parts<'_>,
    uptime: u64,
    info: &SystemInfo,
    #[cfg(feature = "gpu")] now: Instant,
) {
    update_parent_pid(p, parent_pid, parts);

    p.status = parts.status;
    refresh_user_group_ids(p, proc_path, refresh_kind);

    if refresh_kind.exe().needs_update(|| p.exe.is_none()) {
        // Do not use cmd[0] because it is not the same thing.
        // See https://github.com/GuillaumeGomez/sysinfo/issues/697.
        let new_exe = realpath(proc_path.replace_and_join("exe"));
        // Avoid overwriting with None if the process terminated mid-refresh.
        if new_exe.is_some() || p.exe.is_none() {
            p.exe = new_exe;
            // If the target executable file was modified or removed, linux appends ` (deleted)`
            // at the end. We need to remove it.
            // See https://github.com/GuillaumeGomez/sysinfo/issues/1585.
            let deleted = b" (deleted)";
            if let Some(exe) = &mut p.exe
                && let Some(file_name) = exe.file_name()
                && file_name.as_encoded_bytes().ends_with(deleted)
            {
                let mut file_name = file_name.as_encoded_bytes().to_vec();
                file_name.truncate(file_name.len() - deleted.len());
                unsafe {
                    exe.set_file_name(OsString::from_encoded_bytes_unchecked(file_name));
                }
            }
        }
    }

    if refresh_kind.cmd().needs_update(|| p.cmd.is_empty()) {
        let new_cmd = copy_from_file(proc_path.replace_and_join("cmdline"));
        if !new_cmd.is_empty() || p.cmd.is_empty() {
            p.cmd = new_cmd;
        }
    }
    if refresh_kind.environ().needs_update(|| p.environ.is_empty()) {
        let new_environ = copy_from_file(proc_path.replace_and_join("environ"));
        if !new_environ.is_empty() || p.environ.is_empty() {
            p.environ = new_environ;
        }
    }
    if refresh_kind.cwd().needs_update(|| p.cwd.is_none()) {
        update_optional_path(&mut p.cwd, proc_path.replace_and_join("cwd"));
    }
    if refresh_kind.root().needs_update(|| p.root.is_none()) {
        update_optional_path(&mut p.root, proc_path.replace_and_join("root"));
    }

    update_time_and_memory(proc_path, p, parts, uptime, info, refresh_kind);
    if refresh_kind.disk_usage() {
        update_process_disk_activity(p, proc_path);
    }
    // Needs to be after `update_time_and_memory`.
    if refresh_kind.cpu() {
        // The external values for CPU times are in "ticks", which are
        // scaled by "HZ", which is pegged externally at 100 ticks/second.
        p.accumulated_cpu_time =
            p.utime.saturating_add(p.stime).saturating_mul(1_000) / info.clock_cycle;
    }
    #[cfg(feature = "gpu")]
    if refresh_kind.gpu_usage() || refresh_kind.gpu_memory() {
        self::gpu::compute_gpu_usage(proc_path, &mut p.gpu_info, now, refresh_kind);
    }
    p.updated = true;
}

fn update_parent_pid(p: &mut ProcessInner, parent_pid: Option<Pid>, parts: &Parts<'_>) {
    p.parent = match parent_pid {
        Some(parent_pid) if parent_pid.0 != 0 => Some(parent_pid),
        _ => match parts.parent_pid.and_then(parse_ascii_checked_pid_t) {
            Some(p) if p.0 != 0 => Some(p),
            _ => None,
        },
    };
}

#[allow(clippy::too_many_arguments)]
fn retrieve_all_new_process_info(
    is_thread: bool,
    pid: Pid,
    parent_pid: Option<Pid>,
    parts: &Parts<'_>,
    path: &Path,
    info: &SystemInfo,
    refresh_kind: ProcessRefreshKind,
    uptime: u64,
    #[cfg(feature = "gpu")] now: Instant,
) -> Process {
    let mut p = ProcessInner::new(pid, path.to_owned());
    let mut proc_path = PathHandler::new(path);
    let name = parts.short_exe;

    // To be noted that the start time is invalid here, it still needs to be converted into
    // "real" time.
    let start_time_without_boot_time = parts.start_time / info.clock_cycle;
    p.start_time_raw = parts.start_time;
    p.start_time_without_boot_time = start_time_without_boot_time;
    p.start_time = p
        .start_time_without_boot_time
        .saturating_add(info.boot_time);

    p.name = OsStr::from_bytes(name).to_os_string();
    if let Some(part) = parts.flags
        && parse_ascii_checked_culong(part)
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
        &mut proc_path,
        parts,
        uptime,
        info,
        #[cfg(feature = "gpu")]
        now,
    );

    Process { inner: p }
}

#[allow(clippy::too_many_arguments)]
fn update_existing_process(
    is_thread: bool,
    proc: &mut Process,
    parent_pid: Option<Pid>,
    uptime: u64,
    info: &SystemInfo,
    refresh_kind: ProcessRefreshKind,
    tasks: Option<HashSet<Pid>>,
    #[cfg(feature = "gpu")] now: Instant,
) -> Result<Option<Process>, ()> {
    let entry = &mut proc.inner;
    let data = if let Some(mut f) = entry.stat_file.take() {
        match get_all_data_from_file(&mut f, 1024) {
            Ok(data) => {
                // Everything went fine, we put back the file descriptor.
                entry.stat_file = Some(f);
                data
            }
            Err(_) => {
                // It's possible that the file descriptor is no longer valid in case the
                // original process was terminated and another one took its place.
                _get_stat_data(&entry.proc_path, &mut entry.stat_file)?
            }
        }
    } else {
        _get_stat_data(&entry.proc_path, &mut entry.stat_file)?
    };
    entry.tasks = tasks;

    let parts = parse_stat_file(&data).ok_or(())?;

    // It's possible that a new process took this same PID when the "original one" terminated.
    // If the start time differs, then it means it's not the same process anymore and that we
    // need to get all its information, hence why we check it here.
    if parts.start_time == entry.start_time_raw {
        let mut proc_path = PathHandler::new(&entry.proc_path);

        // If the entry was first discovered without thread info
        // (e.g. in ProcessesToUpdate::All mode), fix its thread_kind now.
        if is_thread && entry.thread_kind.is_none() {
            entry.thread_kind = Some(ThreadKind::Userland);
        }

        update_proc_info(
            entry,
            parent_pid,
            refresh_kind,
            &mut proc_path,
            &parts,
            uptime,
            info,
            #[cfg(feature = "gpu")]
            now,
        );

        refresh_user_group_ids(entry, &mut proc_path, refresh_kind);
        return Ok(None);
    }
    // If we're here, it means that the PID still exists but it's a different process.
    let p = retrieve_all_new_process_info(
        is_thread,
        entry.pid,
        parent_pid,
        &parts,
        &entry.proc_path,
        info,
        refresh_kind,
        uptime,
        #[cfg(feature = "gpu")]
        now,
    );
    *proc = p;
    // Since this PID is already in the HashMap, no need to add it again.
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn _get_process_data(
    path: &Path,
    proc_list: &mut HashMap<Pid, Process>,
    pid: Pid,
    is_thread: bool,
    parent_pid: Option<Pid>,
    uptime: u64,
    info: &SystemInfo,
    refresh_kind: ProcessRefreshKind,
    tasks: Option<HashSet<Pid>>,
    #[cfg(feature = "gpu")] now: Instant,
) -> Result<Option<Process>, ()> {
    if let Some(ref mut entry) = proc_list.get_mut(&pid) {
        return update_existing_process(
            is_thread,
            entry,
            parent_pid,
            uptime,
            info,
            refresh_kind,
            tasks,
            #[cfg(feature = "gpu")]
            now,
        );
    }
    let mut stat_file = None;
    let data = _get_stat_data(path, &mut stat_file)?;
    let parts = parse_stat_file(&data).ok_or(())?;

    let mut new_process = retrieve_all_new_process_info(
        is_thread,
        pid,
        parent_pid,
        &parts,
        path,
        info,
        refresh_kind,
        uptime,
        #[cfg(feature = "gpu")]
        now,
    );
    new_process.inner.stat_file = stat_file;
    new_process.inner.tasks = tasks;
    Ok(Some(new_process))
}

fn old_get_memory(entry: &mut ProcessInner, parts: &Parts, info: &SystemInfo) {
    // rss
    entry.memory = parts
        .resident_set_size
        .and_then(parse_ascii_checked_u64)
        .unwrap_or(0)
        .saturating_mul(info.page_size_b);
    // vsz correspond to the Virtual memory size in bytes.
    // see: https://man7.org/linux/man-pages/man5/proc.5.html
    entry.virtual_memory = parts
        .virtual_size
        .and_then(parse_ascii_checked_u64)
        .unwrap_or(0);
}

fn slice_to_nb(s: &[u8]) -> u64 {
    let mut nb: u64 = 0;

    for c in s {
        nb = nb * 10 + (c - b'0') as u64;
    }
    nb
}

fn get_memory(path: &Path, entry: &mut ProcessInner, info: &SystemInfo) -> bool {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_e) => {
            sysinfo_debug!(
                "Using old memory information (failed to open {:?}: {_e:?})",
                path
            );
            return false;
        }
    };
    let mut buf = Vec::new();
    if let Err(_e) = file.read_to_end(&mut buf) {
        sysinfo_debug!(
            "Using old memory information (failed to read {:?}: {_e:?})",
            path
        );
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
    path: &mut PathHandler,
    entry: &mut ProcessInner,
    parts: &Parts,
    uptime: u64,
    info: &SystemInfo,
    refresh_kind: ProcessRefreshKind,
) {
    {
        #[allow(clippy::collapsible_if)]
        if refresh_kind.memory() {
            // Keeping this nested level for readability reasons.
            if !get_memory(path.replace_and_join("statm"), entry, info) {
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
    path: PathBuf,
    tasks: Option<HashSet<Pid>>,
    is_thread: bool,
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
        #[cfg(feature = "gpu")]
        let now = Instant::now();

        iter(pid_iter)
            .flat_map(|(path, pid)| {
                get_proc_and_tasks(path, pid, refresh_kind, processes_to_update)
            })
            .filter_map(|e| {
                let proc_list = proc_list.get();
                let new_process = _get_process_data(
                    e.path.as_path(),
                    proc_list,
                    e.pid,
                    e.is_thread,
                    e.parent_pid,
                    uptime,
                    info,
                    refresh_kind,
                    e.tasks,
                    #[cfg(feature = "gpu")]
                    now,
                )
                .ok()?;
                nb_updated.fetch_add(1, Ordering::Relaxed);
                new_process
            })
            .collect::<Vec<_>>()
    };
    for proc_ in procs {
        proc_list.insert(proc_.pid(), proc_);
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
) -> Vec<ProcAndTasks> {
    let mut parent_pid = None;
    let mut is_thread = false;
    let (mut procs, mut tasks) = if refresh_kind.tasks() {
        let procs = get_proc_tasks(&path, pid);
        let tasks = procs.iter().map(|ProcAndTasks { pid, .. }| *pid).collect();

        (procs, Some(tasks))
    } else {
        (Vec::new(), None)
    };

    // If the process' tgid doesn't match its pid, it is a task (thread).
    // This check must apply in ALL modes, not just `Some`.
    if let Some(tgid) = get_tgid(&path.join("status"))
        && tgid != pid
    {
        parent_pid = Some(tgid);
        tasks = None;
        is_thread = true;
        // Threads don't have meaningful tasks, clear whatever was fetched.
        procs.clear();
    } else if processes_to_update != ProcessesToUpdate::All {
        // Don't add the tasks to the list of processes to update
        procs.clear();
    }

    procs.push(ProcAndTasks {
        is_thread,
        pid,
        parent_pid,
        path,
        tasks,
    });

    procs
}

fn get_proc_tasks(path: &Path, parent_pid: Pid) -> Vec<ProcAndTasks> {
    let task_path = path.join("task");

    read_dir(task_path)
        .ok()
        .map(|task_entries| {
            task_entries
                .filter_map(filter_pid_entries)
                // Needed because tasks have their own PID listed in the "task" folder.
                .filter(|(_, pid)| *pid != parent_pid)
                .map(|(path, pid)| ProcAndTasks {
                    pid,
                    is_thread: true,
                    path,
                    parent_pid: Some(parent_pid),
                    tasks: None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn split_content(mut data: &[u8]) -> Vec<OsString> {
    let mut out = Vec::with_capacity(10);
    while let Some(pos) = data.iter().position(|c| *c == 0) {
        let s = &data[..pos].trim_ascii();
        if !s.is_empty() {
            out.push(OsStr::from_bytes(s).to_os_string());
        }
        data = &data[pos + 1..];
    }
    if !data.is_empty() {
        let s = data.trim_ascii();
        if !s.is_empty() {
            out.push(OsStr::from_bytes(s).to_os_string());
        }
    }
    out
}

fn copy_from_file(entry: &Path) -> Vec<OsString> {
    match File::open(entry) {
        Ok(mut f) => {
            let mut data = Vec::with_capacity(16_384);

            if let Err(_e) = f.read_to_end(&mut data) {
                sysinfo_debug!("Failed to read file in `copy_from_file`: {:?}", _e);
                Vec::new()
            } else {
                split_content(&data)
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
    let status_data = get_all_utf8_data(file_path, 16_385).ok()?;

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

fn get_tgid(file_path: &Path) -> Option<Pid> {
    const TGID_KEY: &str = "Tgid:";
    let status_data = get_all_utf8_data(file_path, 16_385).ok()?;
    let tgid_line = status_data
        .lines()
        .find(|line| line.starts_with(TGID_KEY))?;
    tgid_line[TGID_KEY.len()..].trim_start().parse().ok()
}

/// Type used to correctly handle the `REMAINING_FILES` global.
struct FileCounter(File);

impl FileCounter {
    fn new(f: File) -> Option<Self> {
        let any_remaining =
            remaining_files().try_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
                if remaining > 0 {
                    Some(remaining - 1)
                } else {
                    // All file descriptors we were allowed are being used.
                    None
                }
            });

        any_remaining.ok().map(|_| Self(f))
    }
}

impl std::ops::Deref for FileCounter {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for FileCounter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for FileCounter {
    fn drop(&mut self) {
        remaining_files().fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg_attr(test, derive(PartialEq, Debug))]
struct Parts<'a> {
    short_exe: &'a [u8],
    status: ProcessStatus,
    parent_pid: Option<&'a [u8]>,
    flags: Option<&'a [u8]>,
    user_time: u64,
    system_time: u64,
    start_time: u64,
    virtual_size: Option<&'a [u8]>,
    resident_set_size: Option<&'a [u8]>,
}

fn parse_stat_file(data: &[u8]) -> Option<Parts<'_>> {
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
    let parent_pid = data.next();

    let flags = data.nth(ProcIndex::Flags as usize - ProcIndex::ParentPid as usize - 1);
    let user_time = data
        .nth(ProcIndex::UserTime as usize - ProcIndex::Flags as usize - 1)
        .and_then(parse_ascii_checked_u64)
        .unwrap_or(0);
    let system_time = data.next().and_then(parse_ascii_checked_u64).unwrap_or(0);

    let start_time = data
        .nth(ProcIndex::StartTime as usize - ProcIndex::SystemTime as usize - 1)
        .and_then(parse_ascii_checked_u64)
        .unwrap_or(0);
    let virtual_size = data.next();
    let resident_set_size = data.next();

    Some(Parts {
        status,
        parent_pid,
        flags,
        user_time,
        system_time,
        start_time,
        virtual_size,
        resident_set_size,
        short_exe,
    })
}

#[cfg(test)]
mod tests {
    use super::{Parts, parse_stat_file, split_content};
    use std::ffi::OsString;

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
        let data = parse_stat_file(content).unwrap();
        assert_eq!(
            data,
            Parts {
                short_exe: b"blob",
                status: crate::ProcessStatus::Sleep,
                parent_pid: Some(b"2"),
                flags: Some(b"2129984"),
                user_time: 14,
                start_time: 21,
                system_time: 28,
                virtual_size: Some(b"66"),
                resident_set_size: Some(b"77"),
            }
        );
    }

    // This test ensures that even if we have a `(`/`)` char in the short exe name, we still make
    // it works.
    #[test]
    fn test_parse_stat_file_short_exe() {
        // The (trimmed) content of a stat file.
        let content = b"1 (bl()ob) S 2 0 0 0 -1 2129984 0 0 0 0 14 28 0 0 20 0 1 0 21 66 77";
        let data = parse_stat_file(content).unwrap();
        assert_eq!(
            data,
            Parts {
                short_exe: b"bl()ob",
                status: crate::ProcessStatus::Sleep,
                parent_pid: Some(b"2"),
                flags: Some(b"2129984"),
                user_time: 14,
                start_time: 21,
                system_time: 28,
                virtual_size: Some(b"66"),
                resident_set_size: Some(b"77"),
            }
        );
    }

    #[test]
    fn test_parse_stat_file_long_exe() {
        // In case you wonder: yes, it's a real "short" exe name.
        let content = b"1 (nvidia-modeset/deferred_close_kthread_q) S 2 0 0 0 -1 2129984 0 0 0 0 14 28 0 0 20 0 1 0 21 66 77";
        let data = parse_stat_file(content).unwrap();
        assert_eq!(
            data,
            Parts {
                short_exe: b"nvidia-modeset/deferred_close_kthread_q",
                status: crate::ProcessStatus::Sleep,
                parent_pid: Some(b"2"),
                flags: Some(b"2129984"),
                user_time: 14,
                start_time: 21,
                system_time: 28,
                virtual_size: Some(b"66"),
                resident_set_size: Some(b"77"),
            }
        );
    }
}
