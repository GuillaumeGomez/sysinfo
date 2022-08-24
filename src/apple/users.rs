// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    common::{Gid, Uid},
    User,
};

use crate::sys::utils;
use libc::{c_char, endpwent, getgrgid, getgrouplist, getpwent, gid_t, setpwent, strlen};

fn get_user_groups(name: *const c_char, group_id: gid_t) -> Vec<String> {
    let mut add = 0;

    loop {
        let mut nb_groups = 256 + add;
        let mut groups = Vec::with_capacity(nb_groups as _);
        unsafe {
            if getgrouplist(name, group_id as _, groups.as_mut_ptr(), &mut nb_groups) == -1 {
                add += 100;
                continue;
            }
            groups.set_len(nb_groups as _);
            return groups
                .into_iter()
                .filter_map(|g| {
                    let group = getgrgid(g as _);
                    if group.is_null() {
                        return None;
                    }
                    utils::cstr_to_rust((*group).gr_name)
                })
                .collect();
        }
    }
}

fn endswith(s1: *const c_char, s2: &[u8]) -> bool {
    if s1.is_null() {
        return false;
    }
    unsafe {
        let mut len = strlen(s1) as isize - 1;
        let mut i = s2.len() as isize - 1;
        while len >= 0 && i >= 0 && *s1.offset(len) == s2[i as usize] as _ {
            i -= 1;
            len -= 1;
        }
        i == -1
    }
}

fn users_list<F>(filter: F) -> Vec<User>
where
    F: Fn(*const c_char, u32) -> bool,
{
    let mut users = Vec::new();

    unsafe {
        setpwent();
        loop {
            let pw = getpwent();
            if pw.is_null() {
                break;
            }

            if !filter((*pw).pw_shell, (*pw).pw_uid) {
                // This is not a "real" or "local" user.
                continue;
            }

            let groups = get_user_groups((*pw).pw_name, (*pw).pw_gid);
            let uid = (*pw).pw_uid;
            let gid = (*pw).pw_gid;
            if let Some(name) = utils::cstr_to_rust((*pw).pw_name) {
                users.push(User {
                    uid: Uid(uid),
                    gid: Gid(gid),
                    name,
                    groups,
                });
            }
        }
        endpwent();
    }
    users.sort_unstable_by(|x, y| x.name.partial_cmp(&y.name).unwrap());
    users.dedup_by(|a, b| a.name == b.name);
    users
}

pub(crate) fn get_users_list() -> Vec<User> {
    users_list(|shell, uid| {
        !endswith(shell, b"/false") && !endswith(shell, b"/uucico") && uid < 65536
    })
}

// This was the OSX-based solution. It provides enough information, but what a mess!
// pub fn get_users_list() -> Vec<User> {
//     let mut users = Vec::new();
//     let node_name = b"/Local/Default\0";

//     unsafe {
//         let node_name = ffi::CFStringCreateWithCStringNoCopy(
//             std::ptr::null_mut(),
//             node_name.as_ptr() as *const c_char,
//             ffi::kCFStringEncodingMacRoman,
//             ffi::kCFAllocatorNull as *mut c_void,
//         );
//         let node_ref = ffi::ODNodeCreateWithName(
//             ffi::kCFAllocatorDefault,
//             ffi::kODSessionDefault,
//             node_name,
//             std::ptr::null_mut(),
//         );
//         let query = ffi::ODQueryCreateWithNode(
//             ffi::kCFAllocatorDefault,
//             node_ref,
//             ffi::kODRecordTypeUsers as _, // kODRecordTypeGroups
//             std::ptr::null(),
//             0,
//             std::ptr::null(),
//             std::ptr::null(),
//             0,
//             std::ptr::null_mut(),
//         );
//         if query.is_null() {
//             return users;
//         }
//         let results = ffi::ODQueryCopyResults(
//             query,
//             false as _,
//             std::ptr::null_mut(),
//         );
//         let len = ffi::CFArrayGetCount(results);
//         for i in 0..len {
//             let name = match get_user_name(ffi::CFArrayGetValueAtIndex(results, i)) {
//                 Some(n) => n,
//                 None => continue,
//             };
//             let groups = get_user_groups(&name);
//             users.push(User { name });
//         }

//         ffi::CFRelease(results as *const c_void);
//         ffi::CFRelease(query as *const c_void);
//         ffi::CFRelease(node_ref as *const c_void);
//         ffi::CFRelease(node_name as *const c_void);
//     }
//     users.sort_unstable_by(|x, y| x.name.partial_cmp(&y.name).unwrap());
//     return users;
// }

// fn get_user_name(result: *const c_void) -> Option<String> {
//     let user_name = ffi::ODRecordGetRecordName(result as _);
//     let ptr = ffi::CFStringGetCharactersPtr(user_name);
//     String::from_utf16(&if ptr.is_null() {
//         let len = ffi::CFStringGetLength(user_name); // It returns the len in UTF-16 code pairs.
//         if len == 0 {
//             continue;
//         }
//         let mut v = Vec::with_capacity(len as _);
//         for x in 0..len {
//             v.push(ffi::CFStringGetCharacterAtIndex(user_name, x));
//         }
//         v
//     } else {
//         let mut v: Vec<u16> = Vec::new();
//         let mut x = 0;
//         loop {
//             let letter = *ptr.offset(x);
//             if letter == 0 {
//                 break;
//             }
//             v.push(letter);
//             x += 1;
//         }
//         v
//     }.ok()
// }
