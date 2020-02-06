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

It'll be difficult to build on Raspberry. A good way-around is to be build on Linux before sending it to your Raspberry:

```bash
rustup target add armv7-unknown-linux-gnueabihf
cargo build --target=armv7-unknown-linux-gnueabihf
```

## Code example

You have an example into the `examples` folder. Just run `cargo run` inside the `examples` folder to start it. Otherwise, here is a little code sample:

```rust
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
println!("total memory: {} KiB", sys.get_total_memory());
println!("used memory : {} KiB", sys.get_used_memory());
println!("total swap  : {} KiB", sys.get_total_swap());
println!("used swap   : {} KiB", sys.get_used_swap());

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
test bench_new                   ... bench:  14,738,570 ns/iter (+/- 586,107)
test bench_new_all               ... bench:  27,132,490 ns/iter (+/- 1,292,307)
test bench_refresh_all           ... bench:   3,075,022 ns/iter (+/- 110,711)
test bench_refresh_cpu           ... bench:         392 ns/iter (+/- 30)
test bench_refresh_disks         ... bench:      41,778 ns/iter (+/- 954)
test bench_refresh_disks_lists   ... bench:     113,942 ns/iter (+/- 4,240)
test bench_refresh_memory        ... bench:         578 ns/iter (+/- 41)
test bench_refresh_networks      ... bench:      38,178 ns/iter (+/- 3,718)
test bench_refresh_networks_list ... bench:     668,390 ns/iter (+/- 30,642)
test bench_refresh_process       ... bench:         745 ns/iter (+/- 62)
test bench_refresh_processes     ... bench:   1,179,581 ns/iter (+/- 188,119)
test bench_refresh_system        ... bench:   1,230,542 ns/iter (+/- 64,231)
test bench_refresh_temperatures  ... bench:   1,231,260 ns/iter (+/- 111,274)
```
</details>

**macOS**

<details>

```text
test bench_new                   ... bench:      54,862 ns/iter (+/- 6,528)
test bench_new_all               ... bench:   4,989,120 ns/iter (+/- 1,001,529)
test bench_refresh_all           ... bench:   1,924,596 ns/iter (+/- 341,209)
test bench_refresh_cpu           ... bench:      10,521 ns/iter (+/- 1,623)
test bench_refresh_disks         ... bench:         945 ns/iter (+/- 95)
test bench_refresh_disks_lists   ... bench:      29,315 ns/iter (+/- 3,076)
test bench_refresh_memory        ... bench:       3,275 ns/iter (+/- 143)
test bench_refresh_networks      ... bench:     200,670 ns/iter (+/- 28,674)
test bench_refresh_networks_list ... bench:     200,263 ns/iter (+/- 31,473)
test bench_refresh_process       ... bench:       4,009 ns/iter (+/- 584)
test bench_refresh_processes     ... bench:     790,834 ns/iter (+/- 61,236)
test bench_refresh_system        ... bench:     335,144 ns/iter (+/- 35,713)
test bench_refresh_temperatures  ... bench:     298,823 ns/iter (+/- 77,589)
```
</details>

## Donations

If you appreciate my work and want to support me, you can do it here:

[![Become a patron](https://c5.patreon.com/external/logo/become_a_patron_button.png)](https://www.patreon.com/GuillaumeGomez)
