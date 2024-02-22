// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{Gid, Group, Uid, User};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct GroupInner;

impl GroupInner {
    pub(crate) fn id(&self) -> &Gid {
        &Gid(0)
    }

    pub(crate) fn name(&self) -> &str {
        ""
    }
}

pub(crate) fn get_groups(_: &mut Vec<Group>) {}
