// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    common::{Gid, Uid},
    User,
};

use winapi::shared::lmcons::{MAX_PREFERRED_LENGTH, NET_API_STATUS};
use winapi::shared::minwindef::{DWORD, FALSE, TRUE};
use winapi::shared::winerror::{ERROR_MORE_DATA, ERROR_SUCCESS};
use winapi::um::heapapi::{GetProcessHeap, HeapAlloc, HeapFree};
use winapi::um::lmaccess::{NetUserEnum, NetUserGetInfo, NetUserGetLocalGroups};
use winapi::um::lmaccess::{
    FILTER_NORMAL_ACCOUNT, LG_INCLUDE_INDIRECT, LPLOCALGROUP_USERS_INFO_0, PNET_DISPLAY_USER,
    UF_NORMAL_ACCOUNT, USER_INFO_0, USER_INFO_23,
};
use winapi::um::lmapibuf::NetApiBufferFree;
use winapi::um::securitybaseapi::{CopySid, EqualSid, GetLengthSid};
use winapi::um::winnt::{HEAP_ZERO_MEMORY, LPWSTR, PSID};

#[repr(transparent)]
pub(crate) struct Sid(pub(crate) PSID);

unsafe impl Send for Sid {}
unsafe impl Sync for Sid {}

impl Sid {
    unsafe fn new_from(sid: PSID) -> Option<Self> {
        let size = GetLengthSid(sid);
        let ret_sid: PSID = HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, size as _) as *mut _;
        if ret_sid.is_null() {
            return None;
        }
        if CopySid(size, ret_sid, sid) == 0 {
            HeapFree(GetProcessHeap(), 0, ret_sid as *mut _);
            sysinfo_debug!("CopySid failed...");
            None
        } else {
            Some(Self(ret_sid))
        }
    }
}

impl std::cmp::PartialEq for Sid {
    fn eq(&self, other: &Self) -> bool {
        unsafe { EqualSid(self.0, other.0) == TRUE }
    }
}
impl std::cmp::Eq for Sid {}
impl std::cmp::PartialOrd for Sid {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}
impl std::cmp::Ord for Sid {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
impl std::fmt::Debug for Sid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let mut s = std::ptr::null_mut();
            let to_display = if winapi::shared::sddl::ConvertSidToStringSidW(self.0, &mut s) == TRUE
            {
                to_str(s as *mut _)
            } else {
                "Unknown".to_owned()
            };
            if !s.is_null() {
                winapi::um::winbase::LocalFree(s as *mut _);
            }
            f.debug_struct("Sid").field("0", &to_display).finish()
        }
    }
}
impl Drop for Sid {
    fn drop(&mut self) {
        unsafe {
            HeapFree(GetProcessHeap(), 0, self.0);
        }
    }
}

unsafe fn to_str(p: LPWSTR) -> String {
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

const NERR_Success: NET_API_STATUS = 0;

unsafe fn get_groups_for_user(username: LPWSTR) -> Vec<String> {
    let mut buf: LPLOCALGROUP_USERS_INFO_0 = std::ptr::null_mut();
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

pub unsafe fn get_users() -> Vec<User> {
    let mut users = Vec::new();

    let mut buffer: *mut USER_INFO_0 = std::ptr::null_mut();
    let mut nb_read = 0;
    let mut total = 0;
    let mut resume_handle: DWORD = 0;

    loop {
        let status = NetUserEnum(
            std::ptr::null_mut(),
            0,
            FILTER_NORMAL_ACCOUNT,
            &mut buffer as *mut _ as *mut _,
            MAX_PREFERRED_LENGTH,
            &mut nb_read,
            &mut total,
            &mut resume_handle as *mut _ as *mut _,
        );
        if status != NERR_Success && status != ERROR_MORE_DATA {
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
            break;
        }
        let entries: &[USER_INFO_0] = std::slice::from_raw_parts(buffer, nb_read as _);
        for entry in entries {
            if entry.usri0_name.is_null() {
                continue;
            }
            let mut user: *mut USER_INFO_23 = std::ptr::null_mut();

            if NetUserGetInfo(
                std::ptr::null_mut(),
                entry.usri0_name,
                23,
                &mut user as *mut _ as *mut _,
            ) == NERR_Success
            {
                let uid = match Sid::new_from((*user).usri23_user_sid) {
                    Some(uid) => uid,
                    None => continue,
                };
                let groups = get_groups_for_user(entry.usri0_name);
                users.push(User {
                    uid: Uid(uid),
                    gid: Gid(0),
                    name: to_str(entry.usri0_name),
                    groups,
                });
            }
            if !user.is_null() {
                NetApiBufferFree(user as *mut _);
            }
        }
        if !buffer.is_null() {
            NetApiBufferFree(buffer as *mut _);
            buffer = std::ptr::null_mut();
        }
        if status != ERROR_MORE_DATA {
            break;
        }
    }

    users
}
