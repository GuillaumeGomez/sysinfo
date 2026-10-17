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

pub(crate) fn get_groups(groups: &mut Vec<Group>) {
    groups.clear();

    let mut groups_map = std::collections::HashMap::with_capacity(10);

    unsafe {
        libc::setgrent();
        loop {
            let gr = retry_eintr!(null => libc::getgrent());
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
