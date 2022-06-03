// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::utils::to_str;
use crate::{
    common::{Gid, Uid},
    User,
};

use std::ptr::null_mut;
use winapi::shared::lmcons::{MAX_PREFERRED_LENGTH, NET_API_STATUS};
use winapi::shared::minwindef::DWORD;
use winapi::shared::ntstatus::STATUS_SUCCESS;
use winapi::shared::winerror::ERROR_MORE_DATA;
use winapi::um::lmaccess::{NetUserEnum, NetUserGetLocalGroups};
use winapi::um::lmaccess::{
    FILTER_NORMAL_ACCOUNT, LG_INCLUDE_INDIRECT, LPLOCALGROUP_USERS_INFO_0, USER_INFO_0,
};
use winapi::um::lmapibuf::NetApiBufferFree;
use winapi::um::ntlsa::{
    LsaEnumerateLogonSessions, LsaFreeReturnBuffer, LsaGetLogonSessionData,
    PSECURITY_LOGON_SESSION_DATA,
};
use winapi::um::winnt::{LPWSTR, PLUID};

// FIXME: once this is mreged in winapi, it can be removed.
#[allow(non_upper_case_globals)]
const NERR_Success: NET_API_STATUS = 0;

unsafe fn get_groups_for_user(username: LPWSTR) -> Vec<String> {
    let mut buf: LPLOCALGROUP_USERS_INFO_0 = null_mut();
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

    if status == NERR_Success {
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

// FIXME: For now, the Uid is the user name, which is quite bad. Normally, there is `PSID` for
// that. But when getting the `PSID` from the processes, it doesn't match the ones we have for
// the users (`EqualSid`). Anyway, until I have time and motivation to fix this. It'll remain
// like that...
pub unsafe fn get_users() -> Vec<User> {
    let mut users = Vec::new();
    let mut buffer: *mut USER_INFO_0 = null_mut();
    let mut nb_read = 0;
    let mut total = 0;
    let mut resume_handle: DWORD = 0;

    loop {
        let status = NetUserEnum(
            null_mut(),
            0,
            FILTER_NORMAL_ACCOUNT,
            &mut buffer as *mut _ as *mut _,
            MAX_PREFERRED_LENGTH,
            &mut nb_read,
            &mut total,
            &mut resume_handle as *mut _ as *mut _,
        );
        if status == NERR_Success || status == ERROR_MORE_DATA {
            let entries: &[USER_INFO_0] = std::slice::from_raw_parts(buffer, nb_read as _);
            for entry in entries {
                if entry.usri0_name.is_null() {
                    continue;
                }
                // let mut user: *mut USER_INFO_23 = null_mut();

                // if NetUserGetInfo(
                //     null_mut(),
                //     entry.usri0_name,
                //     23,
                //     &mut user as *mut _ as *mut _,
                // ) == NERR_Success
                // {
                //     let groups = get_groups_for_user((*user).usri23_name);
                //     users.push(User {
                //         uid: Uid(name.clone().into_boxed_str()),
                //         gid: Gid(0),
                //         name: to_str((*user).usri23_name),
                //         groups,
                //     });
                // }
                // if !user.is_null() {
                //     NetApiBufferFree(user as *mut _);
                // }
                let groups = get_groups_for_user(entry.usri0_name);
                let name = to_str(entry.usri0_name);
                users.push(User {
                    uid: Uid(name.clone().into_boxed_str()),
                    gid: Gid(0),
                    name,
                    groups,
                });
            }
        } else {
            sysinfo_debug!(
                "NetUserEnum error: {}",
                if status == winapi::shared::winerror::ERROR_ACCESS_DENIED {
                    "access denied"
                } else if status == winapi::shared::winerror::ERROR_INVALID_LEVEL {
                    "invalid level"
                } else {
                    "unknown error"
                }
            );
        }
        if !buffer.is_null() {
            NetApiBufferFree(buffer as *mut _);
            buffer = null_mut();
        }
        if status != ERROR_MORE_DATA {
            break;
        }
    }

    // First part done. Second part now!
    let mut nb_sessions = 0;
    let mut uids: PLUID = null_mut();
    if LsaEnumerateLogonSessions(&mut nb_sessions, &mut uids) != STATUS_SUCCESS {
        sysinfo_debug!("LsaEnumerateLogonSessions failed");
    } else {
        for offset in 0..nb_sessions {
            let entry = uids.add(offset as _);
            let mut data: PSECURITY_LOGON_SESSION_DATA = null_mut();

            if LsaGetLogonSessionData(entry, &mut data) == STATUS_SUCCESS && !data.is_null() {
                let data = *data;
                if data.LogonType == winapi::um::ntlsa::Network {
                    continue;
                }
                let name = to_str(data.UserName.Buffer);
                if users.iter().any(|u| u.name == name) {
                    continue;
                }
                users.push(User {
                    uid: Uid(name.clone().into_boxed_str()),
                    gid: Gid(0),
                    name,
                    // There is no local groups for a non-local user.
                    groups: Vec::new(),
                });
            }
            if !data.is_null() {
                LsaFreeReturnBuffer(data as *mut _);
            }
        }
    }

    users
}
