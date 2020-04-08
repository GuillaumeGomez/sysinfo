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
use sysinfo::{NetworkExt, NetworksExt, ProcessExt, System, SystemExt};

let mut sys = System::new_all();

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

// We show the processes and some of their information:
for (pid, process) in sys.get_processes() {
    println!("[{}] {} {:?}", pid, process.name(), process.disk_usage());
}
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
test bench_new                     ... bench:     182,536 ns/iter (+/- 21,074)
test bench_new_all                 ... bench:  19,911,714 ns/iter (+/- 1,612,109)
test bench_refresh_all             ... bench:   5,649,643 ns/iter (+/- 444,129)
test bench_refresh_components      ... bench:      25,293 ns/iter (+/- 1,748)
test bench_refresh_components_list ... bench:     382,331 ns/iter (+/- 31,620)
test bench_refresh_cpu             ... bench:      13,633 ns/iter (+/- 1,135)
test bench_refresh_disks           ... bench:       2,509 ns/iter (+/- 75)
test bench_refresh_disks_list      ... bench:      51,488 ns/iter (+/- 5,470)
test bench_refresh_memory          ... bench:      12,941 ns/iter (+/- 3,023)
test bench_refresh_networks        ... bench:     256,506 ns/iter (+/- 37,196)
test bench_refresh_networks_list   ... bench:     266,751 ns/iter (+/- 54,535)
test bench_refresh_process         ... bench:     117,372 ns/iter (+/- 8,732)
test bench_refresh_processes       ... bench:   5,125,929 ns/iter (+/- 560,050)
test bench_refresh_system          ... bench:      52,526 ns/iter (+/- 6,786)
test bench_refresh_users_list      ... bench:   2,479,582 ns/iter (+/- 1,063,982)
```
</details>

**Windows**

<details>

```text
test bench_new                     ... bench:   7,335,755 ns/iter (+/- 469,000)
test bench_new_all                 ... bench:  32,233,480 ns/iter (+/- 1,567,239)
test bench_refresh_all             ... bench:   1,433,015 ns/iter (+/- 126,322)
test bench_refresh_components      ... bench:           1 ns/iter (+/- 0)
test bench_refresh_components_list ... bench:   9,835,060 ns/iter (+/- 407,072)
test bench_refresh_cpu             ... bench:      33,873 ns/iter (+/- 2,177)
test bench_refresh_disks           ... bench:      58,951 ns/iter (+/- 6,128)
test bench_refresh_disks_list      ... bench:     125,199 ns/iter (+/- 2,741)
test bench_refresh_memory          ... bench:       1,004 ns/iter (+/- 56)
test bench_refresh_networks        ... bench:      39,013 ns/iter (+/- 2,676)
test bench_refresh_networks_list   ... bench:   1,341,850 ns/iter (+/- 78,258)
test bench_refresh_process         ... bench:       2,116 ns/iter (+/- 58)
test bench_refresh_processes       ... bench:   1,032,447 ns/iter (+/- 57,695)
test bench_refresh_system          ... bench:      35,374 ns/iter (+/- 3,200)
test bench_refresh_users_list      ... bench:   3,321,140 ns/iter (+/- 135,160)
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
