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
use ::{DiskExt, ProcessExt, SystemExt};
use std::fs::{File, read_link};
use std::io::{self, Read};
use std::str::FromStr;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use libc::{pid_t, uid_t, sysconf, _SC_CLK_TCK, _SC_PAGESIZE};
use utils::realpath;

/// Structs containing system's information.
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

    /// **WARNING**: This method is specific to Linux.
    ///
    /// Refresh *only* the process corresponding to `pid`.
    pub fn refresh_process(&mut self, pid: pid_t) -> bool {
        if let Some(proc_) = self.process_list.tasks.get_mut(&pid) {
            _get_process_data(Path::new(&format!("/proc/{}", pid)), proc_, self.page_size_kb, pid);
            true
        } else {
            false
        }
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
        };
        s.refresh_all();
        s
    }

    fn refresh_system(&mut self) {
        let data = get_all_data("/proc/meminfo").unwrap();
        let lines: Vec<&str> = data.split('\n').collect();

        for component in &mut self.temperatures {
            component.update();
        }
        for line in &lines {
            match *line {
                l if l.starts_with("MemTotal:") => {
                    let parts: Vec<&str> = line.split(' ').collect();

                    self.mem_total = u64::from_str(parts[parts.len() - 2]).unwrap();
                },
                l if l.starts_with("MemAvailable:") => {
                    let parts: Vec<&str> = line.split(' ').collect();

                    self.mem_free = u64::from_str(parts[parts.len() - 2]).unwrap();
                },
                l if l.starts_with("SwapTotal:") => {
                    let parts: Vec<&str> = line.split(' ').collect();

                    self.swap_total = u64::from_str(parts[parts.len() - 2]).unwrap();
                },
                l if l.starts_with("SwapFree:") => {
                    let parts: Vec<&str> = line.split(' ').collect();

                    self.swap_free = u64::from_str(parts[parts.len() - 2]).unwrap();
                },
                _ => continue,
            }
        }
        let data = get_all_data("/proc/stat").unwrap();
        let lines: Vec<&str> = data.split('\n').collect();
        let mut i = 0;
        let first = self.processors.is_empty();
        for line in &lines {
            if !line.starts_with("cpu") {
                break;
            }

            let (parts, _): (Vec<&str>, Vec<&str>) = line.split(' ').partition(|s| !s.is_empty());
            if first {
                self.processors.push(new_processor(parts[0], u64::from_str(parts[1]).unwrap(),
                    u64::from_str(parts[2]).unwrap(),
                    u64::from_str(parts[3]).unwrap(),
                    u64::from_str(parts[4]).unwrap(),
                    u64::from_str(parts[5]).unwrap(),
                    u64::from_str(parts[6]).unwrap(),
                    u64::from_str(parts[7]).unwrap(),
                    u64::from_str(parts[8]).unwrap(),
                    u64::from_str(parts[9]).unwrap(),
                    u64::from_str(parts[10]).unwrap()));
            } else {
                set_processor(&mut self.processors[i],
                    u64::from_str(parts[1]).unwrap(),
                    u64::from_str(parts[2]).unwrap(),
                    u64::from_str(parts[3]).unwrap(),
                    u64::from_str(parts[4]).unwrap(),
                    u64::from_str(parts[5]).unwrap(),
                    u64::from_str(parts[6]).unwrap(),
                    u64::from_str(parts[7]).unwrap(),
                    u64::from_str(parts[8]).unwrap(),
                    u64::from_str(parts[9]).unwrap(),
                    u64::from_str(parts[10]).unwrap());
                i += 1;
            }
        }
    }

    fn refresh_processes(&mut self) {
        if refresh_procs(&mut self.process_list, "/proc", self.page_size_kb, 0) {
            self.clear_procs();
        }
    }

    fn refresh_disks(&mut self) {
        for disk in &mut self.disks {
            disk.update();
        }
    }

    fn refresh_disk_list(&mut self) {
        self.disks = get_all_disks();
    }

    // COMMON PART
    //
    // Need to be moved into a "common" file to avoid duplication.

    fn get_process_list(&self) -> &HashMap<pid_t, Process> {
        &self.process_list.tasks
    }

    fn get_process(&self, pid: pid_t) -> Option<&Process> {
        self.process_list.tasks.get(&pid)
    }

    fn get_process_by_name(&self, name: &str) -> Vec<&Process> {
        let mut ret = vec!();
        for val in self.process_list.tasks.values() {
            if val.name.starts_with(name) {
                ret.push(val);
            }
        }
        ret
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
}

impl Default for System {
    fn default() -> System {
        System::new()
    }
}

pub fn get_all_data(file_path: &str) -> io::Result<String> {
    let mut file = File::open(file_path)?;
    let mut data = String::new();

    file.read_to_string(&mut data).unwrap();
    Ok(data)
}

fn refresh_procs<P: AsRef<Path>>(proc_list: &mut Process, path: P, page_size_kb: u64,
                                 pid: pid_t) -> bool {
    if let Ok(d) = fs::read_dir(&Path::new(path.as_ref())) {
        for entry in d {
            if !entry.is_ok() {
                continue;
            }
            let entry = entry.unwrap();
            let entry = entry.path();

            if entry.is_dir() {
                _get_process_data(entry.as_path(), proc_list, page_size_kb, pid);
            }
        }
        true
    } else {
        false
    }
}

