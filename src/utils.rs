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

#[cfg(any(target_os = "macos", unix))]
pub mod users {
    use crate::User;
    use libc::{c_char, getgrgid, getpwent, gid_t, strlen, setpwent, endpwent, getgrouplist};

    fn cstr_to_rust(c: *const c_char) -> Option<String> {
        let mut s = Vec::new();
        let mut i = 0;
        loop {
            let value = unsafe { *c.offset(i) } as u8;
            if value == 0 {
                break
            }
            s.push(value);
            i += 1;
        }
        String::from_utf8(s).ok()
    }

    fn get_user_groups(name: *const c_char, group_id: gid_t) -> Vec<String> {
        let mut add = 0;

        loop {
            let mut nb_groups = 256 + add;
            let mut groups = Vec::with_capacity(nb_groups as _);
            if unsafe { getgrouplist(name, group_id, groups.as_mut_ptr(), &mut nb_groups) } == -1 {
                add += 100;
                continue;
            }
            unsafe { groups.set_len(nb_groups as _); }
            return groups.into_iter().filter_map(|g| {
                let group = unsafe { getgrgid(g as _) };
                if group.is_null() {
                    return None;
                }
                cstr_to_rust(unsafe { (*group).gr_name })
            }).collect();
        }
    }

    pub fn endswith(s1: *const c_char, s2: &[u8]) -> bool {
        if s1.is_null() {
            return false;
        }
        let mut len = unsafe { strlen(s1) } as isize - 1;
        let mut i = s2.len() as isize - 1;
        while len >= 0 && i >= 0 && unsafe { *s1.offset(len) } == s2[i as usize] as _ {
            i -= 1;
            len -= 1;
        }
        i == -1
    }

    pub fn get_users_list<F>(filter: F) -> Vec<User>
        where F: Fn(*const c_char) -> bool
    {
        let mut users = Vec::new();

        unsafe { setpwent() };
        loop {
            let pw = unsafe { getpwent() };
            if pw.is_null() {
                break;
            }
            if !filter(unsafe { (*pw).pw_shell }) {
                // This is not a "real" user.
                continue;
            }
            let groups = get_user_groups(unsafe { (*pw).pw_name }, unsafe { (*pw).pw_gid });
            if let Some(name) = cstr_to_rust(unsafe { (*pw).pw_name }) {
                users.push(User {
                    name,
                    groups,
                });
            }
        }
        unsafe { endpwent() };
        users.sort_unstable_by(|x, y| x.name.partial_cmp(&y.name).unwrap());
        users.dedup_by(|a, b| a.name == b.name);
        users
    }
}
