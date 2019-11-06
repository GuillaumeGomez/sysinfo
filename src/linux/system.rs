//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use sys::component::{self, Component};
use sys::processor::*;
use sys::process::*;
use sys::Disk;
use sys::disk;
use sys::network;
use sys::NetworkData;
use ::{DiskExt, ProcessExt, RefreshKind, SystemExt};
use Pid;

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::fs::{self, File, read_link};
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;
use libc::{uid_t, sysconf, _SC_CLK_TCK, _SC_PAGESIZE};

use utils::realpath;

use rayon::prelude::*;

macro_rules! to_str {
    ($e:expr) => {
        unsafe { ::std::str::from_utf8_unchecked($e) }
    }
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
            let mut to_delete = Vec::new();
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
        if let Ok(f) = File::open("/proc/stat") {
            let buf = BufReader::new(f);
            let mut i = 0;
            let first = self.processors.is_empty();
            let mut it = buf.split(b'\n');
            let mut count = 0;

            while let Some(Ok(line)) = it.next() {
                if &line[..3] != b"cpu" {
                    break;
                }

                count += 1;
                let mut parts = line.split(|x| *x == b' ').filter(|s| !s.is_empty());
                if first {
                    self.processors.push(new_processor(
                        to_str!(parts.next().unwrap_or(&[])),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0)));
                } else {
                    parts.next(); // we don't want the name again
                    set_processor(&mut self.processors[i],
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0),
                        parts.next().map(|v| {to_u64(v)}).unwrap_or(0));
                    i += 1;
                }
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
            processors: Vec::new(),
            page_size_kb: unsafe { sysconf(_SC_PAGESIZE) as u64 / 1024 },
            temperatures: component::get_components(),
            disks: Vec::new(),
            network: network::new(),
            uptime: get_uptime(),
        };
        s.refresh_specifics(refreshes);
        s
    }

    fn refresh_system(&mut self) {
        self.uptime = get_uptime();
        for component in &mut self.temperatures {
            component.update();
        }
        if let Ok(data) = get_all_data("/proc/meminfo") {
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
        self.refresh_processors(None);
    }

    fn refresh_processes(&mut self) {
        self.uptime = get_uptime();
        if refresh_procs(&mut self.process_list, "/proc", self.page_size_kb, 0, self.uptime,
                         get_secs_since_epoch()) {
            self.clear_procs();
        }
    }

    fn refresh_process(&mut self, pid: Pid) -> bool {
        self.uptime = get_uptime();
        let found = match _get_process_data(&Path::new("/proc/").join(pid.to_string()),
                                            &mut self.process_list, self.page_size_kb, 0,
                                            self.uptime, get_secs_since_epoch()) {
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
}

impl Default for System {
    fn default() -> System {
        System::new()
    }
}

pub fn get_all_data<P: AsRef<Path>>(file_path: P) -> io::Result<String> {
    use std::error::Error;
    let mut file = File::open(file_path.as_ref())?;
    let mut data = vec![0; 16_385];

    let size = file.read(&mut data)?;
    data.truncate(size);
    let data = String::from_utf8(data).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput,
                                                                  e.description()))?;
    Ok(data)
}

fn to_u64(v: &[u8]) -> u64 {
    let mut x = 0;

    for c in v {
        x *= 10;
        x += u64::from(c - b'0');
    }
    x
}

struct Wrap<'a>(UnsafeCell<&'a mut Process>);

impl<'a> Wrap<'a> {
    fn get(&self) -> &'a mut Process {
        unsafe { *(self.0.get()) }
    }
}

unsafe impl<'a> Send for Wrap<'a> {}
unsafe impl<'a> Sync for Wrap<'a> {}

