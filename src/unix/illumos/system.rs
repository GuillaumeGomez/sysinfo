// Take a look at the license at the top of the repository in the LICENSE file.

use std::collections::{HashMap, HashSet};
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    Cpu, CpuRefreshKind, Error, LoadAvg, MemoryRefreshKind, Pid, Process, ProcessRefreshKind,
    ProcessesToUpdate,
};

use super::cpu::{CpusWrapper, possible_cpu_ids};
use super::ffi;
use super::kstat::KstatReader;
use super::process::{ProcessInfo, new_process, update_process};

const UNIX_MODULE: &CStr = c"unix";
const SYSTEM_MISC: &CStr = c"system_misc";
const CPU_INFO_MODULE: &CStr = c"cpu_info";

pub(crate) struct SystemInner {
    process_list: HashMap<Pid, Process>,
    mem_total: u64,
    mem_free: u64,
    mem_used: u64,
    mem_available: u64,
    swap_total: u64,
    swap_used: u64,
    cpus: CpusWrapper,
}

impl SystemInner {
    pub(crate) fn new() -> Result<Self, Error> {
        Ok(Self {
            process_list: HashMap::with_capacity(200),
            mem_total: 0,
            mem_free: 0,
            mem_used: 0,
            mem_available: 0,
            swap_total: 0,
            swap_used: 0,
            cpus: CpusWrapper::new(),
        })
    }

    pub(crate) fn refresh_memory_specifics(&mut self, refresh_kind: MemoryRefreshKind) {
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(0) as u64;
        if refresh_kind.ram() {
            let total_pages = unsafe { libc::sysconf(libc::_SC_PHYS_PAGES) }.max(0) as u64;
            let free_pages = unsafe { libc::sysconf(libc::_SC_AVPHYS_PAGES) }.max(0) as u64;
            self.mem_total = total_pages.saturating_mul(page_size);
            self.mem_free = free_pages.saturating_mul(page_size);
            self.mem_available = self.mem_free;
            self.mem_used = self.mem_total.saturating_sub(self.mem_free);
        }
        if refresh_kind.swap() {
            if let Some((total_pages, free_pages)) = physical_swap_pages() {
                self.swap_total = total_pages.saturating_mul(page_size);
                self.swap_used = total_pages
                    .saturating_sub(free_pages)
                    .saturating_mul(page_size);
            } else {
                sysinfo_debug!("failed to retrieve physical swap information");
            }
        }
    }

    pub(crate) fn cgroup_limits(&self) -> Option<crate::CGroupLimits> {
        None
    }

    pub(crate) fn refresh_cpu_specifics(&mut self, refresh_kind: CpuRefreshKind) {
        self.cpus.refresh(refresh_kind);
    }

    pub(crate) fn refresh_cpu_list(&mut self, refresh_kind: CpuRefreshKind) {
        self.cpus = CpusWrapper::new();
        self.cpus.refresh(refresh_kind);
    }

