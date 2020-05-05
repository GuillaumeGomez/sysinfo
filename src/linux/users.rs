//
// Sysinfo
//
// Copyright (c) 2020 Guillaume Gomez
//

use crate::User;

use libc::{getgrgid, getgrouplist};
use std::fs::File;
use std::io::Read;

pub fn get_users_list() -> Vec<User> {
    let mut s = String::new();
    let mut ngroups = 100;
    let mut groups = vec![0; ngroups as usize];

    let _ = File::open("/etc/passwd").and_then(|mut f| f.read_to_string(&mut s));
    s.lines()
        .filter_map(|line| {
            let mut parts = line.split(':');
            if let Some(username) = parts.next() {
                let mut parts = parts.skip(2);
                if let Some(group_id) = parts.next().and_then(|x| u32::from_str_radix(x, 10).ok()) {
                    if let Some(command) = parts.last() {
                        if command.is_empty()
                            || command.ends_with("/false")
                            || command.ends_with("/nologin")
                        {
                            // We don't want "fake" users so in case the user command is "bad", we
                            // ignore this user.
                            return None;
                        }
                        let mut c_user = username.as_bytes().to_vec();
                        c_user.push(0);
                        loop {
                            let mut current = ngroups;
                            if unsafe {
                                getgrouplist(
                                    c_user.as_ptr() as *const _,
                                    group_id,
                                    groups.as_mut_ptr(),
                                    &mut current,
                                )
                            } == -1
                            {
                                if current > ngroups {
                                    for _ in 0..current - ngroups {
                                        groups.push(0);
                                    }
                                    ngroups = current;
                                    continue;
                                }
                                // It really failed, let's move on...
                                return None;
                            }
                            // Let's get all the group names!
                            return Some(User {
                                name: username.to_owned(),
                                groups: groups[..current as usize]
                                    .iter()
                                    .filter_map(|id| {
                                        let g = unsafe { getgrgid(*id as _) };
                                        if g.is_null() {
                                            return None;
                                        }
                                        let mut group_name = Vec::new();
                                        let c_group_name = unsafe { (*g).gr_name };
                                        let mut x = 0;
                                        loop {
                                            let c = unsafe { *c_group_name.offset(x) };
                                            if c == 0 {
                                                break;
                                            }
                                            group_name.push(c as u8);
                                            x += 1;
                                        }
                                        String::from_utf8(group_name).ok()
                                    })
                                    .collect(),
                            });
                        }
                    }
                }
            }
            None
        })
        .collect()
}
