//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use crate::sys::component::{self, Component};
use crate::sys::disk;
use crate::sys::process::*;
use crate::sys::processor::*;
use crate::{Disk, LoadAvg, Networks, Pid, ProcessExt, RefreshKind, SystemExt, User};

use libc::{self, c_char, gid_t, sysconf, uid_t, _SC_CLK_TCK, _SC_HOST_NAME_MAX, _SC_PAGESIZE};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::utils::{into_iter, realpath};

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
                .map(|v| to_u64(v))
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

/// Structs containing system's information.
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
    uptime: u64,
    users: Vec<User>,
    boot_time: u64,
}

impl System {
    fn clear_procs(&mut self) {
        if !self.processors.is_empty() {
            let (new, old) = get_raw_times(&self.global_processor);
            let total_time = (if old > new { 1 } else { new - old }) as f32;
            let mut to_delete = Vec::with_capacity(20);

            for (pid, proc_) in &mut self.process_list.tasks {
                if !has_been_updated(proc_) {
                    to_delete.push(*pid);
                } else {
                    compute_cpu_usage(proc_, self.processors.len() as u64, total_time);
                }
            }
            for pid in to_delete {
                self.process_list.tasks.remove(&pid);
            }
        }
    }

    fn refresh_processors(&mut self, limit: Option<u32>) {
        if let Ok(f) = File::open("/proc/stat") {
            let buf = BufReader::new(f);
            let mut i: usize = 0;
            let first = self.processors.is_empty();
            let mut it = buf.split(b'\n');
            let mut count = 0;
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
                    parts.next().map(|v| to_u64(v)).unwrap_or(0),
                    parts.next().map(|v| to_u64(v)).unwrap_or(0),
                    parts.next().map(|v| to_u64(v)).unwrap_or(0),
                    parts.next().map(|v| to_u64(v)).unwrap_or(0),
                    parts.next().map(|v| to_u64(v)).unwrap_or(0),
                    parts.next().map(|v| to_u64(v)).unwrap_or(0),
                    parts.next().map(|v| to_u64(v)).unwrap_or(0),
                    parts.next().map(|v| to_u64(v)).unwrap_or(0),
                    parts.next().map(|v| to_u64(v)).unwrap_or(0),
                    parts.next().map(|v| to_u64(v)).unwrap_or(0),
                );
                count += 1;
                if let Some(limit) = limit {
                    if count >= limit {
                        return;
                    }
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
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        get_cpu_frequency(i),
                        vendor_id.clone(),
                        brand.clone(),
                    ));
                } else {
                    parts.next(); // we don't want the name again
                    self.processors[i].set(
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                        parts.next().map(|v| to_u64(v)).unwrap_or(0),
                    );
                    self.processors[i].frequency = get_cpu_frequency(i);
                }
                i += 1;
                count += 1;
                if let Some(limit) = limit {
                    if count >= limit {
                        break;
                    }
                }
            }
            if first {
                self.global_processor.vendor_id = vendor_id;
                self.global_processor.brand = brand;
            }
        }
    }
}

