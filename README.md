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
    println!("{}: {}/{} B", interface_name, data.get_received(), data.get_transmitted());
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
test bench_new                     ... bench:   7,778,330 ns/iter (+/- 355,054)
test bench_new_all                 ... bench:  85,655,800 ns/iter (+/- 2,082,645)
test bench_refresh_all             ... bench:   1,404,736 ns/iter (+/- 106,109)
test bench_refresh_components      ... bench:           1 ns/iter (+/- 0)
test bench_refresh_components_list ... bench:      29,210 ns/iter (+/- 2,278)
test bench_refresh_cpu             ... bench:      34,225 ns/iter (+/- 29,786)
test bench_refresh_disks           ... bench:      54,453 ns/iter (+/- 2,751)
test bench_refresh_disks_list      ... bench:     125,164 ns/iter (+/- 2,692)
test bench_refresh_memory          ... bench:       1,007 ns/iter (+/- 38)
test bench_refresh_networks        ... bench:      38,753 ns/iter (+/- 2,527)
test bench_refresh_networks_list   ... bench:   1,352,400 ns/iter (+/- 146,762)
test bench_refresh_process         ... bench:       1,284 ns/iter (+/- 71)
test bench_refresh_processes       ... bench:   1,045,020 ns/iter (+/- 77,908)
test bench_refresh_system          ... bench:      35,787 ns/iter (+/- 3,587)
test bench_refresh_users_list      ... bench:   3,339,050 ns/iter (+/- 172,762)
```
</details>

**macOS**

<details>

```text
test bench_new                     ... bench:      86,404 ns/iter (+/- 9,402)
test bench_new_all                 ... bench:  21,123,771 ns/iter (+/- 570,722)
test bench_refresh_all             ... bench:   1,757,683 ns/iter (+/- 203,234)
test bench_refresh_components      ... bench:     325,560 ns/iter (+/- 41,068)
test bench_refresh_components_list ... bench:     989,827 ns/iter (+/- 221,093)
test bench_refresh_cpu             ... bench:       8,535 ns/iter (+/- 487)
test bench_refresh_disks           ... bench:         939 ns/iter (+/- 33)
test bench_refresh_disks_lists     ... bench:      25,093 ns/iter (+/- 2,080)
test bench_refresh_memory          ... bench:       2,174 ns/iter (+/- 55)
test bench_refresh_networks        ... bench:     181,558 ns/iter (+/- 7,325)
test bench_refresh_networks_list   ... bench:     180,410 ns/iter (+/- 2,414)
test bench_refresh_process         ... bench:       5,570 ns/iter (+/- 431)
test bench_refresh_processes       ... bench:     683,455 ns/iter (+/- 14,995)
test bench_refresh_system          ... bench:     362,875 ns/iter (+/- 172,547)
test bench_refresh_users_list      ... bench:  16,783,834 ns/iter (+/- 465,111)
```
</details>

## Donations

If you appreciate my work and want to support me, you can do it here:

[![Become a patron](https://c5.patreon.com/external/logo/become_a_patron_button.png)](https://www.patreon.com/GuillaumeGomez)
