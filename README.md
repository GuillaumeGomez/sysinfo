# sysinfo [![][img_travis-ci]][travis-ci] [![Build status](https://ci.appveyor.com/api/projects/status/nhep876b3legunwd/branch/master?svg=true)](https://ci.appveyor.com/project/GuillaumeGomez/sysinfo/branch/master) [![][img_crates]][crates] [![][img_doc]][doc]

[img_travis-ci]: https://api.travis-ci.org/GuillaumeGomez/sysinfo.png?branch=master
[img_crates]: https://img.shields.io/crates/v/sysinfo.svg
[img_doc]: https://img.shields.io/badge/rust-documentation-blue.svg

[travis-ci]: https://travis-ci.org/GuillaumeGomez/sysinfo
[crates]: https://crates.io/crates/sysinfo
[doc]: https://docs.rs/sysinfo/

A system handler to interact with processes.

Support the following platforms:

 * Linux
 * Raspberry
 * Mac OSX
 * Windows

It also compiles for Android but never been tested on it.

### Running on Raspberry

It'll be difficult to build on Raspberry. A good way-around is to be build on Linux before sending it to your Raspberry:

```bash
rustup target add armv7-unknown-linux-gnueabihf
cargo build --target=armv7-unknown-linux-gnueabihf
```

## Code example

You have an example into the `examples` folder. Just run `cargo run` inside the `examples` folder to start it. Otherwise, here is a little code sample:

```rust
extern crate sysinfo;

use sysinfo::{NetworkExt, System, SystemExt};

let mut sys = System::new();

// We display the disks:
println!("=> disk list:");
for disk in sys.get_disks() {
    println!("{:?}", disk);
}

// Network data:
println!("input data : {} B", sys.get_network().get_income());
println!("output data: {} B", sys.get_network().get_outcome());

// Components temperature:
for component in sys.get_components_list() {
    println!("{:?}", component);
}

// Memory information:
println!("total memory: {} kB", sys.get_total_memory());
println!("used memory : {} kB", sys.get_used_memory());
println!("total swap  : {} kB", sys.get_total_swap());
println!("used swap   : {} kB", sys.get_used_swap());

// Number of processors
println!("NB processors: {}", sys.get_processor_list().len());

// To refresh all system information:
sys.refresh_all();
```

## C interface

It's possible to use this crate directly from C. Take a look at the `Makefile` and at the `examples/src/simple.c` file.

To build the C example, just run:

```bash
> make
> ./simple
# If needed:
> LD_LIBRARY_PATH=target/release/ ./simple
```

## Donations

If you appreciate my work and want to support me, you can do it here:

[![Become a patron](https://c5.patreon.com/external/logo/become_a_patron_button.png)](https://www.patreon.com/GuillaumeGomez)
