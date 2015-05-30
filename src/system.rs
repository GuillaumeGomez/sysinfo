// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use processus::*;
use process::*;
use std::fs::{File, read_link, PathExt};
use std::io::Read;
use std::str::FromStr;
use std::path::{Path, PathBuf};
use std::collections::VecMap;
use std::fs;
use libc::types::os::arch::posix01::stat;
use libc::c_char;
use libc::funcs::posix01::stat_::lstat;
use libc::consts::os::posix88::{S_IFLNK, S_IFMT};
use std::path::Component::Normal;

pub struct System {
    processus_list: VecMap<Processus>,
    mem_total: u64,
    mem_used: u64,
    swap_total: u64,
    swap_free: u64,
    processes: Vec<Process>,
}

impl System {
    pub fn new() -> System {
        let mut s = System {
            processus_list: VecMap::new(),
            mem_total: 0,
            mem_used: 0,
            swap_total: 0,
            swap_free: 0,
            processes: Vec::new(),
        };

        s.refresh();
        s
    }

    pub fn refresh(&mut self) {
        let mut proc_list : VecMap<Processus> = VecMap::new();

        match fs::read_dir(&Path::new("/proc")) {
            Ok(d) => {
                for entry in d {
                    if !entry.is_ok() {
                        continue;
                    }
                    let entry = entry.unwrap();
                    let entry = entry.path();

                    if entry.is_dir() {
                        match _get_processus_data(entry.as_path(), &self.processus_list) {
                            Some(p) => {
                                proc_list.insert(p.pid as usize, p);
                            }
                            None => {}
                        };
                    } else {
                        match entry.to_str().unwrap() {
                            "/proc/meminfo" => {
                                let data = get_all_data(entry.to_str().unwrap());
                                let lines : Vec<&str> = data.split('\n').collect();
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
                            },
                            "/proc/stat" => {
                                let data = get_all_data(entry.to_str().unwrap());
                                let lines : Vec<&str> = data.split('\n').collect();
                                let mut i = 0;
                                let first = self.processes.len() == 0;
                                for line in lines.iter() {
                                    if !line.starts_with("cpu") {
                                        break;
                                    }

                                    let (parts, _) : (Vec<&str>, Vec<&str>) = line.split(' ').partition(|s| s.len() > 0);
                                    if first {
                                        self.processes.push(new_process(parts[0], u64::from_str(parts[1]).unwrap(),
                                            u64::from_str(parts[2]).unwrap(),
                                            u64::from_str(parts[3]).unwrap(),
                                            u64::from_str(parts[4]).unwrap()));
                                    } else {
                                        set_process(self.processes.get_mut(i).unwrap(),
                                            u64::from_str(parts[1]).unwrap(),
                                            u64::from_str(parts[2]).unwrap(),
                                            u64::from_str(parts[3]).unwrap(),
                                            u64::from_str(parts[4]).unwrap());
                                        i += 1;
                                    }
                                }
                            }
                           _ => {},
                        }
                    }
                }
                for (pid, proc_) in proc_list.iter_mut() {
                    match self.processus_list.get(&pid) {
                        Some(p) => {
                            let (new, old) = get_raw_times(&self.processes[0]);
                            let (pnew, pold) = get_raw_processus_times(&p);
                            compute_cpu_usage(proc_, pnew, pold, new, old);
                        }
                        None => {}
                    }
                }
                self.processus_list = proc_list;
            }
            Err(_) => {}
        }
    }

    pub fn get_processus_list<'a>(&'a self) -> &'a VecMap<Processus> {
        &self.processus_list
    }

    pub fn get_processus(&self, pid: i64) -> Option<&Processus> {
        self.processus_list.get(&(pid as usize))
    }

    /// The first process in the array is the "main" process
    pub fn get_process_list<'a>(&'a self) -> &'a [Process] {
        self.processes.as_slice()
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
}

fn get_all_data(file_path: &str) -> String {
    let mut file = File::open(file_path).unwrap();
    let mut data = String::new();

    file.read_to_string(&mut data);
    data
}

fn _get_processus_data(path: &Path, proc_list: &VecMap<Processus>) -> Option<Processus> {
    if !path.exists() || !path.is_dir() {
        return None;
    }
    let paths : Vec<&str> = path.as_os_str().to_str().unwrap().split("/").collect();
    let last = paths[paths.len() - 1];
    match i64::from_str(last) {
        Ok(nb) => {
            let mut p = Processus::new(nb);
            let mut tmp = PathBuf::from(path);

            tmp.push("cmdline");
            p.cmd = String::from_str(copy_from_file(&tmp)[0].as_ref());
            {
                let tmp_line : Vec<&str> = p.cmd.split(" ").collect();
                let tmp_name : Vec<&str> = tmp_line[0].split("/").collect();

                p.name = String::from_str(tmp_name[tmp_name.len() - 1]);
            }
            tmp = PathBuf::from(path);
            tmp.push("environ");
            p.environ = copy_from_file(&tmp);
            tmp = PathBuf::from(path);
            tmp.push("exe");

            let s = read_link(tmp.to_str().unwrap());

            if s.is_ok() {
                p.exe = String::from_str(s.unwrap().to_str().unwrap());
            }
            tmp = PathBuf::from(path);
            tmp.push("cwd");
            p.cwd = String::from_str(realpath(Path::new(tmp.to_str().unwrap())).to_str().unwrap());
            tmp = PathBuf::from(path);
            tmp.push("root");
            p.root = String::from_str(realpath(Path::new(tmp.to_str().unwrap())).to_str().unwrap());
            tmp = PathBuf::from(path);
            tmp.push("status");
            let data = get_all_data(tmp.to_str().unwrap());
            let lines : Vec<&str> = data.split('\n').collect();

            for line in lines.iter() {
                match *line {
                    l if l.starts_with("VmRSS") => {
                        let parts : Vec<&str> = line.split(' ').collect();

                        p.memory = u64::from_str(parts[parts.len() - 2]).unwrap();
                        break;
                    }
                    _ => continue,
                }
            }
            tmp = PathBuf::from(path);
            tmp.push("stat");
            let data = get_all_data(tmp.to_str().unwrap());
            // ne marche pas car il faut aussi les anciennes valeurs !!!!
            let (parts, _) : (Vec<&str>, Vec<&str>) = data.split(' ').partition(|s| s.len() > 0);
            set_time(&mut p, u64::from_str(parts[13]).unwrap(), u64::from_str(parts[14]).unwrap());
                //u64::from_str(parts[15]).unwrap(), u64::from_str(parts[16]).unwrap());
            Some(p)
        }
        _ => None
    }
}

fn copy_from_file(entry: &Path) -> Vec<String> {
    match File::open(entry.to_str().unwrap()) {
        Ok(mut f) => {
            let mut d = String::new();

            f.read_to_string(&mut d);
            let v : Vec<&str> = d.split('\0').collect();
            let mut ret = Vec::new();

            for tmp in v.iter() {
                ret.push(String::from_str(tmp));
            }
            ret
        },
        Err(_) => Vec::new()
    }
}

fn old_realpath(ori: &Path) -> PathBuf {
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

fn realpath(original: &Path) -> PathBuf {
    let old = Path::new(original.to_str().unwrap());

    old_realpath(&old)
}