    pub(crate) fn refresh_processes_specifics(
        &mut self,
        processes_to_update: ProcessesToUpdate<'_>,
        refresh_kind: ProcessRefreshKind,
    ) -> usize {
        let pids = match processes_to_update {
            ProcessesToUpdate::Some(&[]) => return 0,
            ProcessesToUpdate::Some(pids) => pids.to_vec(),
            ProcessesToUpdate::All => match std::fs::read_dir("/proc") {
                Ok(entries) => entries
                    .flatten()
                    .filter_map(|entry| entry.file_name().to_str()?.parse::<i32>().ok())
                    .map(Pid)
                    .collect(),
                Err(_error) => {
                    sysinfo_debug!("failed to read /proc: {_error}");
                    return 0;
                }
            },
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut updated = 0;
        for pid in pids {
            let Some(info) = ProcessInfo::read(pid) else {
                continue;
            };
            updated += 1;
            match self.process_list.get_mut(&pid) {
                Some(process) if process.inner.start_time == info_start_time(&info) => {
                    update_process(&info, now, refresh_kind, &mut process.inner);
                }
                _ => {
                    self.process_list.insert(
                        pid,
                        Process {
                            inner: new_process(&info, now, refresh_kind),
                        },
                    );
                }
            }
        }
        updated
    }

    pub(crate) fn processes(&self) -> &HashMap<Pid, Process> {
        &self.process_list
    }

    pub(crate) fn processes_mut(&mut self) -> &mut HashMap<Pid, Process> {
        &mut self.process_list
    }

    pub(crate) fn process(&self, pid: Pid) -> Option<&Process> {
        self.process_list.get(&pid)
    }

    pub(crate) fn global_cpu_usage(&self) -> f32 {
        self.cpus.global_cpu_usage
    }

    pub(crate) fn cpus(&self) -> &[Cpu] {
        &self.cpus.cpus
    }

    pub(crate) fn total_memory(&self) -> u64 {
        self.mem_total
    }

    pub(crate) fn free_memory(&self) -> u64 {
        self.mem_free
    }

    pub(crate) fn available_memory(&self) -> u64 {
        self.mem_available
    }

    pub(crate) fn used_memory(&self) -> u64 {
        self.mem_used
    }

    pub(crate) fn total_swap(&self) -> u64 {
        self.swap_total
    }

    pub(crate) fn free_swap(&self) -> u64 {
        self.swap_total.saturating_sub(self.swap_used)
    }

    pub(crate) fn used_swap(&self) -> u64 {
        self.swap_used
    }

    pub(crate) fn uptime() -> Result<u64, Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Other("system time is before the Unix epoch".into()))?
            .as_secs();
        Ok(now.saturating_sub(Self::boot_time()?))
    }

    pub(crate) fn boot_time() -> Result<u64, Error> {
        let kstat =
            KstatReader::new().ok_or_else(|| Error::Other("failed to open libkstat".into()))?;
        let record = kstat
            .lookup(Some(UNIX_MODULE), 0, Some(SYSTEM_MISC))
            .ok_or_else(|| Error::Other("failed to read system_misc kstat".into()))?;
        record
            .integer("boot_time")
            .ok_or_else(|| Error::Other("boot_time kstat is unavailable".into()))
    }

    pub(crate) fn load_average() -> Result<LoadAvg, Error> {
        let mut load = [0f64; 3];
        if unsafe { libc::getloadavg(load.as_mut_ptr(), load.len() as _) } < 0 {
            Err(Error::Other("getloadavg failed".into()))
        } else {
            Ok(LoadAvg {
                one: load[0],
                five: load[1],
                fifteen: load[2],
            })
        }
    }

    pub(crate) fn name() -> Result<String, Error> {
        os_release_value("NAME").map_or_else(|| system_info(libc::SI_SYSNAME), Ok)
    }

    pub(crate) fn os_version() -> Result<String, Error> {
        os_release_value("VERSION_ID").map_or_else(|| system_info(libc::SI_RELEASE), Ok)
    }

    pub(crate) fn long_os_version() -> Result<String, Error> {
        if let Some(name) = os_release_value("PRETTY_NAME") {
            return Ok(name);
        }
        Ok(format!("{} {}", Self::name()?, Self::os_version()?))
    }

    pub(crate) fn host_name() -> Result<String, Error> {
        system_info(libc::SI_HOSTNAME)
    }

    pub(crate) fn kernel_version() -> Result<String, Error> {
        system_info(libc::SI_VERSION)
    }

    pub(crate) fn distribution_id() -> String {
        os_release_value("ID").unwrap_or_else(|| "illumos".to_owned())
    }

    pub(crate) fn distribution_id_like() -> Vec<String> {
        match os_release_value("ID_LIKE") {
            Some(value) => value.split_ascii_whitespace().map(String::from).collect(),
            None if Self::distribution_id() != "illumos" => vec!["illumos".to_owned()],
            None => Vec::new(),
        }
    }

    pub(crate) fn kernel_name() -> Option<&'static str> {
        Some("SunOS")
    }

    pub(crate) fn cpu_arch() -> Option<String> {
        system_info(libc::SI_ARCHITECTURE_64)
            .or_else(|_| system_info(libc::SI_ARCHITECTURE))
            .ok()
    }

    pub(crate) fn physical_core_count() -> Result<usize, Error> {
        let kstat =
            KstatReader::new().ok_or_else(|| Error::Other("failed to open libkstat".into()))?;
        let mut cores = HashSet::new();
        for id in possible_cpu_ids() {
            if let Some(info) = kstat.lookup(Some(CPU_INFO_MODULE), id, None) {
                let chip = info.integer("chip_id").unwrap_or(id as u64);
                let core = info.integer("core_id").unwrap_or(id as u64);
                cores.insert((chip, core));
            }
        }
        if cores.is_empty() {
            Err(Error::Other(
                "failed to retrieve physical core count".into(),
            ))
        } else {
            Ok(cores.len())
        }
    }

    pub(crate) fn open_files_limit() -> Result<usize, Error> {
        let mut limit = MaybeUninit::<libc::rlimit>::uninit();
        if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, limit.as_mut_ptr()) } != 0 {
            Err(Error::Other("getrlimit(RLIMIT_NOFILE) failed".into()))
        } else {
            Ok(unsafe { limit.assume_init() }.rlim_cur as usize)
        }
    }
}

