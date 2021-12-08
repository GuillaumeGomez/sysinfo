//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use crate::sys::component::{self, Component};
use crate::sys::disk;
use crate::sys::process::*;
use crate::sys::processor::*;
use crate::sys::utils::get_all_data;
use crate::{Disk, LoadAvg, Networks, Pid, ProcessExt, RefreshKind, SystemExt, User};

use libc::{self, c_char, sysconf, _SC_HOST_NAME_MAX, _SC_PAGESIZE};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

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
                // Most linux system now defaults to 1024.
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
            // Most linux system now defaults to 1024.
            1024 / 2
        } else {
            limits.rlim_max as isize / 2
        }
    }
}

macro_rules! to_str {
    ($e:expr) => {
        unsafe { std::str::from_utf8_unchecked($e) }
    };
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
    let mut up = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut up) } == 0 {
        up.tv_sec as u64
    } else {
        sysinfo_debug!("clock_gettime failed: boot time cannot be retrieve...");
        0
    }
}

#[doc = include_str!("../../md_doc/system.md")]
pub struct System {
    process_list: Process,
    mem_total: u64,
    mem_free: u64,
    mem_available: u64,
    mem_buffers: u64,
    mem_page_cache: u64,
    mem_slab_reclaimable: u64,
    swap_total: u64,
    swap_free: u64,
    global_processor: Processor,
    processors: Vec<Processor>,
    page_size_kb: u64,
    components: Vec<Component>,
    disks: Vec<Disk>,
    networks: Networks,
    users: Vec<User>,
    boot_time: u64,
}

impl System {
    /// It is sometime possible that a CPU usage computation is bigger than
    /// `"number of CPUs" * 100`.
    ///
    /// To prevent that, we compute ahead of time this maximum value and ensure that processes'
    /// CPU usage don't go over it.
    fn get_max_process_cpu_usage(&self) -> f32 {
        self.processors.len() as f32 * 100.
    }

    fn clear_procs(&mut self) {
        self.refresh_processors(true);

        let (total_time, compute_cpu, max_value) = if self.processors.is_empty() {
            sysinfo_debug!("cannot compute processes CPU usage: no processor found...");
            (0., false, 0.)
        } else {
            let (new, old) = get_raw_times(&self.global_processor);
            let total_time = if old > new { 1 } else { new - old };
            (
                total_time as f32 / self.processors.len() as f32,
                true,
                self.get_max_process_cpu_usage(),
            )
        };

        let mut to_delete = Vec::with_capacity(20);

        for (pid, proc_) in &mut self.process_list.tasks {
            if !has_been_updated(proc_) {
                to_delete.push(*pid);
            } else if compute_cpu {
                compute_cpu_usage(proc_, total_time, max_value);
            }
        }
        for pid in to_delete {
            self.process_list.tasks.remove(&pid);
        }
    }

    fn refresh_processors(&mut self, only_update_global_processor: bool) {
        if let Ok(f) = File::open("/proc/stat") {
            let buf = BufReader::new(f);
            let mut i: usize = 0;
            let first = self.processors.is_empty();
            let mut it = buf.split(b'\n');
            let (vendor_id, brand) = if first {
                get_vendor_id_and_brand()
            } else {
                (String::new(), String::new())
            };

            if let Some(Ok(line)) = it.next() {
                if &line[..4] != b"cpu " {
                    return;
                }
                let mut parts = line.split(|x| *x == b' ').filter(|s| !s.is_empty());
                if first {
                    self.global_processor.name = to_str!(parts.next().unwrap_or(&[])).to_owned();
                } else {
                    parts.next();
                }
                self.global_processor.set(
                    parts.next().map(to_u64).unwrap_or(0),
                    parts.next().map(to_u64).unwrap_or(0),
                    parts.next().map(to_u64).unwrap_or(0),
                    parts.next().map(to_u64).unwrap_or(0),
                    parts.next().map(to_u64).unwrap_or(0),
                    parts.next().map(to_u64).unwrap_or(0),
                    parts.next().map(to_u64).unwrap_or(0),
                    parts.next().map(to_u64).unwrap_or(0),
                    parts.next().map(to_u64).unwrap_or(0),
                    parts.next().map(to_u64).unwrap_or(0),
                );
                if !first && only_update_global_processor {
                    return;
                }
            }
            while let Some(Ok(line)) = it.next() {
                if &line[..3] != b"cpu" {
                    break;
                }

                let mut parts = line.split(|x| *x == b' ').filter(|s| !s.is_empty());
                if first {
                    self.processors.push(Processor::new_with_values(
                        to_str!(parts.next().unwrap_or(&[])),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        get_cpu_frequency(i),
                        vendor_id.clone(),
                        brand.clone(),
                    ));
                } else {
                    parts.next(); // we don't want the name again
                    self.processors[i].set(
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                        parts.next().map(to_u64).unwrap_or(0),
                    );
                    self.processors[i].frequency = get_cpu_frequency(i);
                }

                i += 1;
            }

            self.global_processor.frequency = self
                .processors
                .iter()
                .map(|p| p.frequency)
                .max()
                .unwrap_or(0);

            if first {
                self.global_processor.vendor_id = vendor_id;
                self.global_processor.brand = brand;
            }
        }
    }
}

