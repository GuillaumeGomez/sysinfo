// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{Gid, Group, Uid, UserExt};

#[doc = include_str!("../../md_doc/user.md")]
pub struct User;

impl UserExt for User {
    fn id(&self) -> &Uid {
        &Uid(0)
    }

    fn group_id(&self) -> Gid {
        Gid(0)
    }

    fn name(&self) -> &str {
        ""
    }

    fn groups(&self) -> Vec<Group> {
        Vec::new()
    }
}
