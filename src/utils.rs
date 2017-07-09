// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

#[cfg(not(target_os = "windows"))]
use std::fs;
#[cfg(not(target_os = "windows"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_os = "windows"))]
use std::ffi::OsStr;
#[cfg(not(target_os = "windows"))]
use std::os::unix::ffi::OsStrExt;
#[cfg(not(target_os = "windows"))]
use libc::{c_char, lstat, stat, S_IFLNK, S_IFMT, pid_t};

#[cfg(not(target_os = "windows"))]
pub fn realpath(original: &Path) -> PathBuf {
    let ori = Path::new(original.to_str().unwrap());

    // Right now lstat on windows doesn't work quite well
    if cfg!(windows) {
        return PathBuf::from(ori);
    }
    let result = PathBuf::from(original);
    let mut result_s = result.to_str().unwrap().as_bytes().to_vec();
    result_s.push(0);
    let mut buf: stat = unsafe { ::std::mem::uninitialized() };
    let res = unsafe { lstat(result_s.as_ptr() as *const c_char,
                             &mut buf as *mut stat) };
    if res < 0 || (buf.st_mode & S_IFMT) != S_IFLNK {
        PathBuf::new()
    } else {
        match fs::read_link(&result) {
            Ok(f) => f,
            Err(_) => PathBuf::new(),
        }
    }
}

/* convert a path to a NUL-terminated Vec<u8> suitable for use with C functions */
#[cfg(not(target_os = "windows"))]
pub fn to_cpath(path: &Path) -> Vec<u8>
{
    let path_os: &OsStr = path.as_ref();
    let mut cpath = path_os.as_bytes().to_vec();
    cpath.push(0);
    cpath
}

/// Returns the pid for the current process.
#[cfg(not(target_os = "windows"))]
pub fn get_current_pid() -> pid_t {
    extern "C" {
        fn getpid() -> pid_t;
    }

    unsafe { getpid() }
}
