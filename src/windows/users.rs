//
// Sysinfo
//
// Copyright (c) 2020 Guillaume Gomez
//

use User;

use winapi::shared::lmcons::MAX_PREFERRED_LENGTH;
use winapi::shared::winerror::{ERROR_MORE_DATA, ERROR_SUCCESS};
use winapi::um::lmaccess::NetQueryDisplayInformation;
use winapi::um::lmaccess::{
    LG_INCLUDE_INDIRECT, LPLOCALGROUP_USERS_INFO_0, PNET_DISPLAY_USER, UF_NORMAL_ACCOUNT,
};
use winapi::um::lmapibuf::NetApiBufferFree;
use winapi::um::winnt::LPWSTR;

use sys::ffi::NetUserGetLocalGroups;

unsafe fn to_str(p: LPWSTR) -> String {
    let mut i = 0;
    let mut s = Vec::new();

    loop {
        let c = *p.offset(i);
        if c == 0 {
            break;
        }
        s.push(c);
        i += 1;
    }
    String::from_utf16(&s).unwrap_or_else(|_e| {
        sysinfo_debug!("Failed to convert to UTF-16 string: {}", _e);
        String::new()
    })
}

unsafe fn get_groups_for_user(username: LPWSTR) -> Vec<String> {
    let mut buf: LPLOCALGROUP_USERS_INFO_0 = ::std::ptr::null_mut();
    let mut nb_entries = 0;
    let mut total_entries = 0;
    let mut groups;

    let status = NetUserGetLocalGroups(
        [0u16].as_ptr(),
        username,
        0,
        LG_INCLUDE_INDIRECT,
        &mut buf as *mut _ as _,
        MAX_PREFERRED_LENGTH,
        &mut nb_entries,
        &mut total_entries,
    );

    if status == 0 {
        // NERR_Success
        groups = Vec::with_capacity(nb_entries as _);

        if !buf.is_null() {
            for i in 0..nb_entries {
                let tmp = buf.offset(i as _);
                if tmp.is_null() {
                    break;
                }
                groups.push(to_str((*tmp).lgrui0_name));
            }
        }
    } else {
        groups = Vec::new();
        sysinfo_debug!("NetUserGetLocalGroups failed with ret code {}", status);
    }
    if !buf.is_null() {
        NetApiBufferFree(buf as *mut _);
    }

    groups
}

pub unsafe fn get_users() -> Vec<User> {
    let mut users = Vec::new();
    let mut i = 0;
    let mut buf: PNET_DISPLAY_USER = ::std::ptr::null_mut();
    let mut nb_entries = 0;
    let mut res = ERROR_MORE_DATA;

    while res == ERROR_MORE_DATA {
        res = NetQueryDisplayInformation(
            [0u16].as_ptr(),
            1,
            i,
            100,
            MAX_PREFERRED_LENGTH,
            &mut nb_entries,
            &mut buf as *mut _ as _,
        );

        if res == ERROR_SUCCESS || res == ERROR_MORE_DATA {
            for it in 0..nb_entries {
                let buf = buf.offset(it as _);
                if (*buf).usri1_flags & UF_NORMAL_ACCOUNT != 0 {
                    let groups = get_groups_for_user((*buf).usri1_name);
                    users.push(User {
                        name: to_str((*buf).usri1_name),
                        groups,
                    });
                }
                i = (*buf).usri1_next_index;
            }
            NetApiBufferFree(buf as *mut _);
        } else {
            sysinfo_debug!("NetQueryDisplayInformation failed with ret code {}", res);
            break;
        }
    }
    users
}