impl SystemExt for System {
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
            uptime: get_uptime(),
            users: Vec::new(),
            boot_time: boot_time(),
        };
        if !refreshes.cpu() {
            s.refresh_processors(None); // We need the processors to be filled.
        }
        s.refresh_specifics(refreshes);
        s
    }

    fn refresh_components_list(&mut self) {
        self.components = component::get_components();
    }

    fn refresh_memory(&mut self) {
        self.uptime = get_uptime();
        if let Ok(data) = get_all_data("/proc/meminfo", 16_385) {
            for line in data.split('\n') {
                let field = match line.split(':').next() {
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
                if let Some(val_str) = line.rsplit(' ').nth(1) {
                    if let Ok(value) = u64::from_str(val_str) {
                        // /proc/meminfo reports KiB, though it says "kB". Convert it.
                        *field = value * 128 / 125;
                    }
                }
            }
        }
    }

    fn refresh_cpu(&mut self) {
        self.uptime = get_uptime();
        self.refresh_processors(None);
    }

    fn refresh_processes(&mut self) {
        self.uptime = get_uptime();
        if refresh_procs(
            &mut self.process_list,
            Path::new("/proc"),
            self.page_size_kb,
            0,
            self.uptime,
            get_secs_since_epoch(),
        ) {
            self.clear_procs();
        }
    }

    fn refresh_process(&mut self, pid: Pid) -> bool {
        self.uptime = get_uptime();
        let found = match _get_process_data(
            &Path::new("/proc/").join(pid.to_string()),
            &mut self.process_list,
            self.page_size_kb,
            0,
            self.uptime,
            get_secs_since_epoch(),
        ) {
            Ok((Some(p), pid)) => {
                self.process_list.tasks.insert(pid, p);
                true
            }
            Ok(_) => true,
            Err(_) => false,
        };
        if found && !self.processors.is_empty() {
            self.refresh_processors(Some(1));
            let (new, old) = get_raw_times(&self.global_processor);
            let total_time = (if old >= new { 1 } else { new - old }) as f32;

            if let Some(p) = self.process_list.tasks.get_mut(&pid) {
                compute_cpu_usage(p, self.processors.len() as u64, total_time);
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

    fn get_processes(&self) -> &HashMap<Pid, Process> {
        &self.process_list.tasks
    }

    fn get_process(&self, pid: Pid) -> Option<&Process> {
        self.process_list.tasks.get(&pid)
    }

    fn get_networks(&self) -> &Networks {
        &self.networks
    }

    fn get_networks_mut(&mut self) -> &mut Networks {
        &mut self.networks
    }

    fn get_global_processor_info(&self) -> &Processor {
        &self.global_processor
    }

    fn get_processors(&self) -> &[Processor] {
        &self.processors
    }

    fn get_physical_core_count(&self) -> Option<usize> {
        get_physical_core_count()
    }

    fn get_total_memory(&self) -> u64 {
        self.mem_total
    }

    fn get_free_memory(&self) -> u64 {
        self.mem_free
    }

    fn get_available_memory(&self) -> u64 {
        self.mem_available
    }

    fn get_used_memory(&self) -> u64 {
        self.mem_total
            - self.mem_free
            - self.mem_buffers
            - self.mem_page_cache
            - self.mem_slab_reclaimable
    }

    fn get_total_swap(&self) -> u64 {
        self.swap_total
    }

    fn get_free_swap(&self) -> u64 {
        self.swap_free
    }

    // need to be checked
    fn get_used_swap(&self) -> u64 {
        self.swap_total - self.swap_free
    }

    fn get_components(&self) -> &[Component] {
        &self.components
    }

    fn get_components_mut(&mut self) -> &mut [Component] {
        &mut self.components
    }

    fn get_disks(&self) -> &[Disk] {
        &self.disks
    }

    fn get_disks_mut(&mut self) -> &mut [Disk] {
        &mut self.disks
    }

    fn get_uptime(&self) -> u64 {
        self.uptime
    }

    fn get_boot_time(&self) -> u64 {
        self.boot_time
    }

    fn get_load_average(&self) -> LoadAvg {
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

    fn get_users(&self) -> &[User] {
        &self.users
    }

    #[cfg(not(target_os = "android"))]
    fn get_name(&self) -> Option<String> {
        get_system_info_linux(
            InfoType::Name,
            Path::new("/etc/os-release"),
            Path::new("/etc/lsb-release"),
        )
    }

    #[cfg(target_os = "android")]
    fn get_name(&self) -> Option<String> {
        get_system_info_android(InfoType::Name)
    }

    fn get_long_os_version(&self) -> Option<String> {
        #[cfg(target_os = "android")]
        let system_name = "Android";

        #[cfg(not(target_os = "android"))]
        let system_name = "Linux";

        Some(format!(
            "{} {} {}",
            system_name,
            self.get_os_version().unwrap_or_default(),
            self.get_name().unwrap_or_default()
        ))
    }

    fn get_host_name(&self) -> Option<String> {
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

    fn get_kernel_version(&self) -> Option<String> {
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
    fn get_os_version(&self) -> Option<String> {
        get_system_info_linux(
            InfoType::OsVersion,
            Path::new("/etc/os-release"),
            Path::new("/etc/lsb-release"),
        )
    }

    #[cfg(target_os = "android")]
    fn get_os_version(&self) -> Option<String> {
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

struct Wrap<'a, T>(UnsafeCell<&'a mut T>);

impl<'a, T> Wrap<'a, T> {
    fn get(&self) -> &'a mut T {
        unsafe { *(self.0.get()) }
    }
}

unsafe impl<'a, T> Send for Wrap<'a, T> {}
unsafe impl<'a, T> Sync for Wrap<'a, T> {}

fn refresh_procs(
    proc_list: &mut Process,
    path: &Path,
    page_size_kb: u64,
    pid: Pid,
    uptime: u64,
    now: u64,
) -> bool {
    if let Ok(d) = fs::read_dir(path) {
        let folders = d
            .filter_map(|entry| {
                if let Ok(entry) = entry {
                    let entry = entry.path();

                    if entry.is_dir() {
                        Some(entry)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if pid == 0 {
            let proc_list = Wrap(UnsafeCell::new(proc_list));

            #[cfg(feature = "multithread")]
            use rayon::iter::ParallelIterator;

            into_iter(folders)
                .filter_map(|e| {
                    if let Ok((p, _)) = _get_process_data(
                        e.as_path(),
                        proc_list.get(),
                        page_size_kb,
                        pid,
                        uptime,
                        now,
                    ) {
                        p
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        } else {
            let mut updated_pids = Vec::with_capacity(folders.len());
            let new_tasks = folders
                .iter()
                .filter_map(|e| {
                    if let Ok((p, pid)) =
                        _get_process_data(e.as_path(), proc_list, page_size_kb, pid, uptime, now)
                    {
                        updated_pids.push(pid);
                        p
                    } else {
                        None
                    }
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
    } else {
        false
    }
}

#[allow(clippy::too_many_arguments)]
fn update_time_and_memory(
    path: &Path,
    entry: &mut Process,
    parts: &[&str],
    page_size_kb: u64,
    parent_memory: u64,
    parent_virtual_memory: u64,
    pid: Pid,
    uptime: u64,
    now: u64,
) {
    {
        // rss
        entry.memory = u64::from_str(parts[23]).unwrap_or(0) * page_size_kb;
        if entry.memory >= parent_memory {
            entry.memory -= parent_memory;
        }
        // vsz
        entry.virtual_memory = u64::from_str(parts[22]).unwrap_or(0);
        if entry.virtual_memory >= parent_virtual_memory {
            entry.virtual_memory -= parent_virtual_memory;
        }
        set_time(
            entry,
            u64::from_str(parts[13]).unwrap_or(0),
            u64::from_str(parts[14]).unwrap_or(0),
        );
    }
    refresh_procs(entry, &path.join("task"), page_size_kb, pid, uptime, now);
}

macro_rules! unwrap_or_return {
    ($data:expr) => {{
        match $data {
            Some(x) => x,
            None => return Err(()),
        }
    }};
}

fn _get_uid_and_gid(status_data: String) -> Option<(uid_t, gid_t)> {
    // We're only interested in the lines starting with Uid: and Gid:
    // here. From these lines, we're looking at the second entry to get
    // the effective u/gid.

    let f = |h: &str, n: &str| -> Option<uid_t> {
        if h.starts_with(n) {
            h.split_whitespace().nth(2).unwrap_or("0").parse().ok()
        } else {
            None
        }
    };
    let mut uid = None;
    let mut gid = None;
    for line in status_data.lines() {
        if let Some(u) = f(line, "Uid:") {
            assert!(uid.is_none());
            uid = Some(u);
        } else if let Some(g) = f(line, "Gid:") {
            assert!(gid.is_none());
            gid = Some(g);
        } else {
            continue;
        }
        if uid.is_some() && gid.is_some() {
            break;
        }
    }
    match (uid, gid) {
        (Some(u), Some(g)) => Some((u, g)),
        _ => None,
    }
}

fn parse_stat_file(data: &str) -> Result<Vec<&str>, ()> {
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
    parts.push(unwrap_or_return!(data_it.next()));
    let mut data_it = unwrap_or_return!(data_it.next()).rsplitn(2, ')');
    let data = unwrap_or_return!(data_it.next());
    parts.push(unwrap_or_return!(data_it.next()));
    parts.extend(data.split_whitespace());
    // Remove command name '('
    if let Some(name) = parts[1].strip_prefix("(") {
        parts[1] = name;
    }
    Ok(parts)
}

fn check_nb_open_files(f: File) -> Option<File> {
    if let Ok(ref mut x) = unsafe { REMAINING_FILES.lock() } {
        if **x > 0 {
            **x -= 1;
            return Some(f);
        }
    }
    // Something bad happened...
    None
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
                return Some(stripped.replace("\"", ""));
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
            return Some(stripped.replace("\"", ""));
        }
    }
    None
}

#[cfg(target_os = "android")]
fn get_system_info_android(info: InfoType) -> Option<String> {
    use libc::c_int;

    // https://android.googlesource.com/platform/bionic/+/refs/heads/master/libc/include/sys/system_properties.h#41
    const PROP_VALUE_MAX: usize = 92;

    extern "C" {
        fn __system_property_get(name: *const c_char, value: *mut c_char) -> c_int;
    }

    // https://android.googlesource.com/platform/frameworks/base/+/refs/heads/master/core/java/android/os/Build.java#58
    let name: &'static [u8] = match info {
        InfoType::Name => b"ro.product.model\0",
        InfoType::OsVersion => b"ro.build.version.release\0",
    };

    let mut value_buffer = vec![0u8; PROP_VALUE_MAX];
    let len = unsafe {
        __system_property_get(
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

fn _get_process_data(
    path: &Path,
    proc_list: &mut Process,
    page_size_kb: u64,
    pid: Pid,
    uptime: u64,
    now: u64,
) -> Result<(Option<Process>, Pid), ()> {
    let nb = match path.file_name().and_then(|x| x.to_str()).map(Pid::from_str) {
        Some(Ok(nb)) if nb != pid => nb,
        _ => return Err(()),
    };

    let get_status = |p: &mut Process, part: &str| {
        p.status = part
            .chars()
            .next()
            .map(ProcessStatus::from)
            .unwrap_or_else(|| ProcessStatus::Unknown(0));
    };
    let parent_memory = proc_list.memory;
    let parent_virtual_memory = proc_list.virtual_memory;
    if let Some(ref mut entry) = proc_list.tasks.get_mut(&nb) {
        let data = if let Some(ref mut f) = entry.stat_file {
            get_all_data_from_file(f, 1024).map_err(|_| ())?
        } else {
            let mut tmp = PathBuf::from(path);
            tmp.push("stat");
            let mut file = File::open(tmp).map_err(|_| ())?;
            let data = get_all_data_from_file(&mut file, 1024).map_err(|_| ())?;
            entry.stat_file = check_nb_open_files(file);
            data
        };
        let parts = parse_stat_file(&data)?;
        get_status(entry, parts[2]);
        update_time_and_memory(
            path,
            entry,
            &parts,
            page_size_kb,
            parent_memory,
            parent_virtual_memory,
            nb,
            uptime,
            now,
        );
        update_process_disk_activity(entry, path);
        return Ok((None, nb));
    }

    let mut tmp = PathBuf::from(path);

    tmp.push("stat");
    let mut file = std::fs::File::open(&tmp).map_err(|_| ())?;
    let data = get_all_data_from_file(&mut file, 1024).map_err(|_| ())?;
    let stat_file = check_nb_open_files(file);
    let parts = parse_stat_file(&data)?;
    let name = parts[1];

    let parent_pid = if proc_list.pid != 0 {
        Some(proc_list.pid)
    } else {
        match Pid::from_str(parts[3]) {
            Ok(p) if p != 0 => Some(p),
            _ => None,
        }
    };

    let clock_cycle = unsafe { sysconf(_SC_CLK_TCK) } as u64;
    let since_boot = u64::from_str(parts[21]).unwrap_or(0) / clock_cycle;
    let start_time = now.saturating_sub(uptime.saturating_sub(since_boot));
    let mut p = Process::new(nb, parent_pid, start_time);

    p.stat_file = stat_file;
    get_status(&mut p, parts[2]);

    tmp.pop();
    tmp.push("status");
    if let Ok(data) = get_all_data(&tmp, 16_385) {
        if let Some((uid, gid)) = _get_uid_and_gid(data) {
            p.uid = uid;
            p.gid = gid;
        }
    }

    if proc_list.pid != 0 {
        // If we're getting information for a child, no need to get those info since we
        // already have them...
        p.cmd = proc_list.cmd.clone();
        p.name = proc_list.name.clone();
        p.environ = proc_list.environ.clone();
        p.exe = proc_list.exe.clone();
        p.cwd = proc_list.cwd.clone();
        p.root = proc_list.root.clone();
    } else {
        p.name = name.into();
        tmp.pop();
        tmp.push("cmdline");
        p.cmd = copy_from_file(&tmp);
        tmp.pop();
        tmp.push("exe");
        match tmp.read_link() {
            Ok(exe_path) => {
                p.exe = exe_path;
            }
            Err(_) => {
                p.exe = PathBuf::new();
            }
        }
        tmp.pop();
        tmp.push("environ");
        p.environ = copy_from_file(&tmp);
        tmp.pop();
        tmp.push("cwd");
        p.cwd = realpath(&tmp);
        tmp.pop();
        tmp.push("root");
        p.root = realpath(&tmp);
    }

    update_time_and_memory(
        path,
        &mut p,
        &parts,
        page_size_kb,
        proc_list.memory,
        proc_list.virtual_memory,
        nb,
        uptime,
        now,
    );
    update_process_disk_activity(&mut p, path);
    Ok((Some(p), nb))
}

fn copy_from_file(entry: &Path) -> Vec<String> {
    match File::open(entry) {
        Ok(mut f) => {
            let mut data = vec![0; 16_384];

            if let Ok(size) = f.read(&mut data) {
                data.truncate(size);
                let mut out = Vec::with_capacity(20);
                let mut start = 0;
                for (pos, x) in data.iter().enumerate() {
                    if *x == 0 {
                        if pos - start >= 1 {
                            if let Ok(s) =
                                std::str::from_utf8(&data[start..pos]).map(|x| x.trim().to_owned())
                            {
                                out.push(s);
                            }
                        }
                        start = pos + 1; // to keeping prevent '\0'
                    }
                }
                out
            } else {
                Vec::new()
            }
        }
        Err(_) => Vec::new(),
    }
}

fn get_all_data_from_file(file: &mut File, size: usize) -> io::Result<String> {
    use std::io::Seek;
    let mut buf = String::with_capacity(size);
    file.seek(::std::io::SeekFrom::Start(0))?;
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

pub fn get_all_data<P: AsRef<Path>>(file_path: P, size: usize) -> io::Result<String> {
    let mut file = File::open(file_path.as_ref())?;
    get_all_data_from_file(&mut file, size)
}

fn get_uptime() -> u64 {
    let content = get_all_data("/proc/uptime", 50).unwrap_or_default();
    content
        .split('.')
        .next()
        .and_then(|t| t.parse().ok())
        .unwrap_or_default()
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
