Process id

Can be used as an integer type by simple casting. For example:

```
use sysinfo::{AsU32, Pid};

let p: Pid = 0;
let i: u32 = p.as_u32();
```

On glibc systems this is a glibc [`pid_t`](https://www.gnu.org/software/libc/manual/html_node/Process-Identification.html).

On Windows systems this is a [`usize` and represents a windows process identifier](https://docs.microsoft.com/en-us/windows/win32/procthread/process-handles-and-identifiers).
