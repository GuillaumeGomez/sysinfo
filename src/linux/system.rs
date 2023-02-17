// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::component::{self, Component};
use crate::sys::cpu::*;
use crate::sys::disk;
use crate::sys::process::*;
use crate::sys::utils::{get_all_data, to_u64};
use crate::{
    CpuRefreshKind, Disk, LoadAvg, Networks, Pid, ProcessRefreshKind, RefreshKind, SystemExt, User,
};

use libc::{self, c_char, c_int, sysconf, _SC_CLK_TCK, _SC_HOST_NAME_MAX, _SC_PAGESIZE};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// This whole thing is to prevent having too many files open at once. It could be problematic
// for processes using a lot of files and using sysinfo at the same time.
#[allow(clippy::mutex_atomic)]
pub(crate) static mut REMAINING_FILES: once_cell::sync::Lazy<Arc<Mutex<isize>>> =
    once_cell::sync::Lazy::new(|| {
        unsafe {
            let mut limits = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            if libc::getrlimit(libc::RLIMIT_NOFILE, &mut limits) != 0 {
                // Most Linux system now defaults to 1024.
                return Arc::new(Mutex::new(1024 / 2));
            }
            // We save the value in case the update fails.
            let current = limits.rlim_cur;

            // The set the soft limit to the hard one.
            limits.rlim_cur = limits.rlim_max;
            // In this part, we leave minimum 50% of the available file descriptors to the process
            // using sysinfo.
            Arc::new(Mutex::new(
                if libc::setrlimit(libc::RLIMIT_NOFILE, &limits) == 0 {
                    limits.rlim_cur / 2
                } else {
                    current / 2
                } as _,
            ))
        }
    });

pub(crate) fn get_max_nb_fds() -> isize {
    unsafe {
        let mut limits = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut limits) != 0 {
            // Most Linux system now defaults to 1024.
            1024 / 2
        } else {
            limits.rlim_max as isize / 2
        }
    }
}

fn boot_time() -> u64 {
    if let Ok(f) = File::open("/proc/stat") {
        let buf = BufReader::new(f);
        let line = buf
            .split(b'\n')
            .filter_map(|r| r.ok())
            .find(|l| l.starts_with(b"btime"));

        if let Some(line) = line {
            return line
                .split(|x| *x == b' ')
                .filter(|s| !s.is_empty())
                .nth(1)
                .map(to_u64)
                .unwrap_or(0);
        }
    }
    // Either we didn't find "btime" or "/proc/stat" wasn't available for some reason...
    unsafe {
        let mut up: libc::timespec = std::mem::zeroed();
        if libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut up) == 0 {
            up.tv_sec as u64
        } else {
            sysinfo_debug!("clock_gettime failed: boot time cannot be retrieve...");
            0
        }
    }
}

pub(crate) struct SystemInfo {
    pub(crate) page_size_kb: u64,
    pub(crate) clock_cycle: u64,
    pub(crate) boot_time: u64,
}

impl SystemInfo {
    fn new() -> Self {
        unsafe {
            Self {
                page_size_kb: sysconf(_SC_PAGESIZE) as _,
                clock_cycle: sysconf(_SC_CLK_TCK) as _,
                boot_time: boot_time(),
            }
        }
    }
}

declare_signals! {
    c_int,
    Signal::Hangup => libc::SIGHUP,
    Signal::Interrupt => libc::SIGINT,
    Signal::Quit => libc::SIGQUIT,
    Signal::Illegal => libc::SIGILL,
    Signal::Trap => libc::SIGTRAP,
    Signal::Abort => libc::SIGABRT,
    Signal::IOT => libc::SIGIOT,
    Signal::Bus => libc::SIGBUS,
    Signal::FloatingPointException => libc::SIGFPE,
    Signal::Kill => libc::SIGKILL,
    Signal::User1 => libc::SIGUSR1,
    Signal::Segv => libc::SIGSEGV,
    Signal::User2 => libc::SIGUSR2,
    Signal::Pipe => libc::SIGPIPE,
    Signal::Alarm => libc::SIGALRM,
    Signal::Term => libc::SIGTERM,
    Signal::Child => libc::SIGCHLD,
    Signal::Continue => libc::SIGCONT,
    Signal::Stop => libc::SIGSTOP,
    Signal::TSTP => libc::SIGTSTP,
    Signal::TTIN => libc::SIGTTIN,
    Signal::TTOU => libc::SIGTTOU,
    Signal::Urgent => libc::SIGURG,
    Signal::XCPU => libc::SIGXCPU,
    Signal::XFSZ => libc::SIGXFSZ,
    Signal::VirtualAlarm => libc::SIGVTALRM,
    Signal::Profiling => libc::SIGPROF,
    Signal::Winch => libc::SIGWINCH,
    Signal::IO => libc::SIGIO,
    Signal::Poll => libc::SIGPOLL,
    Signal::Power => libc::SIGPWR,
    Signal::Sys => libc::SIGSYS,
}

