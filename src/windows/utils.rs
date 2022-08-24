// Take a look at the license at the top of the repository in the LICENSE file.

use winapi::shared::minwindef::FILETIME;
use winapi::um::winnt::LPWSTR;

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
