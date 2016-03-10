// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use Component;
use processor::*;
use process::*;
use std::fs::{File, read_link};
use std::io::Read;
use std::str::FromStr;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use libc::{stat, lstat, c_char, sysconf, _SC_PAGESIZE, S_IFLNK, S_IFMT};
use std::path::Component::Normal;

pub struct System {
    process_list: HashMap<usize, Process>,
    mem_total: u64,
    mem_used: u64,
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
            mem_used: 0,
            swap_total: 0,
            swap_free: 0,
            processors: Vec::new(),
            page_size_kb: unsafe { sysconf(_SC_PAGESIZE) as u64 / 1024 },
            temperatures: Vec::new(),
        };
        s.refresh_all();
        s
    }

    pub fn refresh_system(&mut self) {
        let data = get_all_data("/proc/meminfo");
        let lines : Vec<&str> = data.split('\n').collect();

        for component in self.temperatures.iter_mut() {
            component.update();
        }
        for line in lines.iter() {
            match *line {
                l if l.starts_with("MemTotal:") => {
                    let parts : Vec<&str> = line.split(' ').collect();

                    self.mem_total = u64::from_str(parts[parts.len() - 2]).unwrap();
                },
                l if l.starts_with("MemFree:") => {
                    let parts : Vec<&str> = line.split(' ').collect();

                    self.mem_used = u64::from_str(parts[parts.len() - 2]).unwrap();
                },
                l if l.starts_with("SwapTotal:") => {
                    let parts : Vec<&str> = line.split(' ').collect();

                    self.swap_total = u64::from_str(parts[parts.len() - 2]).unwrap();
                },
                l if l.starts_with("SwapFree:") => {
                    let parts : Vec<&str> = line.split(' ').collect();

                    self.swap_free = u64::from_str(parts[parts.len() - 2]).unwrap();
                },
                _ => continue,
            }
        }
        let data = get_all_data("/proc/stat");
        let lines : Vec<&str> = data.split('\n').collect();
        let mut i = 0;
        let first = self.processors.len() == 0;
        for line in lines.iter() {
            if !line.starts_with("cpu") {
                break;
            }

            let (parts, _) : (Vec<&str>, Vec<&str>) = line.split(' ').partition(|s| s.len() > 0);
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
                    } else {
                        match entry.to_str().unwrap() {
                           _ => {},
                        }
                    }
                }
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
            Err(_) => {}
        }
    }

    pub fn refresh_all(&mut self) {
        self.refresh_system();
        self.refresh_process();
    }

    pub fn get_process_list<'a>(&'a self) -> &'a HashMap<usize, Process> {
        &self.process_list
    }

    pub fn get_process(&self, pid: i64) -> Option<&Process> {
        self.process_list.get(&(pid as usize))
    }

    /// The first process in the array is the "main" process
    pub fn get_processor_list<'a>(&'a self) -> &'a [Processor] {
        &self.processors[..]
    }

    pub fn get_total_memory(&self) -> u64 {
        self.mem_total
    }

    pub fn get_used_memory(&self) -> u64 {
        self.mem_used
    }

    pub fn get_total_swap(&self) -> u64 {
        self.swap_total
    }

    // need to be checked
    pub fn get_used_swap(&self) -> u64 {
        self.swap_free
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

fn _get_process_data(path: &Path, proc_list: &mut HashMap<usize, Process>, page_size_kb: u64) {
    if !path.exists() || !path.is_dir() {
        return ;
    }
    let paths : Vec<&str> = path.as_os_str().to_str().unwrap().split("/").collect();
    let last = paths[paths.len() - 1];
    match i64::from_str(last) {
        Ok(nb) => {
            let mut tmp = PathBuf::from(path);

            tmp.push("stat");
            let data = get_all_data(tmp.to_str().unwrap());
            let (parts, _) : (Vec<&str>, Vec<&str>) = data.split(' ').partition(|s| s.len() > 0);
            match proc_list.get(&(nb as usize)) {
                Some(_) => {}
                None => {
                    let mut p = Process::new(nb, u64::from_str(parts[21]).unwrap());

                    tmp = PathBuf::from(path);
                    tmp.push("cmdline");
                    p.cmd = copy_from_file(&tmp)[0].clone();
                    p.name = p.cmd.split("/").last().unwrap().split(":").collect::<Vec<&str>>()[0].to_owned();
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
                    proc_list.insert(nb as usize, p);
                }
            }
            let mut entry = proc_list.get_mut(&(nb as usize)).unwrap();

            //entry.name = parts[1][1..].to_owned();
            //entry.name.pop();
            // we get the rss then we add the vsize
            entry.memory = u64::from_str(parts[23]).unwrap() * page_size_kb + u64::from_str(parts[22]).unwrap() / 1024;
            set_time(&mut entry, u64::from_str(parts[13]).unwrap(),
                u64::from_str(parts[14]).unwrap());
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
            let v : Vec<&str> = d.split('\0').collect();
            let mut ret : Vec<String> = Vec::new();

            for tmp in v.iter() {
                ret.push((*tmp).to_owned());
            }
            ret
        },
        Err(_) => Vec::new()
    }
}

fn realpath(original: &Path) -> PathBuf {
    let ori = Path::new(original.to_str().unwrap());

    const MAX_LINKS_FOLLOWED: usize = 256;

    // Right now lstat on windows doesn't work quite well
    if cfg!(windows) {
        return PathBuf::from(ori);
    }
    let original = ::std::env::current_dir().unwrap().join(ori);
    let mut result = PathBuf::from(original.parent().unwrap());
    let mut followed = 0;
    for part in original.components() {
        match part {
            Normal(s) => {
                result.push(s);
                loop {
                    if followed == MAX_LINKS_FOLLOWED {
                        return PathBuf::new();
                    }
                    let mut buf : stat = unsafe { ::std::mem::uninitialized() };
                    let res = unsafe { lstat(result.to_str().unwrap().as_ptr() as *const c_char, &mut buf as *mut stat) };
                    
                    if res < 0 || (buf.st_mode  & S_IFMT) != S_IFLNK {
                        break;
                    } else {
                        followed += 1;
                        let path = fs::read_link(&result).unwrap();
                        result.pop();
                        result.push(path);
                    }
                }
            }
            _ => {}
        }
    }
    result
}
