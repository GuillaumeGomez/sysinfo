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
use ::{DiskExt, ProcessExt, SystemExt};
use std::fs::{File, read_link};
use std::io::{self, BufRead, BufReader, Read};
use std::str::FromStr;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use libc::{uid_t, sysconf, _SC_CLK_TCK, _SC_PAGESIZE};
use utils::realpath;
use Pid;

use rayon::prelude::*;

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
}

macro_rules! to_str {
    ($e:expr) => {
        unsafe { ::std::str::from_utf8_unchecked($e) }
    }
}

impl SystemExt for System {
    fn new() -> System {
        let mut s = System {
            process_list: Process::new(0, None, 0),
            mem_total: 0,
            mem_free: 0,
            swap_total: 0,
            swap_free: 0,
            processors: Vec::new(),
            page_size_kb: unsafe { sysconf(_SC_PAGESIZE) as u64 / 1024 },
            temperatures: component::get_components(),
            disks: get_all_disks(),
            network: network::new(),
            uptime: get_uptime(),
        };
        s.refresh_all();
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
                    Some("MemAvailable") => &mut self.mem_free,
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
        if let Ok(f) = File::open("/proc/stat") {
            let mut buf = BufReader::new(f);
            let mut i = 0;
            let first = self.processors.is_empty();
            let mut it = buf.split(b'\n');

            while let Some(Ok(line)) = it.next() {
                if &line[..3] != b"cpu" {
                    break;
                }

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
            }
        }
    }

    fn refresh_processes(&mut self) {
        if refresh_procs(&mut self.process_list, "/proc", self.page_size_kb, 0) {
            self.clear_procs();
        }
    }

    fn refresh_process(&mut self, pid: Pid) -> bool {
        let found = match _get_process_data(&Path::new("/proc/").join(pid.to_string()), &mut self.process_list,
                                self.page_size_kb, 0) {
            Ok(Some(p)) => {
                self.process_list.tasks.insert(p.pid(), p);
                false
            }
            Ok(_) => true,
            Err(_) => false,
        };
        if found && !self.processors.is_empty() {
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
        x += (c - b'0') as u64;
    }
    x
}

fn refresh_procs<P: AsRef<Path>>(proc_list: &mut Process, path: P, page_size_kb: u64,
                                 pid: Pid) -> bool {
    if let Ok(d) = fs::read_dir(path.as_ref()) {
        let mut folders = d.filter_map(|entry| {
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
            let proc_list = proc_list as *mut Process as usize;
            folders.par_iter()
                   .filter_map(|e| {
                       let proc_list = unsafe { &mut *(proc_list as *mut Process) };
                       if let Ok(p) = _get_process_data(e.as_path(), proc_list, page_size_kb, pid) {
                           p
                       } else {
                           None
                       }
                   })
                   .collect::<Vec<_>>()
        } else {
            folders.iter()
                   .filter_map(|e| {
                       if let Ok(p) = _get_process_data(e.as_path(), proc_list, page_size_kb, pid) {
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

fn update_time_and_memory(path: &Path, entry: &mut Process, parts: &[&str], page_size_kb: u64,
                          parent_memory: u64, pid: Pid) {
    // we get the rss
    {
        if entry.memory >= parent_memory {
            entry.memory -= parent_memory;
        }
        set_time(entry,
                 u64::from_str(parts[13]).unwrap_or(0),
                 u64::from_str(parts[14]).unwrap_or(0));
    }
    refresh_procs(entry, path.join(Path::new("task")), page_size_kb, pid);
}

macro_rules! unwrap_or_return {
    ($data:expr) => {{
        match $data {
            Some(x) => x,
            None => return Err(()),
        }
    }}
}

fn _get_process_data(path: &Path, proc_list: &mut Process, page_size_kb: u64,
                     pid: Pid) -> Result<Option<Process>, ()> {
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
                update_time_and_memory(path, entry, &parts, page_size_kb, parent_memory, nb);
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

            let mut p = Process::new(nb,
                                     parent_pid,
                                     u64::from_str(parts[21]).unwrap_or(0) /
                                     unsafe { sysconf(_SC_CLK_TCK) } as u64);

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
                              .map(|x| x.split('/').last().unwrap_or("").to_owned())
                              .unwrap_or(String::new());
                tmp = PathBuf::from(path);
                tmp.push("environ");
                p.environ = copy_from_file(&tmp);
                tmp = PathBuf::from(path);
                tmp.push("exe");

                let s = read_link(tmp.to_str().unwrap_or(""));

                if s.is_ok() {
                    p.exe = if let Ok(s) = s {
                        s.to_str().unwrap_or("").to_owned()
                    } else {
                        String::new()
                    };
                }
                tmp = PathBuf::from(path);
                tmp.push("cwd");
                p.cwd = realpath(&tmp).to_str().unwrap_or("").to_owned();
                tmp = PathBuf::from(path);
                tmp.push("root");
                p.root = realpath(&tmp).to_str().unwrap_or("").to_owned();
            }

            update_time_and_memory(path, &mut p, &parts, page_size_kb, proc_list.memory, nb);
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
    #[allow(or_fun_call)]
    let content = get_all_data("/proc/mounts").unwrap_or(String::new());
    let disks = content.lines()
        .filter(|line| line.trim_left().starts_with("/dev/sd"));
    let mut ret = vec![];

    for line in disks {
        let mut split = line.split(' ');
        if let (Some(name), Some(mountpt), Some(fs)) = (split.next(), split.next(), split.next()) {
            ret.push(disk::new(name[5..].as_ref(), Path::new(mountpt), fs.as_bytes()));
        }
    }
    ret
}

#[allow(or_fun_call)]
fn get_uptime() -> u64 {
    let content = get_all_data("/proc/uptime").unwrap_or(String::new());
    u64::from_str_radix(content.split('.').next().unwrap_or("0"), 10).unwrap_or(0)
}
