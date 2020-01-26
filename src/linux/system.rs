//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use sys::component::{self, Component};
use sys::disk;
use sys::network;
use sys::process::*;
use sys::processor::*;
use sys::Disk;
use sys::NetworkData;
use LoadAvg;
use Pid;
use {DiskExt, ProcessExt, RefreshKind, SystemExt};

use libc::{self, gid_t, sysconf, uid_t, _SC_CLK_TCK, _SC_PAGESIZE};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, read_link, File};
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use utils::realpath;

use rayon::prelude::*;

// This whole thing is to prevent having too much files open at once. It could be problematic
// for processes using a lot of files and using sysinfo at the same time.
pub(crate) static mut REMAINING_FILES: once_cell::sync::Lazy<Arc<Mutex<isize>>> =
    once_cell::sync::Lazy::new(|| {
        #[cfg(target_os = "android")]
        {
            // The constant "RLIMIT_NOFILE" doesn't exist on Android so we have to return a value.
            // The default value seems to be 1024 so let's return 50% of it...
            Arc::new(Mutex::new(1024 / 2))
        }
        #[cfg(not(target_os = "android"))]
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
    #[cfg(target_os = "android")]
    {
        // The constant "RLIMIT_NOFILE" doesn't exist on Android so we have to return a value.
        // The default value seems to be 1024...
        1024 / 2
    }
    #[cfg(not(target_os = "android"))]
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
        unsafe { ::std::str::from_utf8_unchecked($e) }
    };
}

/// Structs containing system's information.
#[derive(Debug)]
pub struct System {
    process_list: Process,
    mem_total: u64,
    mem_free: u64,
    swap_total: u64,
    swap_free: u64,
    processors: Vec<Processor>,
    page_size_kb: u64,
    temperatures: Vec<Component>,
    disks: Vec<Disk>,
    network: NetworkData,
    uptime: u64,
}

impl System {
    fn clear_procs(&mut self) {
        if !self.processors.is_empty() {
            let (new, old) = get_raw_times(&self.processors[0]);
            let total_time = (if old > new { 1 } else { new - old }) as f32;
            let mut to_delete = Vec::with_capacity(20);
            let nb_processors = self.processors.len() as u64 - 1;

            for (pid, proc_) in &mut self.process_list.tasks {
                if !has_been_updated(proc_) {
                    to_delete.push(*pid);
                } else {
                    compute_cpu_usage(proc_, nb_processors, total_time);
                }
            }
            for pid in to_delete {
                self.process_list.tasks.remove(&pid);
            }
        }
    }

