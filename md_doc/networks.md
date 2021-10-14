Networks interfaces.

```no_run
use sysinfo::{NetworksExt, System, SystemExt};

let s = System::new_all();
let networks = s.networks();
```
