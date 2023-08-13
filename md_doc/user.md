Type containing user information.

It is returned by [`SystemExt::users`][crate::SystemExt::users].

```no_run
use sysinfo::{System, SystemExt};

let s = System::new_all();
println!("users: {:?}", s.users());
```
