// Take a look at the license at the top of the repository in the LICENSE file.

use winapi::shared::minwindef::FILETIME;
use winapi::shared::minwindef::{DWORD, HKEY};
use winapi::shared::winerror;
use winapi::um::winnt::{KEY_READ, LPWSTR};
use winapi::um::winreg::{RegCloseKey, RegOpenKeyExW, RegQueryValueExW};

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::time::SystemTime;

#[inline]
pub(crate) fn filetime_to_u64(f: FILETIME) -> u64 {
    (f.dwHighDateTime as u64) << 32 | (f.dwLowDateTime as u64)
}

#[inline]
pub(crate) fn get_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|n| n.as_secs())
        .unwrap_or(0)
}

pub(crate) unsafe fn to_str(p: LPWSTR) -> String {
    let mut i = 0;

    loop {
        let c = *p.offset(i);
        if c == 0 {
            break;
        }
        i += 1;
    }
    let s = std::slice::from_raw_parts(p, i as _);
    String::from_utf16(s).unwrap_or_else(|_e| {
        sysinfo_debug!("Failed to convert to UTF-16 string: {}", _e);
        String::new()
    })
}

fn utf16_str<S: AsRef<OsStr> + ?Sized>(text: &S) -> Vec<u16> {
    OsStr::new(text)
        .encode_wide()
        .chain(Some(0).into_iter())
        .collect::<Vec<_>>()
}

struct RegKey(HKEY);

impl RegKey {
    unsafe fn open(hkey: HKEY, path: &[u16]) -> Option<Self> {
        let mut new_hkey: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(hkey, path.as_ptr(), 0, KEY_READ, &mut new_hkey) != 0 {
            return None;
        }
        Some(Self(new_hkey))
    }

    unsafe fn get_value(&self, field_name: &[u16], buf: &mut [u8], buf_len: &mut DWORD) -> DWORD {
        let mut buf_type: DWORD = 0;

        RegQueryValueExW(
            self.0,
            field_name.as_ptr(),
            std::ptr::null_mut(),
            &mut buf_type,
            buf.as_mut_ptr() as _,
            buf_len,
        ) as DWORD
    }
}

impl Drop for RegKey {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}

pub(crate) fn get_reg_string_value(hkey: HKEY, path: &str, field_name: &str) -> Option<String> {
    let c_path = utf16_str(path);
    let c_field_name = utf16_str(field_name);

    unsafe {
        let new_key = RegKey::open(hkey, &c_path)?;
        let mut buf_len: DWORD = 2048;
        let mut buf: Vec<u8> = Vec::with_capacity(buf_len as usize);

        loop {
            match new_key.get_value(&c_field_name, &mut buf, &mut buf_len) {
                winerror::ERROR_SUCCESS => break,
                winerror::ERROR_MORE_DATA => {
                    buf.reserve(buf_len as _);
                }
                _ => return None,
            }
        }

        buf.set_len(buf_len as _);

        let words = std::slice::from_raw_parts(buf.as_ptr() as *const u16, buf.len() / 2);
        let mut s = String::from_utf16_lossy(words);
        while s.ends_with('\u{0}') {
            s.pop();
        }
        Some(s)
    }
}

pub(crate) fn get_reg_value_u32(hkey: HKEY, path: &str, field_name: &str) -> Option<[u8; 4]> {
    let c_path = utf16_str(path);
    let c_field_name = utf16_str(field_name);

    unsafe {
        let new_key = RegKey::open(hkey, &c_path)?;
        let mut buf_len: DWORD = 4;
        let mut buf = [0u8; 4];

        match new_key.get_value(&c_field_name, &mut buf, &mut buf_len) {
            winerror::ERROR_SUCCESS => Some(buf),
            _ => None,
        }
    }
}
