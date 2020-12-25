//
// Sysinfo
//
// Copyright (c) 2020 Guillaume Gomez
//

use libc::c_char;

pub fn cstr_to_rust(c: *const c_char) -> Option<String> {
    let mut s = Vec::new();
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
