// Take a look at the license at the top of the repository in the LICENSE file.

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{self, FILETIME};
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, KEY_READ, REG_NONE,
};

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

pub(crate) unsafe fn to_str(p: PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }

    p.to_string().unwrap_or_else(|_e| {
        sysinfo_debug!("Failed to convert to UTF-16 string: {}", _e);
        String::new()
    })
}

fn utf16_str<S: AsRef<OsStr> + ?Sized>(text: &S) -> Vec<u16> {
    OsStr::new(text)
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>()
}

struct RegKey(HKEY);

impl RegKey {
    unsafe fn open(hkey: HKEY, path: &[u16]) -> Option<Self> {
        let mut new_hkey = Default::default();
        if RegOpenKeyExW(
            hkey,
            PCWSTR::from_raw(path.as_ptr()),
            0,
            KEY_READ,
            &mut new_hkey,
        )
        .is_err()
        {
            return None;
        }
        Some(Self(new_hkey))
    }

    unsafe fn get_value(
        &self,
        field_name: &[u16],
        buf: &mut [u8],
        buf_len: &mut u32,
    ) -> windows::core::Result<()> {
        let mut buf_type = REG_NONE;

        RegQueryValueExW(
            self.0,
            PCWSTR::from_raw(field_name.as_ptr()),
            None,
            Some(&mut buf_type),
            Some(buf.as_mut_ptr()),
            Some(buf_len),
        )
    }
}

impl Drop for RegKey {
    fn drop(&mut self) {
        let _err = unsafe { RegCloseKey(self.0) };
    }
}

pub(crate) fn get_reg_string_value(hkey: HKEY, path: &str, field_name: &str) -> Option<String> {
    let c_path = utf16_str(path);
    let c_field_name = utf16_str(field_name);

    unsafe {
        let new_key = RegKey::open(hkey, &c_path)?;
        let mut buf_len: u32 = 2048;
        let mut buf: Vec<u8> = Vec::with_capacity(buf_len as usize);

        loop {
            match new_key.get_value(&c_field_name, &mut buf, &mut buf_len) {
                Ok(()) => break,
                Err(err) if err.code() == Foundation::ERROR_MORE_DATA.to_hresult() => {
                    // Needs to be updated for `Vec::reserve` to actually add additional capacity.
                    buf.set_len(buf.capacity());
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
        let mut buf_len: u32 = 4;
        let mut buf = [0u8; 4];

        new_key
            .get_value(&c_field_name, &mut buf, &mut buf_len)
            .map(|_| buf)
            .ok()
    }
}
