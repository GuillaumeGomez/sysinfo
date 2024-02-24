// Take a look at the license at the top of the repository in the LICENSE file.

use crate::common::Gid;

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
use crate::Group;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct GroupInner(crate::common::BaseGroupInner);

impl GroupInner {
    pub(crate) fn new(id: Gid, name: String) -> Self {
        Self(crate::common::BaseGroupInner { id, name })
    }

    pub(crate) fn id(&self) -> &Gid {
        &self.0.id
    }

    pub(crate) fn name(&self) -> &str {
        &self.0.name
    }
}

// Not used by mac.
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(crate) fn get_groups(groups: &mut Vec<Group>) {
    use std::fs::File;
    use std::io::Read;

    #[inline]
    fn parse_id(id: &str) -> Option<u32> {
        id.parse::<u32>().ok()
    }

    groups.clear();

    let mut s = String::new();

    let _ = File::open("/etc/group").and_then(|mut f| f.read_to_string(&mut s));

    for line in s.lines() {
        let mut parts = line.split(':');
        if let Some(name) = parts.next() {
            let mut parts = parts.skip(1);
            // Skip the user if the uid cannot be parsed correctly
            if let Some(gid) = parts.next().and_then(parse_id) {
                groups.push(Group {
                    inner: GroupInner::new(Gid(gid), name.to_owned()),
                });
            }
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) use crate::unix::apple::groups::get_groups;
