// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::utils::to_utf8_str;
use crate::{Gid, Group, GroupInner};

use std::ptr::null_mut;
use windows::Win32::Foundation::{ERROR_MORE_DATA, ERROR_SUCCESS};
use windows::Win32::NetworkManagement::NetManagement::{
    NetApiBufferFree, NetQueryDisplayInformation, MAX_PREFERRED_LENGTH, NET_DISPLAY_GROUP,
};

impl GroupInner {
    pub(crate) fn new(id: Gid, name: String) -> Self {
        Self { id, name }
    }

    pub(crate) fn id(&self) -> &Gid {
        &self.id
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

struct NetApiBuffer(*mut NET_DISPLAY_GROUP);

impl Drop for NetApiBuffer {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { NetApiBufferFree(Some(self.0.cast())) };
        }
    }
}

impl Default for NetApiBuffer {
    fn default() -> Self {
        Self(null_mut())
    }
}

impl NetApiBuffer {
    pub fn inner_mut(&mut self) -> *mut *mut NET_DISPLAY_GROUP {
        &mut self.0 as *mut _
    }
}

pub(crate) fn get_groups(groups: &mut Vec<Group>) {
    groups.clear();

    unsafe {
        let mut i = 0;
        let mut nb_entries = 0;
        loop {
            let mut buff = NetApiBuffer::default();
            let res = NetQueryDisplayInformation(
                None,
                3, // Level. Here, group account information.
                i, // If it's not the first iteration, frm which index we want to start from.
                1000,
                MAX_PREFERRED_LENGTH,
                &mut nb_entries,
                buff.inner_mut() as *mut _,
            );
            if res != ERROR_SUCCESS.0 && res != ERROR_MORE_DATA.0 {
                sysinfo_debug!("NetQueryDisplayInformation failed: {res:?}");
                break;
            }
            let mut p = buff.0;
            for _ in 0..nb_entries {
                let name = to_utf8_str((*p).grpi3_name);
                groups.push(Group {
                    inner: GroupInner::new(Gid((*p).grpi3_group_id), name),
                });
                //
                // If there is more data, set the index.
                //
                i = (*p).grpi3_next_index;
                p = p.add(1);
            }
            if res != ERROR_MORE_DATA.0 {
                break;
            }
        }
    }
}
