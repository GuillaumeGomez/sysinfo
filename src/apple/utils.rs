// Take a look at the license at the top of the repository in the LICENSE file.

use libc::c_char;

pub(crate) fn cstr_to_rust(c: *const c_char) -> Option<String> {
    cstr_to_rust_with_size(c, None)
}

pub(crate) fn cstr_to_rust_with_size(c: *const c_char, size: Option<usize>) -> Option<String> {
    if c.is_null() {
        return None;
    }
    let mut s = match size {
        Some(len) => Vec::with_capacity(len),
        None => Vec::new(),
    };
    let mut i = 0;
    unsafe {
        loop {
            let value = *c.offset(i) as u8;
            if value == 0 {
                break;
            }
            s.push(value);
            i += 1;
        }
        String::from_utf8(s).ok()
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn vec_to_rust(buf: Vec<i8>) -> Option<String> {
    String::from_utf8(
        buf.into_iter()
            .flat_map(|b| if b > 0 { Some(b as u8) } else { None })
            .collect(),
    )
    .ok()
}
