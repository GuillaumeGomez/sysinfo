Type containing user information.

It is returned by [`Users`][crate::Users].

```no_run
use sysinfo::{Users, UsersExt};

let mut users = Users::new();
users.refresh_list();
for user in users.users() {
    println!("{:?}", user);
}
```