#[doc = include_str!("../../md_doc/system.md")]
pub struct System {
    process_list: Process,
    mem_total: u64,
    mem_free: u64,
    mem_available: u64,
    mem_buffers: u64,
    mem_page_cache: u64,
    mem_shmem: u64,
    mem_slab_reclaimable: u64,
    swap_total: u64,
    swap_free: u64,
    components: Vec<Component>,
    disks: Vec<Disk>,
    networks: Networks,
    users: Vec<User>,
    info: SystemInfo,
    cpus: CpusWrapper,
}

impl System {
    /// It is sometime possible that a CPU usage computation is bigger than
    /// `"number of CPUs" * 100`.
    ///
    /// To prevent that, we compute ahead of time this maximum value and ensure that processes'
    /// CPU usage don't go over it.
    fn get_max_process_cpu_usage(&self) -> f32 {
        self.cpus.len() as f32 * 100.
    }

    fn clear_procs(&mut self, refresh_kind: ProcessRefreshKind) {
        let (total_time, compute_cpu, max_value) = if refresh_kind.cpu() {
            self.cpus
                .refresh_if_needed(true, CpuRefreshKind::new().with_cpu_usage());

            if self.cpus.is_empty() {
                sysinfo_debug!("cannot compute processes CPU usage: no CPU found...");
                (0., false, 0.)
            } else {
                let (new, old) = self.cpus.get_global_raw_times();
                let total_time = if old > new { 1 } else { new - old };
                (
                    total_time as f32 / self.cpus.len() as f32,
                    true,
                    self.get_max_process_cpu_usage(),
                )
            }
        } else {
            (0., false, 0.)
        };

        self.process_list.tasks.retain(|_, proc_| {
            if !proc_.updated {
                return false;
            }
            if compute_cpu {
                compute_cpu_usage(proc_, total_time, max_value);
            }
            unset_updated(proc_);
            true
        });
    }

    fn refresh_cpus(&mut self, only_update_global_cpu: bool, refresh_kind: CpuRefreshKind) {
        self.cpus.refresh(only_update_global_cpu, refresh_kind);
    }
}

impl SystemExt for System {
    const IS_SUPPORTED: bool = true;
    const SUPPORTED_SIGNALS: &'static [Signal] = supported_signals();
    const MINIMUM_CPU_UPDATE_INTERVAL: Duration = Duration::from_millis(200);

    fn new_with_specifics(refreshes: RefreshKind) -> System {
        let process_list = Process::new(Pid(0));
        let mut s = System {
            process_list,
            mem_total: 0,
            mem_free: 0,
            mem_available: 0,
            mem_buffers: 0,
            mem_page_cache: 0,
            mem_shmem: 0,
            mem_slab_reclaimable: 0,
            swap_total: 0,
            swap_free: 0,
            cpus: CpusWrapper::new(),
            components: Vec::new(),
            disks: Vec::with_capacity(2),
            networks: Networks::new(),
            users: Vec::new(),
            info: SystemInfo::new(),
        };
        s.refresh_specifics(refreshes);
        s
    }

    fn refresh_components_list(&mut self) {
        self.components = component::get_components();
    }

    fn refresh_memory(&mut self) {
        if let Ok(data) = get_all_data("/proc/meminfo", 16_385) {
            let mut mem_available_found = false;

            for line in data.split('\n') {
                let mut iter = line.split(':');
                let field = match iter.next() {
                    Some("MemTotal") => &mut self.mem_total,
                    Some("MemFree") => &mut self.mem_free,
                    Some("MemAvailable") => {
                        mem_available_found = true;
                        &mut self.mem_available
                    }
                    Some("Buffers") => &mut self.mem_buffers,
                    Some("Cached") => &mut self.mem_page_cache,
                    Some("Shmem") => &mut self.mem_shmem,
                    Some("SReclaimable") => &mut self.mem_slab_reclaimable,
                    Some("SwapTotal") => &mut self.swap_total,
                    Some("SwapFree") => &mut self.swap_free,
                    _ => continue,
                };
                if let Some(val_str) = iter.next().and_then(|s| s.trim_start().split(' ').next()) {
                    if let Ok(value) = u64::from_str(val_str) {
                        // /proc/meminfo reports KiB, though it says "kB". Convert it.
                        *field = value.saturating_mul(1_024);
                    }
                }
            }

            // Linux < 3.14 may not have MemAvailable in /proc/meminfo
            // So it should fallback to the old way of estimating available memory
            // https://github.com/KittyKatt/screenFetch/issues/386#issuecomment-249312716
            if !mem_available_found {
                self.mem_available = self.mem_free
                    + self.mem_buffers
                    + self.mem_page_cache
                    + self.mem_slab_reclaimable
                    - self.mem_shmem;
            }
        }
    }

