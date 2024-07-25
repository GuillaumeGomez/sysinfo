
With the `serde` feature enabled, you can then serialize `sysinfo` types. Let's see an example with `serde_json`:

```
use sysinfo::System;

let mut sys = System::new();
// First we update all information of our `System` struct.
sys.refresh_all();

println!("{}", serde_json::to_string(&sys).unwrap());
```
