// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    common::{Gid, Uid},
    User,
};

use libc::{getgrgid_r, getgrouplist};
use std::fs::File;
use std::io::Read;

pub(crate) unsafe fn get_group_name(
    id: libc::gid_t,
    buffer: &mut Vec<libc::c_char>,
) -> Option<String> {
    let mut g = std::mem::MaybeUninit::<libc::group>::uninit();
    let mut tmp_ptr = std::ptr::null_mut();
    let mut last_errno = 0;
    loop {
        if retry_eintr!(set_to_0 => last_errno => getgrgid_r(
            id as _,
            g.as_mut_ptr() as _,
            buffer.as_mut_ptr(),
            buffer.capacity() as _,
            &mut tmp_ptr as _
        )) != 0
        {
            // If there was not enough memory, we give it more.
            if last_errno == libc::ERANGE as _ {
                buffer.reserve(2048);
                continue;
            }
            return None;
        }
        break;
    }
    let g = g.assume_init();
    let mut group_name = Vec::new();
    let c_group_name = g.gr_name;
    let mut x = 0;
    loop {
        let c = *c_group_name.offset(x);
        if c == 0 {
            break;
        }
        group_name.push(c as u8);
        x += 1;
    }
    String::from_utf8(group_name).ok()
}

pub(crate) unsafe fn get_user_groups(
    name: *const libc::c_char,
    group_id: libc::gid_t,
    groups: &mut Vec<crate::GroupId>,
    buffer: &mut Vec<libc::c_char>,
) -> Vec<String> {
    loop {
        let mut nb_groups = groups.capacity();
        if getgrouplist(
            name,
            group_id as _,
            groups.as_mut_ptr(),
            &mut nb_groups as *mut _ as *mut _,
        ) == -1
        {
            groups.reserve(256);
            continue;
        }
        groups.set_len(nb_groups as _);
        return groups
            .iter()
            .filter_map(|group_id| crate::users::get_group_name(*group_id as _, buffer))
            .collect();
    }
}

// Not used by mac.
#[allow(unused)]
pub(crate) fn get_users_list() -> Vec<User> {
    #[inline]
    fn parse_id(id: &str) -> Option<u32> {
        id.parse::<u32>().ok()
    }

    let mut s = String::new();
    let mut buffer = Vec::with_capacity(2048);
    let mut groups = Vec::with_capacity(256);

    let _ = File::open("/etc/passwd").and_then(|mut f| f.read_to_string(&mut s));
    s.lines()
        .filter_map(|line| {
            let mut parts = line.split(':');
            if let Some(username) = parts.next() {
                let mut parts = parts.skip(1);
                // Skip the user if the uid cannot be parsed correctly
                if let Some(uid) = parts.next().and_then(parse_id) {
                    if let Some(group_id) = parts.next().and_then(parse_id) {
                        let mut c_user = username.as_bytes().to_vec();
                        c_user.push(0);
                        // Let's get all the group names!
                        return Some(User {
                            uid: Uid(uid),
                            gid: Gid(group_id),
                            name: username.to_owned(),
                            groups: unsafe {
                                get_user_groups(
                                    c_user.as_ptr() as *const _,
                                    group_id,
                                    &mut groups,
                                    &mut buffer,
                                )
                            },
                        });
                    }
                }
            }
            None
        })
        .collect()
}
