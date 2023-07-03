
With the `serde` feature enabled, you can then serialize `sysinfo` types. Let's see an example
with `serde_json` (you need to add `serde_json` dependency in your `Cargo.toml` file to run
this example):

```
use sysinfo::{System, SystemExt};

let mut sys = System::new_all();
// First we update all information of our `System` struct.
sys.refresh_all();

println!("{}", serde_json::to_string(&sys).unwrap());
```
