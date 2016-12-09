// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use sys::component::Component;
use sys::processor::*;
use sys::process::*;
use std::fs::{File, read_link};
use std::io::Read;
use std::str::FromStr;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use libc::{stat, lstat, c_char, sysconf, _SC_CLK_TCK, _SC_PAGESIZE, S_IFLNK, S_IFMT};

pub struct System {
    process_list: HashMap<usize, Process>,
    mem_total: u64,
    mem_free: u64,
    swap_total: u64,
    swap_free: u64,
    processors: Vec<Processor>,
    page_size_kb: u64,
    temperatures: Vec<Component>,
}

impl System {
    pub fn new() -> System {
        let mut s = System {
            process_list: HashMap::new(),
            mem_total: 0,
            mem_free: 0,
            swap_total: 0,
            swap_free: 0,
            processors: Vec::new(),
            page_size_kb: unsafe { sysconf(_SC_PAGESIZE) as u64 / 1024 },
            temperatures: Component::get_components(),
        };
        s.refresh_all();
        s
    }

    pub fn refresh_system(&mut self) {
        let data = get_all_data("/proc/meminfo");
        let lines: Vec<&str> = data.split('\n').collect();

        for component in self.temperatures.iter_mut() {
            component.update();
        }
        for line in lines.iter() {
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
        let data = get_all_data("/proc/stat");
        let lines: Vec<&str> = data.split('\n').collect();
        let mut i = 0;
        let first = self.processors.len() == 0;
        for line in lines.iter() {
            if !line.starts_with("cpu") {
                break;
            }

            let (parts, _): (Vec<&str>, Vec<&str>) = line.split(' ').partition(|s| s.len() > 0);
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
                set_processor(self.processors.get_mut(i).unwrap(),
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

    pub fn refresh_process(&mut self) {
        match fs::read_dir(&Path::new("/proc")) {
            Ok(d) => {
                for entry in d {
                    if !entry.is_ok() {
                        continue;
                    }
                    let entry = entry.unwrap();
                    let entry = entry.path();

                    if entry.is_dir() {
                        _get_process_data(entry.as_path(), &mut self.process_list, self.page_size_kb);
                    }
                }
                self.clear_procs();
            }
            Err(_) => {}
        }
    }

    fn clear_procs(&mut self) {
        if self.processors.len() > 0 {
            let (new, old) = get_raw_times(&self.processors[0]);
            let total_time = (new - old) as f32;
            let mut to_delete = Vec::new();
            let nb_processors = self.processors.len() as u64 - 1;

            for (pid, proc_) in self.process_list.iter_mut() {
                if has_been_updated(&proc_) == false {
                    to_delete.push(*pid);
                } else {
                    compute_cpu_usage(proc_, nb_processors, total_time);
                }
            }
            for pid in to_delete {
                self.process_list.remove(&pid);
            }
        }
    }
    
    // COMMON PART
    //
    // Need to be moved into a "common" file to avoid duplication.

    pub fn refresh_all(&mut self) {
        self.refresh_system();
        self.refresh_process();
    }

    pub fn get_process_list<'a>(&'a self) -> &'a HashMap<usize, Process> {
        &self.process_list
    }

    /// Return the process corresponding to the given pid or None if no such process exists.
    pub fn get_process(&self, pid: i64) -> Option<&Process> {
        self.process_list.get(&(pid as usize))
    }

    /// Return a list of process starting with the given name.
    pub fn get_process_by_name(&self, name: &str) -> Vec<&Process> {
        let mut ret = vec!();
        for val in self.process_list.values() {
            if val.name.starts_with(name) {
                ret.push(val);
            }
        }
        ret
    }

    /// The first process in the array is the "main" process
    pub fn get_processor_list<'a>(&'a self) -> &'a [Processor] {
        &self.processors[..]
    }

    pub fn get_total_memory(&self) -> u64 {
        self.mem_total
    }

    pub fn get_free_memory(&self) -> u64 {
        self.mem_free
    }

    pub fn get_used_memory(&self) -> u64 {
        self.mem_total - self.mem_free
    }