fn physical_swap_pages() -> Option<(u64, u64)> {
    let count = unsafe { ffi::swapctl(ffi::SC_GETNSWP, std::ptr::null_mut()) };
    if count < 0 {
        return None;
    }
    let capacity = count as usize;
    if capacity == 0 {
        return Some((0, 0));
    }

    let table_size = std::mem::size_of::<ffi::SwapTable>()
        .checked_add(capacity.checked_mul(std::mem::size_of::<ffi::SwapEntry>())?)?;
    let word_size = std::mem::size_of::<usize>();
    let word_count = table_size.checked_add(word_size - 1)? / word_size;
    let mut table_storage = vec![0usize; word_count];
    let table = table_storage.as_mut_ptr().cast::<ffi::SwapTable>();
    unsafe { (*table).count = count };

    let path_size = libc::PATH_MAX as usize;
    let mut paths = vec![0 as libc::c_char; capacity.checked_mul(path_size)?];
    let entries =
        unsafe { std::slice::from_raw_parts_mut((*table).entries.as_mut_ptr(), capacity) };
    for (index, entry) in entries.iter_mut().enumerate() {
        entry.path = unsafe { paths.as_mut_ptr().add(index * path_size) };
    }

    let listed = unsafe { ffi::swapctl(ffi::SC_LIST, table.cast::<libc::c_void>()) };
    if listed < 0 {
        return None;
    }

    let mut total_pages = 0u64;
    let mut free_pages = 0u64;
    for entry in entries.iter().take((listed as usize).min(capacity)) {
        if entry.flags & ffi::ST_INDEL != 0 {
            continue;
        }
        total_pages = total_pages.saturating_add(entry.pages.max(0) as u64);
        free_pages = free_pages.saturating_add(entry.free.max(0) as u64);
    }
    Some((total_pages, free_pages.min(total_pages)))
}

fn info_start_time(info: &ProcessInfo) -> u64 {
    // Keep the raw procfs representation private to process.rs while still
    // allowing PID-reuse detection here.
    info.start_time()
}

fn system_info(command: libc::c_int) -> Result<String, Error> {
    let mut value = [0 as libc::c_char; 257];
    if unsafe { libc::sysinfo(command, value.as_mut_ptr(), value.len() as libc::c_long) } < 0 {
        return Err(Error::Other("sysinfo failed".into()));
    }
    Ok(unsafe { CStr::from_ptr(value.as_ptr()) }
        .to_string_lossy()
        .into_owned())
}

fn os_release_value(key: &str) -> Option<String> {
    let contents = std::fs::read_to_string("/etc/os-release").ok()?;
    let value = contents.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name == key).then_some(value.trim())
    })?;
    Some(
        value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .or_else(|| {
                value
                    .strip_prefix('\'')
                    .and_then(|value| value.strip_suffix('\''))
            })
            .unwrap_or(value)
            .to_owned(),
    )
}
