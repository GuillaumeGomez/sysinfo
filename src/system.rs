// license info

/*!

*/

use processus::*;
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

pub struct System {
    processus_list: Vec<Processus>
}

impl System {
    pub fn new() -> System {
        let mut s = System {
            processus_list: Vec::new()
        };

        s.refresh();
        s
    }

    pub fn refresh(&mut self) {
        match fs::readdir(&PosixPath::new("/proc")) {
            Ok(v) => {
                self.processus_list = Vec::new();

                for entry in v.iter() {
                    if entry.is_dir() {
                        match _get_processus_data(entry) {
                            Some(p) => self.processus_list.push(p),
                            None => {}
                        };
                    }
                }
            },
            Err(e) => {
                panic!("readdir error: {}", e);
            }
        }
    }

    pub fn get_processus_list<'a>(&'a self) -> &'a [Processus] {
        self.processus_list.as_slice()
    }

    pub fn get_processus(&self, pid: i32) -> Option<&Processus> {
        for pro in self.processus_list.iter() {
            if pro.pid == pid {
                return Some(pro);
            }
        }
        None
    }
}

fn _get_processus_data(path: &PosixPath) -> Option<Processus> {
    if !path.exists() || !path.is_dir() {
        return None;
    }
    match fs::readdir(path) {
        Ok(v) => {
            let paths : Vec<&str> = path.as_os_str().to_str().unwrap().split("/").collect();
            let last = paths[paths.len() - 1];
            match i32::from_str(last) {
                Ok(nb) => {
                    let mut p = Processus {
                        pid: nb,
                        cmd: String::new(),
                        environ: Vec::new(),
                        exe: String::new(),
                        cwd: String::new(),
                        root: String::new(),
                        memory: 0
                    };

                    for entry in v.iter() {
                        let t = get_last(entry);

                        match t {
                            "cmdline" => {
                                p.cmd = String::from_str(copy_from_file(entry)[0].as_slice());
                            },
                            "environ" => {
                                p.environ = copy_from_file(entry);
                            },
                            "exe" => {
                                let s = read_link(entry.as_str().unwrap());
                                if s.is_ok() {
                                    p.exe = String::from_str(s.unwrap().as_os_str().to_str().unwrap());
                                }
                            },
                            "cwd" => {
                                p.cwd = String::from_str(realpath(Path::new(entry.as_str().unwrap())).unwrap().as_os_str().to_str().unwrap());
                            },
                            "root" => {
                                p.root = String::from_str(realpath(Path::new(entry.as_str().unwrap())).unwrap().as_os_str().to_str().unwrap());
                            },
                            "status" => {
                                let mut file = File::open(entry.as_str().unwrap()).unwrap();
                                let mut data = String::new();

                                file.read_to_string(&mut data);
                                let lines : Vec<&str> = data.split('\n').collect();
                                for line in lines.iter() {
                                    match *line {
                                        l if l.starts_with("VmSize") => {
                                            let parts : Vec<&str> = line.split(' ').collect();

                                            p.memory = u32::from_str(parts[parts.len() - 2]).unwrap();
                                            break;
                                        }
                                        _ => continue,
                                    }
                                }
                            }
                            l => {}
                        };
                    }
                    Some(p)
                },
                Err(_) => None
            }
        }
        Err(_) => None
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