    pub fn get_total_swap(&self) -> u64 {
        self.swap_total
    }

    pub fn get_free_swap(&self) -> u64 {
        self.swap_free
    }

    // need to be checked
    pub fn get_used_swap(&self) -> u64 {
        self.swap_total - self.swap_free
    }

    pub fn get_components_list<'a>(&'a self) -> &'a [Component] {
        &self.temperatures[..]
    }
}

fn get_all_data(file_path: &str) -> String {
    let mut file = File::open(file_path).unwrap();
    let mut data = String::new();

    file.read_to_string(&mut data).unwrap();
    data
}

fn update_time_and_memory(entry: &mut Process, parts: &[&str], page_size_kb: u64) {
    //entry.name = parts[1][1..].to_owned();
    //entry.name.pop();
    // we get the rss then we add the vsize
    entry.memory = u64::from_str(parts[23]).unwrap() * page_size_kb +
                   u64::from_str(parts[22]).unwrap() / 1024;
    set_time(entry,
             u64::from_str(parts[13]).unwrap(),
             u64::from_str(parts[14]).unwrap());
}

fn _get_process_data(path: &Path, proc_list: &mut HashMap<usize, Process>, page_size_kb: u64) {
    if !path.exists() || !path.is_dir() {
        return;
    }
    let paths : Vec<&str> = path.as_os_str().to_str().unwrap().split("/").collect();
    let last = paths[paths.len() - 1];
    match i64::from_str(last) {
        Ok(nb) => {
            let mut tmp = PathBuf::from(path);

            tmp.push("stat");
            let data = get_all_data(tmp.to_str().unwrap());

            // The stat file is "interesting" to parse, because spaces cannot
            // be used as delimiters. The second field stores the command name
            // sourrounded by parentheses. Unfortunately, whitespace and
            // parentheses are legal parts of the command, so parsing has to
            // proceed like this: The first field is delimited by the first
            // whitespace, the second field is everything until the last ')'
            // in the entire string. All other fields are delimited by
            // whitespace.

            let mut parts = Vec::new();
            let mut data_it = data.splitn(2, " ");
            parts.push(data_it.next().unwrap());
            // The following loses the ) from the input, but that's ok because
            // we're not using it anyway.
            let mut data_it = data_it.next().unwrap().rsplitn(2, ")");
            let data = data_it.next().unwrap();
            parts.push(data_it.next().unwrap());
            parts.extend(data.split_whitespace());
            if let Some(ref mut entry) = proc_list.get_mut(&(nb as usize)) {
                update_time_and_memory(entry, &parts, page_size_kb);
                return;
            }

            let parent = match i64::from_str(parts[3]).unwrap() {
                0 => None,
                p => Some(p)
            };

            let mut p = Process::new(nb,
                                     parent,
                                     u64::from_str(parts[21]).unwrap() /
                                     unsafe { sysconf(_SC_CLK_TCK) } as u64);

            tmp = PathBuf::from(path);
            tmp.push("cmdline");
            p.cmd = copy_from_file(&tmp);
            p.name = p.cmd[0].split("/").last().unwrap().to_owned();
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

            update_time_and_memory(&mut p, &parts, page_size_kb);
            proc_list.insert(nb as usize, p);
        }
        _ => {}
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

fn realpath(original: &Path) -> PathBuf {
    let ori = Path::new(original.to_str().unwrap());

    // Right now lstat on windows doesn't work quite well
    if cfg!(windows) {
        return PathBuf::from(ori);
    }
    let result = PathBuf::from(original);
    let mut buf: stat = unsafe { ::std::mem::uninitialized() };
    let res = unsafe { lstat(result.to_str().unwrap().as_ptr() as *const c_char, &mut buf as *mut stat) };
    if res < 0 || (buf.st_mode & S_IFMT) != S_IFLNK {
        PathBuf::new()
    } else {
        match fs::read_link(&result) {
            Ok(f) => f,
        Err(_) => PathBuf::new(),
        }
    }
}