    fn refresh_cpu_specifics(&mut self, refresh_kind: CpuRefreshKind) {
        self.refresh_cpus(false, refresh_kind);
    }

    fn refresh_processes_specifics(&mut self, refresh_kind: ProcessRefreshKind) {
        let uptime = self.uptime();
        refresh_procs(
            &mut self.process_list,
            Path::new("/proc"),
            Pid(0),
            uptime,
            &self.info,
            refresh_kind,
        );
        self.clear_procs(refresh_kind);
        self.cpus.set_need_cpus_update();
    }

    fn refresh_process_specifics(&mut self, pid: Pid, refresh_kind: ProcessRefreshKind) -> bool {
        let uptime = self.uptime();
        match _get_process_data(
            &Path::new("/proc/").join(pid.to_string()),
            &mut self.process_list,
            Pid(0),
            uptime,
            &self.info,
            refresh_kind,
        ) {
            Ok((Some(p), pid)) => {
                self.process_list.tasks.insert(pid, p);
            }
            Ok(_) => {}
            Err(_e) => {
                sysinfo_debug!("Cannot get information for PID {:?}: {:?}", pid, _e);
                return false;
            }
        };
        if refresh_kind.cpu() {
            self.refresh_cpus(true, CpuRefreshKind::new().with_cpu_usage());

            if self.cpus.is_empty() {
                eprintln!("Cannot compute process CPU usage: no cpus found...");
                return true;
            }
            let (new, old) = self.cpus.get_global_raw_times();
            let total_time = (if old >= new { 1 } else { new - old }) as f32;
            let total_time = total_time / self.cpus.len() as f32;

            let max_cpu_usage = self.get_max_process_cpu_usage();
            if let Some(p) = self.process_list.tasks.get_mut(&pid) {
                compute_cpu_usage(p, total_time, max_cpu_usage);
                unset_updated(p);
            }
        } else if let Some(p) = self.process_list.tasks.get_mut(&pid) {
            unset_updated(p);
        }
        true
    }

    fn refresh_disks_list(&mut self) {
        self.disks = disk::get_all_disks();
    }

    fn refresh_users_list(&mut self) {
        self.users = crate::users::get_users_list();
    }

    // COMMON PART
    //
    // Need to be moved into a "common" file to avoid duplication.

    fn processes(&self) -> &HashMap<Pid, Process> {
        &self.process_list.tasks
    }

    fn process(&self, pid: Pid) -> Option<&Process> {
        self.process_list.tasks.get(&pid)
    }

    fn networks(&self) -> &Networks {
        &self.networks
    }

    fn networks_mut(&mut self) -> &mut Networks {
        &mut self.networks
    }

    fn global_cpu_info(&self) -> &Cpu {
        &self.cpus.global_cpu
    }

    fn cpus(&self) -> &[Cpu] {
        &self.cpus.cpus
    }

    fn physical_core_count(&self) -> Option<usize> {
        get_physical_core_count()
    }

    fn total_memory(&self) -> u64 {
        self.mem_total
    }

    fn free_memory(&self) -> u64 {
        self.mem_free
    }

    fn available_memory(&self) -> u64 {
        self.mem_available
    }

    fn used_memory(&self) -> u64 {
        self.mem_total - self.mem_available
    }

    fn total_swap(&self) -> u64 {
        self.swap_total
    }

    fn free_swap(&self) -> u64 {
        self.swap_free
    }

    // need to be checked
    fn used_swap(&self) -> u64 {
        self.swap_total - self.swap_free
    }

    fn components(&self) -> &[Component] {
        &self.components
    }

    fn components_mut(&mut self) -> &mut [Component] {
        &mut self.components
    }

    fn disks(&self) -> &[Disk] {
        &self.disks
    }

    fn disks_mut(&mut self) -> &mut [Disk] {
        &mut self.disks
    }

