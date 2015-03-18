use std::fmt::{self, Formatter, Debug};
use libc::{c_int};

pub struct Processus {
    pub cmd: String, // command line
    pub exe: String, // path to the executable
    pub pid: i32, // pid of the processus
    pub environ: Vec<String>, // environment of the processus
    pub cwd: String, // current working directory
    pub root: String, // path of the root directory
    pub memory: u32 // memory usage
}

impl Processus {
    pub fn kill(&self, signal: ::Signal) -> bool {
        unsafe { ::ffi::kill(self.pid as c_int, signal as c_int) == 0 }
    }
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
        write!(f, "memory usage: {} kB\n", self.memory);
        write!(f, "root path: {}", self.root)
    }
}