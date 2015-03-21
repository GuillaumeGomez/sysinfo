// license info

/*!

*/

use processus::*;
use process::*;
use std::old_io::fs;
use std::fs::{File, read_link, PathExt};
use std::io::Read;
use std::ffi::AsOsStr;
use std::old_io::fs::PathExtensions;
use std::old_path::posix::Path as PosixPath;
use std::str::FromStr;
use std::os;
use std::path::{Path, PathBuf};
use std::io;
use std::old_path;
use std::old_io;
use std::fmt::{self, Formatter, Debug};
use std::collections::VecMap;

pub struct System {
    processus_list: VecMap<Processus>,
    mem_total: u64,
    mem_free: u64,
    swap_total: u64,
    swap_free: u64,
    processes: Vec<Process>,
}

impl System {
    pub fn new() -> System {
        let mut s = System {
            processus_list: VecMap::new(),
            mem_total: 0,
            mem_free: 0,
            swap_total: 0,
            swap_free: 0,
            processes: Vec::new(),
        };

        s.refresh();
        s
    }

    pub fn refresh(&mut self) {
        match fs::readdir(&PosixPath::new("/proc")) {
            Ok(v) => {
                let mut proc_list : VecMap<Processus> = VecMap::new();

                for entry in v.iter() {
                    if entry.is_dir() {
                        match _get_processus_data(entry, &self.processus_list) {
                            Some(p) => {
                                proc_list.insert(p.pid as usize, p);
                            }
                            None => {}
                        };
                    } else {
                        match entry.as_os_str().to_str().unwrap() {
                            "/proc/meminfo" => {
                                let mut data = get_all_data(entry.as_str().unwrap());
                                let lines : Vec<&str> = data.split('\n').collect();
                                for line in lines.iter() {
                                    match *line {
                                        l if l.starts_with("MemTotal:") => {
                                            let parts : Vec<&str> = line.split(' ').collect();

                                            self.mem_total = u64::from_str(parts[parts.len() - 2]).unwrap();
                                        },
                                        l if l.starts_with("MemFree:") => {
                                            let parts : Vec<&str> = line.split(' ').collect();

                                            self.mem_free = u64::from_str(parts[parts.len() - 2]).unwrap();
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
                                let mut data = get_all_data(entry.as_str().unwrap());
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
            },
            Err(e) => {
                panic!("cannot read /proc ! error: {}", e);
            }
        }
    }

    pub fn get_processus_list<'a>(&'a self) -> &'a VecMap<Processus> {
        &self.processus_list
    }

    pub fn get_processus(&self, pid: i64) -> Option<&Processus> {
        self.processus_list.get(&(pid as usize))
    }

    // The first process in the array is the "main" process
    pub fn get_process_list<'a>(&'a self) -> &'a [Process] {
        self.processes.as_slice()
    }

    pub fn get_total_memory(&self) -> u64 {
        self.mem_total
    }

    pub fn get_free_memory(&self) -> u64 {
        self.mem_free
    }

    pub fn get_total_swap(&self) -> u64 {
        self.swap_total
    }

    pub fn get_free_swap(&self) -> u64 {
        self.swap_free
    }
}

fn get_all_data(file_path: &str) -> String {
    let mut file = File::open(file_path).unwrap();
    let mut data = String::new();

    file.read_to_string(&mut data);
    data
}

fn _get_processus_data(path: &PosixPath, proc_list: &VecMap<Processus>) -> Option<Processus> {
    if !path.exists() || !path.is_dir() {
        return None;
    }
    let paths : Vec<&str> = path.as_os_str().to_str().unwrap().split("/").collect();
    let last = paths[paths.len() - 1];
    match i64::from_str(last) {
        Ok(nb) => {
            let mut p = Processus::new(nb);
            let mut tmp = path.clone();
            tmp.push("cmdline");
            p.cmd = String::from_str(copy_from_file(&tmp)[0].as_slice());
            tmp = path.clone();
            tmp.push("environ");
            p.environ = copy_from_file(&tmp);
            tmp = path.clone();
            tmp.push("exe");
            let s = read_link(tmp.as_str().unwrap());
            if s.is_ok() {
                p.exe = String::from_str(s.unwrap().as_os_str().to_str().unwrap());
            }
            tmp = path.clone();
            tmp.push("cwd");
            p.cwd = String::from_str(realpath(Path::new(tmp.as_str().unwrap())).unwrap().as_os_str().to_str().unwrap());
            tmp = path.clone();
            tmp.push("root");
            p.root = String::from_str(realpath(Path::new(tmp.as_str().unwrap())).unwrap().as_os_str().to_str().unwrap());
            tmp = path.clone();
            tmp.push("status");
            let mut data = get_all_data(tmp.as_str().unwrap());

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
            tmp = path.clone();
            tmp.push("stat");
            let mut data2 = get_all_data(tmp.as_str().unwrap());
            // ne marche pas car il faut aussi les anciennes valeurs !!!!
            let (parts, _) : (Vec<&str>, Vec<&str>) = data2.split(' ').partition(|s| s.len() > 0);
            set_time(&mut p, u64::from_str(parts[13]).unwrap(), u64::from_str(parts[14]).unwrap());
            Some(p)
        }
        _ => None
    }
}

fn get_last(path: &PosixPath) -> &str {
    let t = path.as_os_str().to_str().unwrap();
    let v : Vec<&str> = t.split('/').collect();

    if v.len() > 0 {
        v[v.len() - 1]
    } else {
        ""
    }
}

fn copy_from_file(entry: &PosixPath) -> Vec<String> {
    match File::open(entry) {
        Ok(mut f) => {
            let mut d = String::new();

            f.read_to_string(&mut d);
            let mut v : Vec<&str> = d.split('\0').collect();
            let mut ret = Vec::new();

            for tmp in v.iter() {
                ret.push(String::from_str(tmp));
            }
            ret
        },
        Err(_) => Vec::new()
    }
}

fn old_realpath(original: &old_path::Path) -> old_io::IoResult<old_path::Path> {
    const MAX_LINKS_FOLLOWED: usize = 256;
    let original = try!(os::getcwd()).join(original);
    // Right now lstat on windows doesn't work quite well
    if cfg!(windows) {
        return Ok(original)
    }
    let result = original.root_path();
    let mut result = result.expect("make_absolute has no root_path");
    let mut followed = 0;
    for part in original.components() {
        result.push(part);
        loop {
            if followed == MAX_LINKS_FOLLOWED {
                return Ok(old_path::Path::new(""));
            }
            match fs::lstat(&result) {
                Err(..) => break,
                Ok(ref stat) if stat.kind != old_io::FileType::Symlink => break,
                Ok(..) => {
                    followed += 1;
                    let path = try!(fs::readlink(&result));
                    result.pop();
                    result.push(path);
                }
            }
        }
    }
    return Ok(result);
}

fn realpath(original: &Path) -> io::Result<PathBuf> {
    let old = old_path::Path::new(original.to_str().unwrap());

    match old_realpath(&old) {
        Ok(p) => Ok(PathBuf::new(p.as_str().unwrap())),
        Err(_) => Ok(PathBuf::new(""))
    }
}