fn update_time_and_memory(path: &Path, entry: &mut Process, parts: &[&str], page_size_kb: u64,
                          parent_memory: u64, pid: pid_t) {
    //entry.name = parts[1][1..].to_owned();
    //entry.name.pop();
    // we get the rss
    {
        entry.memory = u64::from_str(parts[23]).unwrap() * page_size_kb;
        if entry.memory >= parent_memory {
            entry.memory -= parent_memory;
        }
        set_time(entry,
                 u64::from_str(parts[13]).unwrap(),
                 u64::from_str(parts[14]).unwrap());
    }
    refresh_procs(entry, path.join(Path::new("task")).as_path(), page_size_kb, pid);
}

fn _get_process_data(path: &Path, proc_list: &mut Process, page_size_kb: u64, pid: pid_t) {
    if !path.exists() || !path.is_dir() {
        return
    }
    let last = path.to_str().unwrap().split('/').last();
    if last.is_none() {
        return
    }
    if let Ok(nb) = pid_t::from_str(last.unwrap()) {
        if nb == pid {
            return
        }
        let mut tmp = PathBuf::from(path);

        tmp.push("stat");
        let data = get_all_data(tmp.to_str().unwrap()).unwrap();

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
        parts.push(data_it.next().unwrap());
        // The following loses the ) from the input, but that's ok because
        // we're not using it anyway.
        let mut data_it = data_it.next().unwrap().rsplitn(2, ')');
        let data = data_it.next().unwrap();
        parts.push(data_it.next().unwrap());
        parts.extend(data.split_whitespace());
        let parent_memory = proc_list.memory;
        if let Some(ref mut entry) = proc_list.tasks.get_mut(&nb) {
            update_time_and_memory(path, entry, &parts, page_size_kb, parent_memory, nb);
            return;
        }

        let parent_pid = if proc_list.pid != 0 {
            Some(proc_list.pid)
        } else {
            match pid_t::from_str(parts[3]).unwrap() {
                0 => None,
                p => Some(p),
            }
        };

        let mut p = Process::new(nb,
                                 parent_pid,
                                 u64::from_str(parts[21]).unwrap() /
                                 unsafe { sysconf(_SC_CLK_TCK) } as u64);

        p.status = parts[2].chars().next().and_then(|c| Some(ProcessStatus::from(c)));

        tmp = PathBuf::from(path);
        tmp.push("status");
        let status_data = get_all_data(tmp.to_str().unwrap()).unwrap();

        // We're only interested in the lines starting with Uid: and Gid:
        // here. From these lines, we're looking at the second entry to get
        // the effective u/gid.

        let f = |h: &str, n: &str| -> Option<uid_t> {
            if h.starts_with(n) {
                h.split_whitespace().nth(2).unwrap().parse().ok()
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
            p.name = p.cmd[0].split('/').last().unwrap().to_owned();
            tmp = PathBuf::from(path);
            tmp.push("environ");
            p.environ = copy_from_file(&tmp);
            tmp = PathBuf::from(path);
            tmp.push("exe");

            let s = read_link(tmp.to_str().unwrap());

            if s.is_ok() {
                p.exe = s.unwrap().to_str().unwrap().to_owned();
            }
            tmp = PathBuf::from(path);
            tmp.push("cwd");
            p.cwd = realpath(Path::new(tmp.to_str().unwrap())).to_str().unwrap().to_owned();
            tmp = PathBuf::from(path);
            tmp.push("root");
            p.root = realpath(Path::new(tmp.to_str().unwrap())).to_str().unwrap().to_owned();
        }

        update_time_and_memory(path, &mut p, &parts, page_size_kb, proc_list.memory, nb);
        proc_list.tasks.insert(nb, p);
    }
}

#[allow(unused_must_use)] 
fn copy_from_file(entry: &Path) -> Vec<String> {
    match File::open(entry.to_str().unwrap()) {
        Ok(mut f) => {
            let mut d = String::new();

            f.read_to_string(&mut d);
            d.split('\0').map(|x| x.to_owned()).collect()
        },
        Err(_) => Vec::new()
    }
}

fn get_all_disks() -> Vec<Disk> {
    #[allow(or_fun_call)]
    let content = get_all_data("/proc/mounts").unwrap_or(String::new());
    let disks: Vec<_> = content.lines()
                               .filter(|line| line.trim_left()
                                                  .starts_with("/dev/sd"))
                               .collect();
    let mut ret = Vec::with_capacity(disks.len());

    if disks.len() == 1 {
        let info: Vec<_> = disks[0].split(' ').collect();
        if info.len() < 3 {
            return ret;
        }
        ret.push(disk::new_disk("sda", info[1], info[2]));
    } else {
        for line in disks {
            let info: Vec<_> = line.split(' ').collect();
            if info.len() < 3 {
                continue
            }
            ret.push(disk::new_disk(&info[0][5..], info[1], info[2]));
        }
    }
    ret
}
