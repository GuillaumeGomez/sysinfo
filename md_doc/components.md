Component interfaces.

```no_run
use sysinfo::{Components, ComponentsExt};

let mut components = Components::new();
components.refresh_list();
for component in components.components() {
    println!("{component:?}");
}
```