    fn refresh_processors(&mut self, limit: Option<u32>) {
        fn get_callbacks(
            first: bool,
        ) -> Box<dyn Fn(&mut dyn Iterator<Item = &[u8]>, &mut Vec<Processor>, &mut usize)> {
            if first {
                let frequency = get_cpu_frequency();
                let vendor_id = get_vendor_id();
                Box::new(
                    move |parts: &mut dyn Iterator<Item = &[u8]>,
                          processors: &mut Vec<Processor>,
                          _| {
                        processors.push(Processor::new_with_values(
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
                            frequency,
                            vendor_id.clone(),
                        ));
                    },
                )
            } else {
                Box::new(
                    |parts: &mut dyn Iterator<Item = &[u8]>,
                     processors: &mut Vec<Processor>,
                     i: &mut usize| {
                        parts.next(); // we don't want the name again
                        processors[*i].set(
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
                        *i += 1;
                    },
                )
            }
        }
        if let Ok(f) = File::open("/proc/stat") {
            let buf = BufReader::new(f);
            let mut i = 0;
            let first = self.processors.is_empty();
            let mut it = buf.split(b'\n');
            let mut count = 0;
            let callback = get_callbacks(first);

            while let Some(Ok(line)) = it.next() {
                if &line[..3] != b"cpu" {
                    break;
                }

                count += 1;
                let mut parts = line.split(|x| *x == b' ').filter(|s| !s.is_empty());
                callback(&mut parts, &mut self.processors, &mut i);
                if let Some(limit) = limit {
                    if count >= limit {
                        break;
                    }
                }
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
            swap_total: 0,
            swap_free: 0,
            processors: Vec::with_capacity(4),
            page_size_kb: unsafe { sysconf(_SC_PAGESIZE) as u64 / 1024 },
            temperatures: component::get_components(),
            disks: Vec::with_capacity(2),
            network: network::new(),
            uptime: get_uptime(),
        };
        s.refresh_specifics(refreshes);
        s
    }

    fn refresh_memory(&mut self) {
        self.uptime = get_uptime();
        if let Ok(data) = get_all_data("/proc/meminfo", 16_385) {
            for line in data.split('\n') {
                let field = match line.split(':').next() {
                    Some("MemTotal") => &mut self.mem_total,
                    Some("MemAvailable") | Some("MemFree") => &mut self.mem_free,
                    Some("SwapTotal") => &mut self.swap_total,
                    Some("SwapFree") => &mut self.swap_free,
                    _ => continue,
                };
                if let Some(val_str) = line.rsplit(' ').nth(1) {
                    if let Ok(value) = u64::from_str(val_str) {
                        *field = value;
                    }
                }
            }
        }
    }

    fn refresh_cpu(&mut self) {
        self.uptime = get_uptime();
        self.refresh_processors(None);
    }

    fn refresh_temperatures(&mut self) {
        for component in &mut self.temperatures {
            component.update();
        }
    }

    fn refresh_processes(&mut self) {
        self.uptime = get_uptime();
        if refresh_procs(
            &mut self.process_list,
            "/proc",
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
            Ok(Some(p)) => {
                self.process_list.tasks.insert(p.pid(), p);
                false
            }
            Ok(_) => true,
            Err(_) => false,
        };
        if found && !self.processors.is_empty() {
            self.refresh_processors(Some(1));
            let (new, old) = get_raw_times(&self.processors[0]);
            let total_time = (if old > new { 1 } else { new - old }) as f32;
            let nb_processors = self.processors.len() as u64 - 1;

            if let Some(p) = self.process_list.tasks.get_mut(&pid) {
                compute_cpu_usage(p, nb_processors, total_time);
            }
        }
        found
    }

    fn refresh_disks(&mut self) {
        for disk in &mut self.disks {
            disk.update();
        }
    }

    fn refresh_disk_list(&mut self) {
        self.disks = get_all_disks();
    }

    fn refresh_network(&mut self) {
        network::update_network(&mut self.network);
    }

    // COMMON PART
    //
    // Need to be moved into a "common" file to avoid duplication.

    fn get_process_list(&self) -> &HashMap<Pid, Process> {
        &self.process_list.tasks
    }

    fn get_process(&self, pid: Pid) -> Option<&Process> {
        self.process_list.tasks.get(&pid)
    }

    fn get_network(&self) -> &NetworkData {
        &self.network
    }

    fn get_processor_list(&self) -> &[Processor] {
        &self.processors[..]
    }

    fn get_total_memory(&self) -> u64 {
        self.mem_total
    }

    fn get_free_memory(&self) -> u64 {
        self.mem_free
    }

    fn get_used_memory(&self) -> u64 {
        self.mem_total - self.mem_free
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

    fn get_components_list(&self) -> &[Component] {
        &self.temperatures[..]
    }

    fn get_disks(&self) -> &[Disk] {
        &self.disks[..]
    }

    fn get_uptime(&self) -> u64 {
        self.uptime
    }

    fn get_load_average(&self) -> LoadAvg {
        let mut s = String::new();
        if let Err(_) = File::open("/proc/loadavg").and_then(|mut f| f.read_to_string(&mut s)) {
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

fn refresh_procs<P: AsRef<Path>>(
    proc_list: &mut Process,
    path: P,
    page_size_kb: u64,
    pid: Pid,
    uptime: u64,
    now: u64,
) -> bool {
    if let Ok(d) = fs::read_dir(path.as_ref()) {
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
            folders
                .par_iter()
                .filter_map(|e| {
                    if let Ok(p) = _get_process_data(
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
            folders
                .iter()
                .filter_map(|e| {
                    if let Ok(p) =
                        _get_process_data(e.as_path(), proc_list, page_size_kb, pid, uptime, now)
                    {
                        p
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
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
        entry.virtual_memory = u64::from_str(parts[22]).unwrap_or(0) * page_size_kb;
        if entry.virtual_memory >= parent_virtual_memory {
            entry.virtual_memory -= parent_virtual_memory;
        }
        set_time(
            entry,
            u64::from_str(parts[13]).unwrap_or(0),
            u64::from_str(parts[14]).unwrap_or(0),
        );
    }
    refresh_procs(
        entry,
        path.join(Path::new("task")),
        page_size_kb,
        pid,
        uptime,
        now,
    );
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
    // The following loses the ) from the input, but that's ok because
    // we're not using it anyway.
    let mut data_it = unwrap_or_return!(data_it.next()).rsplitn(2, ')');
    let data = unwrap_or_return!(data_it.next());
    parts.push(unwrap_or_return!(data_it.next()));
    parts.extend(data.split_whitespace());
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
    return None;
}

fn _get_process_data(
    path: &Path,
    proc_list: &mut Process,
    page_size_kb: u64,
    pid: Pid,
    uptime: u64,
    now: u64,
) -> Result<Option<Process>, ()> {
    let nb = match path.file_name().and_then(|x| x.to_str()).map(Pid::from_str) {
        Some(Ok(nb)) if nb != pid => nb,
        _ => return Err(()),
    };

    let get_status = |p: &mut Process, part: &str| {
        p.status = part
            .chars()
            .next()
            .and_then(|c| Some(ProcessStatus::from(c)))
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
            let mut file = ::std::fs::File::open(tmp).map_err(|_| ())?;
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
        return Ok(None);
    }

    let mut tmp = PathBuf::from(path);

    tmp.push("stat");
    let mut file = ::std::fs::File::open(&tmp).map_err(|_| ())?;
    let data = get_all_data_from_file(&mut file, 1024).map_err(|_| ())?;
    let stat_file = check_nb_open_files(file);
    let parts = parse_stat_file(&data)?;

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
    let start_time = now
        .checked_sub(uptime.checked_sub(since_boot).unwrap_or_else(|| 0))
        .unwrap_or_else(|| 0);
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
        tmp.pop();
        tmp.push("cmdline");
        p.cmd = copy_from_file(&tmp);
        p.name = p
            .cmd
            .get(0)
            .map(|x| x.split('/').last().unwrap_or_else(|| "").to_owned())
            .unwrap_or_default();
        tmp.pop();
        tmp.push("environ");
        p.environ = copy_from_file(&tmp);
        tmp.pop();
        tmp.push("exe");
        p.exe = read_link(tmp.to_str().unwrap_or_else(|| "")).unwrap_or_else(|_| PathBuf::new());

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
    Ok(Some(p))
}

fn copy_from_file(entry: &Path) -> Vec<String> {
    match File::open(entry.to_str().unwrap_or("/")) {
        Ok(mut f) => {
            let mut data = vec![0; 16_384];

            if let Ok(size) = f.read(&mut data) {
                data.truncate(size);
                let mut out = Vec::with_capacity(20);
                let mut start = 0;
                for (pos, x) in data.iter().enumerate() {
                    if *x == 0 {
                        if pos - start > 1 {
                            if let Ok(s) = ::std::str::from_utf8(&data[start..pos])
                                .map(|x| x.trim().to_owned())
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

    let mut data = Vec::with_capacity(size);
    unsafe {
        data.set_len(size);
    }

    file.seek(::std::io::SeekFrom::Start(0))?;
    let size = file.read(&mut data)?;
    data.truncate(size);
    Ok(String::from_utf8(data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.description()))?)
}

pub fn get_all_data<P: AsRef<Path>>(file_path: P, size: usize) -> io::Result<String> {
    let mut file = File::open(file_path.as_ref())?;
    get_all_data_from_file(&mut file, size)
}

fn get_all_disks() -> Vec<Disk> {
    let content = get_all_data("/proc/mounts", 16_385).unwrap_or_default();
    let disks = content.lines().filter(|line| {
        let line = line.trim_start();
        // While the `sd` prefix is most common, some disks instead use the `nvme` prefix. This
        // prefix refers to NVM (non-volatile memory) cabale SSDs. These disks run on the NVMe
        // storage controller protocol (not the scsi protocol) and as a result use a different
        // prefix to support NVMe namespaces.
        //
        // In some other cases, it uses a device mapper to map physical block devices onto
        // higher-level virtual block devices (on `/dev/mapper`).
        //
        // Raspbian uses root and mmcblk for physical disks
        line.starts_with("/dev/sd")
            || line.starts_with("/dev/nvme")
            || line.starts_with("/dev/mapper/")
            || line.starts_with("/dev/root")
            || line.starts_with("/dev/mmcblk")
    });
    let mut ret = vec![];

    for line in disks {
        let mut split = line.split(' ');
        if let (Some(name), Some(mountpt), Some(fs)) = (split.next(), split.next(), split.next()) {
            ret.push(disk::new(
                name[5..].as_ref(),
                Path::new(mountpt),
                fs.as_bytes(),
            ));
        }
    }
    ret
}

fn get_uptime() -> u64 {
    let content = get_all_data("/proc/uptime", 50).unwrap_or_default();
    u64::from_str_radix(content.split('.').next().unwrap_or_else(|| "0"), 10).unwrap_or_else(|_| 0)
}

fn get_secs_since_epoch() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}
