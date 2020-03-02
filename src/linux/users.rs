//
// Sysinfo
//
// Copyright (c) 2020 Guillaume Gomez
//

use crate::utils::users::{endswith, get_users_list as users_list};
use crate::User;

// On linux, we actually can read the /etc/passwd file directly. Maybe it'd be better to use it?
pub fn get_users_list() -> Vec<User> {
    users_list(|shell| !endswith(shell, b"/false") && !endswith(shell, b"/nologin"))
}