fn refresh_procs<P: AsRef<Path>>(
    proc_list: &mut Process,
    path: P,
    page_size_kb: u64,
    pid: Pid,
    uptime: u64,
    now: u64,
) -> bool {
    if let Ok(d) = fs::read_dir(path.as_ref()) {
        let folders = d.filter_map(|entry| {
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
        }).collect::<Vec<_>>();
        if pid == 0 {
            let proc_list = Wrap(UnsafeCell::new(proc_list));
            folders.par_iter()
                   .filter_map(|e| {
                       if let Ok(p) = _get_process_data(e.as_path(),
                                                        proc_list.get(),
                                                        page_size_kb,
                                                        pid,
                                                        uptime,
                                                        now) {
                           p
                       } else {
                           None
                       }
                   })
                   .collect::<Vec<_>>()
        } else {
            folders.iter()
                   .filter_map(|e| {
                       if let Ok(p) = _get_process_data(e.as_path(), proc_list, page_size_kb, pid,
                                                        uptime, now) {
                           p
                       } else {
                           None
                       }
                   })
                   .collect::<Vec<_>>()
        }.into_iter().for_each(|e| {
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
    pid: Pid,
    uptime: u64,
    now: u64,
) {
    // we get the rss
    {
        entry.memory = u64::from_str(parts[23]).unwrap_or(0) * page_size_kb;
        if entry.memory >= parent_memory {
            entry.memory -= parent_memory;
        }
        set_time(entry,
                 u64::from_str(parts[13]).unwrap_or(0),
                 u64::from_str(parts[14]).unwrap_or(0));
    }
    refresh_procs(entry, path.join(Path::new("task")), page_size_kb, pid, uptime, now);
}

macro_rules! unwrap_or_return {
    ($data:expr) => {{
        match $data {
            Some(x) => x,
            None => return Err(()),
        }
    }}
}

fn _get_process_data(
    path: &Path,
    proc_list: &mut Process,
    page_size_kb: u64,
    pid: Pid,
    uptime: u64,
    now: u64,
) -> Result<Option<Process>, ()> {
    if let Some(Ok(nb)) = path.file_name().and_then(|x| x.to_str()).map(Pid::from_str) {
        if nb == pid {
            return Err(());
        }
        let mut tmp = PathBuf::from(path);

        tmp.push("stat");
        if let Ok(data) = get_all_data(&tmp) {

            // The stat file is "interesting" to parse, because spaces cannot
            // be used as delimiters. The second field stores the command name
            // sourrounded by parentheses. Unfortunately, whitespace and
            // parentheses are legal parts of the command, so parsing has to
            // proceed like this: The first field is delimited by the first
            // whitespace, the second field is everything until the last ')'
            // in the entire string. All other fields are delimited by
            // whitespace.

            let mut parts = Vec::new();
            let mut data_it = data.splitn(2, ' ');
            parts.push(unwrap_or_return!(data_it.next()));
            // The following loses the ) from the input, but that's ok because
            // we're not using it anyway.
            let mut data_it = unwrap_or_return!(data_it.next()).rsplitn(2, ')');
            let data = unwrap_or_return!(data_it.next());
            parts.push(unwrap_or_return!(data_it.next()));
            parts.extend(data.split_whitespace());
            let parent_memory = proc_list.memory;
            if let Some(ref mut entry) = proc_list.tasks.get_mut(&nb) {
                update_time_and_memory(path, entry, &parts, page_size_kb, parent_memory, nb, uptime,
                                       now);
                return Ok(None);
            }

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
            let start_time = now.checked_sub(
                uptime.checked_sub(since_boot).unwrap_or_else(|| 0),
            ).unwrap_or_else(|| 0);
            let mut p = Process::new(nb, parent_pid, start_time);

            p.status = parts[2].chars()
                               .next()
                               .and_then(|c| Some(ProcessStatus::from(c)))
                               .unwrap_or(ProcessStatus::Unknown(0));

            tmp = PathBuf::from(path);
            tmp.push("status");
            if let Ok(status_data) = get_all_data(&tmp) {
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
                let mut set_uid = false;
                let mut set_gid = false;
                for line in status_data.lines() {
                    if let Some(u) = f(line, "Uid:") {
                        assert!(!set_uid);
                        set_uid = true;
                        p.uid = u;
                    }
                    if let Some(g) = f(line, "Gid:") {
                        assert!(!set_gid);
                        set_gid = true;
                        p.gid = g;
                    }
                }
                assert!(set_uid && set_gid);
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
                tmp = PathBuf::from(path);
                tmp.push("cmdline");
                p.cmd = copy_from_file(&tmp);
                p.name = p.cmd.get(0)
                              .map(|x| x.split('/').last().unwrap_or_else(|| "").to_owned())
                              .unwrap_or_default();
                tmp = PathBuf::from(path);
                tmp.push("environ");
                p.environ = copy_from_file(&tmp);
                tmp = PathBuf::from(path);
                tmp.push("exe");

                p.exe = read_link(tmp.to_str()
                                     .unwrap_or_else(|| "")).unwrap_or_else(|_| PathBuf::new());

                tmp = PathBuf::from(path);
                tmp.push("cwd");
                p.cwd = realpath(&tmp);
                tmp = PathBuf::from(path);
                tmp.push("root");
                p.root = realpath(&tmp);
            }

            update_time_and_memory(path, &mut p, &parts, page_size_kb, proc_list.memory, nb, uptime,
                                   now);
            return Ok(Some(p));
        }
    }
    Err(())
}

fn copy_from_file(entry: &Path) -> Vec<String> {
    match File::open(entry.to_str().unwrap_or("/")) {
        Ok(mut f) => {
            let mut data = vec![0; 16_384];

            if let Ok(size) = f.read(&mut data) {
                data.truncate(size);
                data.split(|x| *x == b'\0')
                    .filter_map(|x| ::std::str::from_utf8(x).map(|x| x.trim().to_owned()).ok())
                    .filter(|x| !x.is_empty())
                    .collect()
            } else {
                Vec::new()
            }
        }
        Err(_) => Vec::new(),
    }
}

fn get_all_disks() -> Vec<Disk> {
    let content = get_all_data("/proc/mounts").unwrap_or_default();
    let disks = content.lines()
        .filter(|line| {
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
            line.starts_with("/dev/sd") ||
            line.starts_with("/dev/nvme") ||
            line.starts_with("/dev/mapper/") ||
            line.starts_with("/dev/root") ||
            line.starts_with("/dev/mmcblk")
        });
    let mut ret = vec![];

    for line in disks {
        let mut split = line.split(' ');
        if let (Some(name), Some(mountpt), Some(fs)) = (split.next(), split.next(), split.next()) {
            ret.push(disk::new(name[5..].as_ref(), Path::new(mountpt), fs.as_bytes()));
        }
    }
    ret
}

fn get_uptime() -> u64 {
    let content = get_all_data("/proc/uptime").unwrap_or_default();
    u64::from_str_radix(content.split('.').next().unwrap_or_else(|| "0"), 10).unwrap_or_else(|_| 0)
}

fn get_secs_since_epoch() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}
