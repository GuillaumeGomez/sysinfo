//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

#[cfg(not(any(target_os = "windows", target_os = "unknown", target_arch = "wasm32")))]
use libc::{c_char, lstat, stat, S_IFLNK, S_IFMT};
#[cfg(not(any(target_os = "windows", target_os = "unknown", target_arch = "wasm32")))]
use std::ffi::OsStr;
#[cfg(not(any(target_os = "windows", target_os = "unknown", target_arch = "wasm32")))]
use std::fs;
#[cfg(not(any(target_os = "windows", target_os = "unknown", target_arch = "wasm32")))]
use std::os::unix::ffi::OsStrExt;
#[cfg(not(any(target_os = "windows", target_os = "unknown", target_arch = "wasm32")))]
use std::path::{Path, PathBuf};
use Pid;

#[cfg(not(any(target_os = "windows", target_os = "unknown", target_arch = "wasm32")))]
pub fn realpath(original: &Path) -> PathBuf {
    use std::mem::MaybeUninit;

    fn and(x: u32, y: u32) -> u32 {
        x & y
    }

    #[allow(clippy::identity_conversion)]
    if let Some(original_str) = original.to_str() {
        let ori = Path::new(original_str);

        // Right now lstat on windows doesn't work quite well
        if cfg!(windows) {
            return PathBuf::from(ori);
        }
        let result = PathBuf::from(original);
        let mut result_s = result.to_str().unwrap_or("").as_bytes().to_vec();
        result_s.push(0);
        let mut buf = MaybeUninit::<stat>::uninit();
        let res = unsafe { lstat(result_s.as_ptr() as *const c_char, buf.as_mut_ptr()) };
        let buf = unsafe { buf.assume_init() };
        if res < 0 || and(buf.st_mode.into(), S_IFMT.into()) != S_IFLNK.into() {
            PathBuf::new()
        } else {
            match fs::read_link(&result) {
                Ok(f) => f,
                Err(_) => PathBuf::new(),
            }
        }
    } else {
        PathBuf::new()
    }
}

/* convert a path to a NUL-terminated Vec<u8> suitable for use with C functions */
#[cfg(not(any(target_os = "windows", target_os = "unknown", target_arch = "wasm32")))]
pub fn to_cpath(path: &Path) -> Vec<u8> {
    let path_os: &OsStr = path.as_ref();
    let mut cpath = path_os.as_bytes().to_vec();
    cpath.push(0);
    cpath
}

/// Returns the pid for the current process.
///
/// `Err` is returned in case the platform isn't supported.
///
/// ```no_run
/// use sysinfo::get_current_pid;
///
/// match get_current_pid() {
///     Ok(pid) => {
///         println!("current pid: {}", pid);
///     }
///     Err(e) => {
///         eprintln!("failed to get current pid: {}", e);
///     }
/// }
/// ```
pub fn get_current_pid() -> Result<Pid, &'static str> {
    cfg_if! {
        if #[cfg(not(any(target_os = "windows", target_os = "unknown", target_arch = "wasm32")))] {
            fn inner() -> Result<Pid, &'static str> {
                unsafe { Ok(::libc::getpid()) }
            }
        } else if #[cfg(target_os = "windows")] {
            fn inner() -> Result<Pid, &'static str> {
                use winapi::um::processthreadsapi::GetCurrentProcessId;

                unsafe { Ok(GetCurrentProcessId() as Pid) }
            }
        } else if #[cfg(target_os = "unknown")] {
            fn inner() -> Result<Pid, &'static str> {
                Err("Unavailable on this platform")
            }
        } else {
            fn inner() -> Result<Pid, &'static str> {
                Err("Unknown platform")
            }
        }
    }
    inner()
}
