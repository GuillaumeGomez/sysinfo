Returns the list of the supported signals on this system (used by
[`ProcessExt::kill_with`][crate::ProcessExt::kill_with]).

```
use sysinfo::{System, SUPPORTED_SIGNALS};

println!("supported signals: {:?}", SUPPORTED_SIGNALS);
```