impl SystemExt for System {
    const IS_SUPPORTED: bool = true;

    fn new_with_specifics(refreshes: RefreshKind) -> System {
        let mut s = System {
            process_list: Process::new(0, None, 0),
            mem_total: 0,
            mem_free: 0,
            mem_available: 0,
            mem_buffers: 0,
            mem_page_cache: 0,
            mem_slab_reclaimable: 0,
            swap_total: 0,
            swap_free: 0,
            global_processor: Processor::new_with_values(
                "",
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                String::new(),
                String::new(),
            ),
            processors: Vec::with_capacity(4),
            page_size_kb: unsafe { sysconf(_SC_PAGESIZE) as u64 / 1024 },
            components: Vec::new(),
            disks: Vec::with_capacity(2),
            networks: Networks::new(),
            users: Vec::new(),
            boot_time: boot_time(),
        };
        if !refreshes.cpu() {
            s.refresh_processors(false); // We need the processors to be filled.
        }
        s.refresh_specifics(refreshes);
        s
    }

    fn refresh_components_list(&mut self) {
        self.components = component::get_components();
    }

    fn refresh_memory(&mut self) {
        if let Ok(data) = get_all_data("/proc/meminfo", 16_385) {
            for line in data.split('\n') {
                let mut iter = line.split(':');
                let field = match iter.next() {
                    Some("MemTotal") => &mut self.mem_total,
                    Some("MemFree") => &mut self.mem_free,
                    Some("MemAvailable") => &mut self.mem_available,
                    Some("Buffers") => &mut self.mem_buffers,
                    Some("Cached") => &mut self.mem_page_cache,
                    Some("SReclaimable") => &mut self.mem_slab_reclaimable,
                    Some("SwapTotal") => &mut self.swap_total,
                    Some("SwapFree") => &mut self.swap_free,
                    _ => continue,
                };
                if let Some(val_str) = iter.next().and_then(|s| s.trim_start().split(' ').next()) {
                    if let Ok(value) = u64::from_str(val_str) {
                        // /proc/meminfo reports KiB, though it says "kB". Convert it.
                        *field = value * 128 / 125;
                    }
                }
            }
        }
    }

    fn refresh_cpu(&mut self) {
        self.refresh_processors(false);
    }

    fn refresh_processes(&mut self) {
        let uptime = self.uptime();
        if refresh_procs(
            &mut self.process_list,
            Path::new("/proc"),
            self.page_size_kb,
            0,
            uptime,
            get_secs_since_epoch(),
        ) {
            self.clear_procs();
        }
    }

