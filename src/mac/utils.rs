//
// Sysinfo
//
// Copyright (c) 2020 Guillaume Gomez
//

use libc::c_char;

pub fn cstr_to_rust(c: *const c_char) -> Option<String> {
    cstr_to_rust_with_size(c, None)
}

pub fn cstr_to_rust_with_size(c: *const c_char, size: Option<usize>) -> Option<String> {
    if c.is_null() {
        return None;
    }
    let mut s = match size {
        Some(len) => Vec::with_capacity(len),
        None => Vec::new(),
    };
    let mut i = 0;
    loop {
        let value = unsafe { *c.offset(i) } as u8;
        if value == 0 {
            break;
        }
        s.push(value);
        i += 1;
    }
    String::from_utf8(s).ok()
}
