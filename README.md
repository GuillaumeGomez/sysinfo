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
 * Android
 * macOS
 * Windows

It also compiles for Android but never been tested on it.

### Running on Raspberry

It'll be difficult to build on Raspberry. A good way-around is to be build on Linux before sending it to your Raspberry.

First install the arm toolchain, for example on Ubuntu: `sudo apt-get install gcc-multilib-arm-linux-gnueabihf`.

Then configure cargo to use the corresponding toolchain:

```bash
cat << EOF > ~/.cargo/config
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
EOF
```

Finally, cross compile:

```bash
rustup target add armv7-unknown-linux-gnueabihf
cargo build --target=armv7-unknown-linux-gnueabihf
```

## Code example

You have an example into the `examples` folder. Just run `cargo run` inside the `examples` folder to start it. Otherwise, here is a little code sample:

```rust
use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};

let mut sys = System::new();

// We display the disks:
println!("=> disk list:");
for disk in sys.get_disks() {
    println!("{:?}", disk);
}

// Network data:
for (interface_name, data) in sys.get_networks() {
    println!("{}: {}/{} B", interface_name, data.get_income(), data.get_outcome());
}

// Components temperature:
for component in sys.get_components() {
    println!("{:?}", component);
}

// Memory information:
println!("total memory: {} KiB", sys.get_total_memory());
println!("used memory : {} KiB", sys.get_used_memory());
println!("total swap  : {} KiB", sys.get_total_swap());
println!("used swap   : {} KiB", sys.get_used_swap());

// Number of processors
println!("NB processors: {}", sys.get_processors().len());

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

### Benchmarks

You can run the benchmarks locally with rust **nightly** by doing:

```bash
> cargo bench
```

Here are the current results:

**Linux**

<details>

```text
test bench_new                     ... bench:       3,741 ns/iter (+/- 252)
test bench_new_all                 ... bench:  10,491,084 ns/iter (+/- 450,925)
test bench_refresh_all             ... bench:   2,787,974 ns/iter (+/- 235,649)
test bench_refresh_components      ... bench:      24,270 ns/iter (+/- 1,127)
test bench_refresh_components_list ... bench:     370,693 ns/iter (+/- 51,925)
test bench_refresh_cpu             ... bench:      13,367 ns/iter (+/- 1,858)
test bench_refresh_disks           ... bench:       2,532 ns/iter (+/- 108)
test bench_refresh_disks_lists     ... bench:      50,359 ns/iter (+/- 5,877)
test bench_refresh_memory          ... bench:      11,713 ns/iter (+/- 1,006)
test bench_refresh_networks        ... bench:     220,246 ns/iter (+/- 24,294)
test bench_refresh_networks_list   ... bench:     229,648 ns/iter (+/- 82,050)
test bench_refresh_process         ... bench:      77,375 ns/iter (+/- 10,657)
test bench_refresh_processes       ... bench:   2,282,106 ns/iter (+/- 154,098)
test bench_refresh_system          ... bench:      52,466 ns/iter (+/- 4,710)
```
</details>

**Windows**

<details>

```text
test bench_new                     ... bench:   7,688,460 ns/iter (+/- 1,230,010)
test bench_new_all                 ... bench:  24,098,860 ns/iter (+/- 5,260,950)
test bench_refresh_all             ... bench:   3,096,107 ns/iter (+/- 94,257)
test bench_refresh_components      ... bench:   1,205,378 ns/iter (+/- 40,071)
test bench_refresh_components_list ... bench:   3,181,602 ns/iter (+/- 102,533)
test bench_refresh_cpu             ... bench:         395 ns/iter (+/- 18)
test bench_refresh_disks           ... bench:      53,082 ns/iter (+/- 1,834)
test bench_refresh_disks_lists     ... bench:     114,080 ns/iter (+/- 1,920)
test bench_refresh_memory          ... bench:         596 ns/iter (+/- 48)
test bench_refresh_networks        ... bench:      37,549 ns/iter (+/- 1,622)
test bench_refresh_networks_list   ... bench:     667,180 ns/iter (+/- 59,859)
test bench_refresh_process         ... bench:         755 ns/iter (+/- 47)
test bench_refresh_processes       ... bench:   1,217,488 ns/iter (+/- 69,041)
test bench_refresh_system          ... bench:   1,214,780 ns/iter (+/- 52,013)
```
</details>

**macOS**

<details>

```text
test bench_new                     ... bench:      56,861 ns/iter (+/- 5,653)
test bench_new_all                 ... bench:   4,634,509 ns/iter (+/- 1,604,369)
test bench_refresh_all             ... bench:   1,962,343 ns/iter (+/- 129,726)
test bench_refresh_components      ... bench:     294,752 ns/iter (+/- 45,107)
test bench_refresh_components_list ... bench:     895,672 ns/iter (+/- 112,586)
test bench_refresh_cpu             ... bench:      11,187 ns/iter (+/- 2,483)
test bench_refresh_disks           ... bench:         975 ns/iter (+/- 50)
test bench_refresh_disks_lists     ... bench:      25,955 ns/iter (+/- 3,159)
test bench_refresh_memory          ... bench:       3,440 ns/iter (+/- 198)
test bench_refresh_networks        ... bench:     211,552 ns/iter (+/- 16,686)
test bench_refresh_networks_list   ... bench:     211,138 ns/iter (+/- 22,644)
test bench_refresh_process         ... bench:       4,174 ns/iter (+/- 1,249)
test bench_refresh_processes       ... bench:     803,559 ns/iter (+/- 42,974)
test bench_refresh_system          ... bench:     365,762 ns/iter (+/- 55,893)
```
</details>

## Donations

If you appreciate my work and want to support me, you can do it here:

[![Become a patron](https://c5.patreon.com/external/logo/become_a_patron_button.png)](https://www.patreon.com/GuillaumeGomez)