    fn refresh_process(&mut self, pid: Pid) -> bool {
        let uptime = self.uptime();
        let found = match _get_process_data(
            &Path::new("/proc/").join(pid.to_string()),
            &mut self.process_list,
            self.page_size_kb,
            0,
            uptime,
            get_secs_since_epoch(),
        ) {
            Ok((Some(p), pid)) => {
                self.process_list.tasks.insert(pid, p);
                true
            }
            Ok(_) => true,
            Err(_) => false,
        };
        if found {
            self.refresh_processors(true);

            if self.processors.is_empty() {
                sysinfo_debug!("Cannot compute process CPU usage: no processors found...");
                return found;
            }
            let (new, old) = get_raw_times(&self.global_processor);
            let total_time = (if old >= new { 1 } else { new - old }) as f32;

            let max_cpu_usage = self.get_max_process_cpu_usage();
            if let Some(p) = self.process_list.tasks.get_mut(&pid) {
                compute_cpu_usage(p, total_time / self.processors.len() as f32, max_cpu_usage);
            }
        }
        found
    }

    fn refresh_disks_list(&mut self) {
        self.disks = disk::get_all_disks();
    }

    fn refresh_users_list(&mut self) {
        self.users = crate::linux::users::get_users_list();
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

    fn global_processor_info(&self) -> &Processor {
        &self.global_processor
    }

    fn processors(&self) -> &[Processor] {
        &self.processors
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
        self.mem_total
            - self.mem_free
            - self.mem_buffers
            - self.mem_page_cache
            - self.mem_slab_reclaimable
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

    fn uptime(&self) -> u64 {
        let content = get_all_data("/proc/uptime", 50).unwrap_or_default();
        content
            .split('.')
            .next()
            .and_then(|t| t.parse().ok())
            .unwrap_or_default()
    }

    fn boot_time(&self) -> u64 {
        self.boot_time
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
        let hostname_max = unsafe { sysconf(_SC_HOST_NAME_MAX) };
        let mut buffer = vec![0_u8; hostname_max as usize];
        if unsafe { libc::gethostname(buffer.as_mut_ptr() as *mut c_char, buffer.len()) } == 0 {
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

    fn kernel_version(&self) -> Option<String> {
        let mut raw = std::mem::MaybeUninit::<libc::utsname>::zeroed();

        if unsafe { libc::uname(raw.as_mut_ptr()) } == 0 {
            let info = unsafe { raw.assume_init() };

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
}

impl Default for System {
    fn default() -> System {
        System::new()
    }
}

fn to_u64(v: &[u8]) -> u64 {
    let mut x = 0;

    for c in v {
        x *= 10;
        x += u64::from(c - b'0');
    }
    x
}

#[derive(PartialEq)]
enum InfoType {
    /// The end-user friendly name of:
    /// - Android: The device model
    /// - Linux: The distributions name
    Name,
    OsVersion,
}

#[cfg(not(target_os = "android"))]
fn get_system_info_linux(info: InfoType, path: &Path, fallback_path: &Path) -> Option<String> {
    if let Ok(f) = File::open(path) {
        let reader = BufReader::new(f);

        let info_str = match info {
            InfoType::Name => "NAME=",
            InfoType::OsVersion => "VERSION_ID=",
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
    use libc::c_int;

    // https://android.googlesource.com/platform/frameworks/base/+/refs/heads/master/core/java/android/os/Build.java#58
    let name: &'static [u8] = match info {
        InfoType::Name => b"ro.product.model\0",
        InfoType::OsVersion => b"ro.build.version.release\0",
    };

    let mut value_buffer = vec![0u8; libc::PROP_VALUE_MAX as usize];
    let len = unsafe {
        libc::__system_property_get(
            name.as_ptr() as *const c_char,
            value_buffer.as_mut_ptr() as *mut c_char,
        )
    };

    if len != 0 {
        if let Some(pos) = value_buffer.iter().position(|c| *c == 0) {
            value_buffer.resize(pos, 0);
        }
        String::from_utf8(value_buffer).ok()
    } else {
        None
    }
}

fn get_secs_since_epoch() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        _ => panic!("SystemTime before UNIX EPOCH!"),
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

        // Check for the "fallback" path: "/etc/lsb-release"
        assert_eq!(
            get_system_info_linux(InfoType::OsVersion, Path::new(""), &tmp2),
            Some("20.10".to_owned())
        );
        assert_eq!(
            get_system_info_linux(InfoType::Name, Path::new(""), &tmp2),
            Some("Ubuntu".to_owned())
        );
    }
}
