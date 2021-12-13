// Take a look at the license at the top of the repository in the LICENSE file.

use winapi::shared::minwindef::FILETIME;

use std::time::SystemTime;

#[inline]
pub fn filetime_to_u64(f: FILETIME) -> u64 {
    (f.dwHighDateTime as u64) << 32 | (f.dwLowDateTime as u64)
}

#[inline]
pub fn get_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|n| n.as_secs())
        .unwrap_or(0)
}
