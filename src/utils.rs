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
use libc::{c_char, lstat, stat, S_IFLNK, S_IFMT};

#[cfg(not(target_os = "windows"))]
pub fn realpath(original: &Path) -> PathBuf {
    let ori = Path::new(original.to_str().unwrap());

    // Right now lstat on windows doesn't work quite well
    if cfg!(windows) {
        return PathBuf::from(ori);
    }
    let result = PathBuf::from(original);
    let mut buf: stat = unsafe { ::std::mem::uninitialized() };
    let res = unsafe { lstat(result.to_str().unwrap().as_ptr() as *const c_char,
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
