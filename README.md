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

### Benchmarks

You can run the benchmarks locally with rust **nightly** by doing:

```bash
> cargo bench
```

Here are the current results:

**Linux**

<details>

```text
test bench_new                  ... bench:  10,437,759 ns/iter (+/- 531,424)
test bench_refresh_all          ... bench:   2,658,946 ns/iter (+/- 189,612)
test bench_refresh_cpu          ... bench:      13,429 ns/iter (+/- 537)
test bench_refresh_disk_lists   ... bench:      50,688 ns/iter (+/- 8,032)
test bench_refresh_disks        ... bench:       2,582 ns/iter (+/- 226)
test bench_refresh_memory       ... bench:      12,015 ns/iter (+/- 537)
test bench_refresh_network      ... bench:      23,661 ns/iter (+/- 617)
test bench_refresh_process      ... bench:      56,157 ns/iter (+/- 2,445)
test bench_refresh_processes    ... bench:   2,486,534 ns/iter (+/- 121,187)
test bench_refresh_system       ... bench:      53,739 ns/iter (+/- 6,793)
test bench_refresh_temperatures ... bench:      25,770 ns/iter (+/- 1,164)
```
</details>

**Windows**

<details>

```text
test bench_new                  ... bench:  61,548,071 ns/iter (+/- 196,093,742)
test bench_refresh_all          ... bench:   2,541,951 ns/iter (+/- 482,285)
test bench_refresh_cpu          ... bench:         460 ns/iter (+/- 478)
test bench_refresh_disk_lists   ... bench:     152,940 ns/iter (+/- 8,330)
test bench_refresh_disks        ... bench:      55,597 ns/iter (+/- 9,629)
test bench_refresh_memory       ... bench:       2,130 ns/iter (+/- 486)
test bench_refresh_network      ... bench:         212 ns/iter (+/- 216)
test bench_refresh_process      ... bench:          38 ns/iter (+/- 33)
test bench_refresh_processes    ... bench:   2,175,034 ns/iter (+/- 315,585)
test bench_refresh_system       ... bench:       2,508 ns/iter (+/- 224)
test bench_refresh_temperatures ... bench:           1 ns/iter (+/- 0)
```
</details>

**OSX**

<details>

```text
test bench_new                  ... bench:  64,062,254 ns/iter (+/- 8,845,126)
test bench_refresh_all          ... bench:   4,285,670 ns/iter (+/- 467,963)
test bench_refresh_cpu          ... bench:      10,712 ns/iter (+/- 1,493)
test bench_refresh_disk_lists   ... bench:     837,489 ns/iter (+/- 48,593)
test bench_refresh_disks        ... bench:         956 ns/iter (+/- 128)
test bench_refresh_memory       ... bench:       3,327 ns/iter (+/- 462)
test bench_refresh_network      ... bench:      34,465 ns/iter (+/- 5,228)
test bench_refresh_process      ... bench:       3,935 ns/iter (+/- 1,135)
test bench_refresh_processes    ... bench:   2,489,203 ns/iter (+/- 140,567)
test bench_refresh_system       ... bench:     741,774 ns/iter (+/- 335,431)
test bench_refresh_temperatures ... bench:     680,362 ns/iter (+/- 167,343)
```
</details>

## Donations

If you appreciate my work and want to support me, you can do it here:

[![Become a patron](https://c5.patreon.com/external/logo/become_a_patron_button.png)](https://www.patreon.com/GuillaumeGomez)