    fn sort_disks_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Disk, &Disk) -> std::cmp::Ordering,
    {
        self.disks.sort_unstable_by(compare);
    }

    fn uptime(&self) -> u64 {
        let content = get_all_data("/proc/uptime", 50).unwrap_or_default();
        content
            .split('.')
            .next()
            .and_then(|t| t.parse().ok())
            .unwrap_or_default()
    }

    fn boot_time(&self) -> u64 {
        self.info.boot_time
    }

    fn load_average(&self) -> LoadAvg {
        let mut s = String::new();
        if File::open("/proc/loadavg")
            .and_then(|mut f| f.read_to_string(&mut s))
            .is_err()
        {
            return LoadAvg::default();
        }
        let loads = s
            .trim()
            .split(' ')
            .take(3)
            .map(|val| val.parse::<f64>().unwrap())
            .collect::<Vec<f64>>();
        LoadAvg {
            one: loads[0],
            five: loads[1],
            fifteen: loads[2],
        }
    }

    fn users(&self) -> &[User] {
        &self.users
    }

    #[cfg(not(target_os = "android"))]
    fn name(&self) -> Option<String> {
        get_system_info_linux(
            InfoType::Name,
            Path::new("/etc/os-release"),
            Path::new("/etc/lsb-release"),
        )
    }

    #[cfg(target_os = "android")]
    fn name(&self) -> Option<String> {
        get_system_info_android(InfoType::Name)
    }

    fn long_os_version(&self) -> Option<String> {
        #[cfg(target_os = "android")]
        let system_name = "Android";

        #[cfg(not(target_os = "android"))]
        let system_name = "Linux";

        Some(format!(
            "{} {} {}",
            system_name,
            self.os_version().unwrap_or_default(),
            self.name().unwrap_or_default()
        ))
    }

    fn host_name(&self) -> Option<String> {
        unsafe {
            let hostname_max = sysconf(_SC_HOST_NAME_MAX);
            let mut buffer = vec![0_u8; hostname_max as usize];
            if libc::gethostname(buffer.as_mut_ptr() as *mut c_char, buffer.len()) == 0 {
                if let Some(pos) = buffer.iter().position(|x| *x == 0) {
                    // Shrink buffer to terminate the null bytes
                    buffer.resize(pos, 0);
                }
                String::from_utf8(buffer).ok()
            } else {
                sysinfo_debug!("gethostname failed: hostname cannot be retrieved...");
                None
            }
        }
    }

    fn kernel_version(&self) -> Option<String> {
        let mut raw = std::mem::MaybeUninit::<libc::utsname>::zeroed();

        unsafe {
            if libc::uname(raw.as_mut_ptr()) == 0 {
                let info = raw.assume_init();

                let release = info
                    .release
                    .iter()
                    .filter(|c| **c != 0)
                    .map(|c| *c as u8 as char)
                    .collect::<String>();

                Some(release)
            } else {
                None
            }
        }
    }

    #[cfg(not(target_os = "android"))]
    fn os_version(&self) -> Option<String> {
        get_system_info_linux(
            InfoType::OsVersion,
            Path::new("/etc/os-release"),
            Path::new("/etc/lsb-release"),
        )
    }

    #[cfg(target_os = "android")]
    fn os_version(&self) -> Option<String> {
        get_system_info_android(InfoType::OsVersion)
    }

    #[cfg(not(target_os = "android"))]
    fn distribution_id(&self) -> String {
        get_system_info_linux(
            InfoType::DistributionID,
            Path::new("/etc/os-release"),
            Path::new(""),
        )
        .unwrap_or_else(|| std::env::consts::OS.to_owned())
    }

    #[cfg(target_os = "android")]
    fn distribution_id(&self) -> String {
        // Currently get_system_info_android doesn't support InfoType::DistributionID and always
        // returns None. This call is done anyway for consistency with non-Android implementation
        // and to suppress dead-code warning for DistributionID on Android.
        get_system_info_android(InfoType::DistributionID)
            .unwrap_or_else(|| std::env::consts::OS.to_owned())
    }
}

impl Default for System {
    fn default() -> System {
        System::new()
    }
}

#[derive(PartialEq, Eq)]
enum InfoType {
    /// The end-user friendly name of:
    /// - Android: The device model
    /// - Linux: The distributions name
    Name,
    OsVersion,
    /// Machine-parseable ID of a distribution, see
    /// https://www.freedesktop.org/software/systemd/man/os-release.html#ID=
    DistributionID,
}

