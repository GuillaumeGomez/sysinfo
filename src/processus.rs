use std::old_io::fs;
use std::fs::{File, read_link, PathExt};
use std::io::Read;
use std::ffi::AsOsStr;
use std::old_io::fs::PathExtensions;
use std::old_path::posix::Path as PosixPath;
use std::str::FromStr;
use std::fmt::{self, Formatter, Debug};
use std::os;
use std::path::{Path, PathBuf};
use std::io;
use std::old_path;
use std::old_io;

pub struct Processus {
    pub cmd: String, // command line
    pub exe: String, // path to the executable
    pub pid: i32,
    pub environ: Vec<String>,
    pub cwd: String, // current working directory
    pub root: String // path of the root directory
}

impl Debug for Processus {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "pid: {}\n", self.pid);
        write!(f, "environment:");
        for var in self.environ.iter() {
            write!(f, "\n\t{}", var);
        }
        write!(f, "command: {}\n", self.cmd);
        write!(f, "executable path: {}\n", self.exe);
        write!(f, "current working directory: {}\n", self.cwd);
        write!(f, "root path: {}", self.root)
    }
}