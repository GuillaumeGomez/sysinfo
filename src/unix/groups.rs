// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{Error, Gid, Group, GroupInner};

impl GroupInner {
    pub(crate) fn new(id: crate::Gid, name: String) -> Self {
        Self { id, name }
    }

    pub(crate) fn id(&self) -> &crate::Gid {
        &self.id
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(crate) fn get_groups(groups: &mut Vec<crate::Group>) {
    use std::io::{BufRead, BufReader};
    use std::str::FromStr;

    groups.clear();

    let mut groups_map = std::collections::HashMap::with_capacity(10);

    // We cannot use `getgrent` because they're not thread-safe on linux. We could use
    // `fgetgrent_r`, but it's not available on all platforms, so we read the group file directly.
    let Ok(file) = std::fs::File::open("/etc/group") else {
        sysinfo_debug!("failed to open `/etc/group`");
        return;
    };

    for line in BufReader::new(file).lines() {
        let Ok(line) = line else { continue };
        let mut parts = line.split(':');
        let Some(name) = parts.next() else { continue };
        if groups_map.contains_key(name) {
            continue;
        }
        parts.next(); // We skip the password field.
        let Some(gid) = parts.next().and_then(|v| libc::gid_t::from_str(v).ok()) else {
            continue;
        };
        // Next item is the users list in this group (comma separated), but we don't use it so
        // no need to retrieve it.

        groups_map.insert(name.to_owned(), Gid(gid));
    }

    for (name, gid) in groups_map {
        groups.push(Group {
            inner: GroupInner::new(gid, name),
        });
    }
}

// Interestingly enough, on macOS and iOS, `setgrent`, `getgrent` and `endgrent` are thread-safe.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) fn get_groups(groups: &mut Vec<Group>) {
    groups.clear();

    let mut groups_map = std::collections::HashMap::with_capacity(10);

    unsafe {
        libc::setgrent();
        loop {
            let gr = retry_eintr!(set_to_0 => null => libc::getgrent());
            if gr.is_null() {
                break;
            }

            if let Some(name) = crate::unix::utils::cstr_to_rust((*gr).gr_name) {
                if groups_map.contains_key(&name) {
                    continue;
                }

                let gid = (*gr).gr_gid;
                groups_map.insert(name, Gid(gid));
            }
        }
        libc::endgrent();
    }
    for (name, gid) in groups_map {
        groups.push(Group {
            inner: GroupInner::new(gid, name),
        });
    }
}

pub(crate) fn new_groups() -> Result<Vec<Group>, Error> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    // A stale `EINTR` left by an earlier call must not make `get_groups` loop forever.
    #[test]
    fn get_groups_ignores_stale_eintr() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            unsafe {
                *crate::unix::libc_errno() = libc::EINTR;
            }
            let mut groups = Vec::new();
            super::get_groups(&mut groups);
            let _ = tx.send(groups.len());
        });
        let found = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("get_groups did not return");
        assert!(found > 0);
    }
}