#[cfg(not(target_os = "android"))]
fn get_system_info_linux(info: InfoType, path: &Path, fallback_path: &Path) -> Option<String> {
    if let Ok(f) = File::open(path) {
        let reader = BufReader::new(f);

        let info_str = match info {
            InfoType::Name => "NAME=",
            InfoType::OsVersion => "VERSION_ID=",
            InfoType::DistributionID => "ID=",
        };

        for line in reader.lines().flatten() {
            if let Some(stripped) = line.strip_prefix(info_str) {
                return Some(stripped.replace('"', ""));
            }
        }
    }

    // Fallback to `/etc/lsb-release` file for systems where VERSION_ID is not included.
    // VERSION_ID is not required in the `/etc/os-release` file
    // per https://www.linux.org/docs/man5/os-release.html
    // If this fails for some reason, fallback to None
    let reader = BufReader::new(File::open(fallback_path).ok()?);

    let info_str = match info {
        InfoType::OsVersion => "DISTRIB_RELEASE=",
        InfoType::Name => "DISTRIB_ID=",
        InfoType::DistributionID => {
            // lsb-release is inconsistent with os-release and unsupported.
            return None;
        }
    };
    for line in reader.lines().flatten() {
        if let Some(stripped) = line.strip_prefix(info_str) {
            return Some(stripped.replace('"', ""));
        }
    }
    None
}

#[cfg(target_os = "android")]
fn get_system_info_android(info: InfoType) -> Option<String> {
    // https://android.googlesource.com/platform/frameworks/base/+/refs/heads/master/core/java/android/os/Build.java#58
    let name: &'static [u8] = match info {
        InfoType::Name => b"ro.product.model\0",
        InfoType::OsVersion => b"ro.build.version.release\0",
        InfoType::DistributionID => {
            // Not supported.
            return None;
        }
    };

    let mut value_buffer = vec![0u8; libc::PROP_VALUE_MAX as usize];
    unsafe {
        let len = libc::__system_property_get(
            name.as_ptr() as *const c_char,
            value_buffer.as_mut_ptr() as *mut c_char,
        );

        if len != 0 {
            if let Some(pos) = value_buffer.iter().position(|c| *c == 0) {
                value_buffer.resize(pos, 0);
            }
            String::from_utf8(value_buffer).ok()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    #[cfg(target_os = "android")]
    use super::get_system_info_android;
    #[cfg(not(target_os = "android"))]
    use super::get_system_info_linux;
    use super::InfoType;

    #[test]
    #[cfg(target_os = "android")]
    fn lsb_release_fallback_android() {
        assert!(get_system_info_android(InfoType::OsVersion).is_some());
        assert!(get_system_info_android(InfoType::Name).is_some());
        assert!(get_system_info_android(InfoType::DistributionID).is_none());
    }

    #[test]
    #[cfg(not(target_os = "android"))]
    fn lsb_release_fallback_not_android() {
        use std::path::Path;

        let dir = tempfile::tempdir().expect("failed to create temporary directory");
        let tmp1 = dir.path().join("tmp1");
        let tmp2 = dir.path().join("tmp2");

        // /etc/os-release
        std::fs::write(
            &tmp1,
            r#"NAME="Ubuntu"
VERSION="20.10 (Groovy Gorilla)"
ID=ubuntu
ID_LIKE=debian
PRETTY_NAME="Ubuntu 20.10"
VERSION_ID="20.10"
VERSION_CODENAME=groovy
UBUNTU_CODENAME=groovy
"#,
        )
        .expect("Failed to create tmp1");

        // /etc/lsb-release
        std::fs::write(
            &tmp2,
            r#"DISTRIB_ID=Ubuntu
DISTRIB_RELEASE=20.10
DISTRIB_CODENAME=groovy
DISTRIB_DESCRIPTION="Ubuntu 20.10"
"#,
        )
        .expect("Failed to create tmp2");

        // Check for the "normal" path: "/etc/os-release"
        assert_eq!(
            get_system_info_linux(InfoType::OsVersion, &tmp1, Path::new("")),
            Some("20.10".to_owned())
        );
        assert_eq!(
            get_system_info_linux(InfoType::Name, &tmp1, Path::new("")),
            Some("Ubuntu".to_owned())
        );
        assert_eq!(
            get_system_info_linux(InfoType::DistributionID, &tmp1, Path::new("")),
            Some("ubuntu".to_owned())
        );

        // Check for the "fallback" path: "/etc/lsb-release"
        assert_eq!(
            get_system_info_linux(InfoType::OsVersion, Path::new(""), &tmp2),
            Some("20.10".to_owned())
        );
        assert_eq!(
            get_system_info_linux(InfoType::Name, Path::new(""), &tmp2),
            Some("Ubuntu".to_owned())
        );
        assert_eq!(
            get_system_info_linux(InfoType::DistributionID, Path::new(""), &tmp2),
            None
        );
    }
}
