# Migration guide

## 0.34 to 0.35

### Major changes

`Process::open_files*` methods now return a `usize`.

The `set_open_files_limit` function argument type is now `usize`.

### New APIs

`System` has a new `open_files_limit` method.

`Process` has two new methods: `kill_and_wait` and `kill_with_and_wait`.

## 0.33 to 0.34

### Major changes

The `multithread` feature is now disabled by default. It is to reduce the issues linked to the `rayon` crate usage.

The `System::physical_core_count` method was turned into an associated function. No need to have a `System` instance to get this information anymore.

The `System::refresh_all` and `System::refresh_specifics` methods now remove dead processes. If you don't want the dead processes to be removed, you will need to use `System::refresh_processes_specifics` directly instead.

### New APIs

The `Process` type has new methods: `accumulated_cpu_time`, `exists`, `open_files` and `open_files_limit`.

The `Process::wait` method now returns `Option<ExitStatus>` instead of `ExitStatus`.

The `System` type has new methods: `distribution_id_like` and `kernel_long_version`.

The `ProcessRefreshKind` type has a new "refresh kind": `tasks`.

## 0.32 to 0.33

### Major changes

`Users::refresh_list`, `Groups::refresh_list`, `Components::refresh_list`, `Networks::refresh_list` and `Disks::refresh_list` methods were renamed into `refresh`. All of them except for `Users::refresh` and `Groups::refresh` expect a boolean to tell whether or not `sysinfo` should keep removed items.

`Component::temperature` and `Component::max` now returns `Option<f32>` instead of returning `f32::NaN` in case the information isn't available.

`*RefreshKind::new` methods were renamed `nothing` to better the match the `*RefreshKind::everything` method.

### New APIs

`Disks` now has a new `refresh_specifics` method expecting a `DiskRefreshKind` argument to allow you finer-grained refreshes.

The `NetworkData` type now has a new `mtu` method to retrieve the Maximum Transfer Unit.

## 0.31 to 0.32

### Major changes

`System::refresh_process` and `System::refresh_process_specifics` methods now take
an extra `remove_dead_processes` argument. When set to `true`, dead processes will
be removed.

## 0.30 to 0.31

With this update, the minimum supported Rust version goes up to 1.74.

### Major changes

`System::refresh_process`, `System::refresh_process_specifics` and `System::refresh_pids`
methods were removed. The `System::refresh_processes` and `System::refresh_processes_specifics`
methods take a new argument of type `ProcessesToUpdate`.

The equivalent of `System::refresh_process`, `System::refresh_process_specifics` and
`System::refresh_pids` looks like this:

```rust
use sysinfo::{ProcessesToUpdate, System};

let pid = 1337;
let mut s = System::new();
s.refresh_processes(ProcessesToUpdate::Some(&[pid.into()]));
```

The equivalent of `System::refresh_processes` and `System::refresh_processes_specifics` looks
like this:

```rust
use sysinfo::{ProcessesToUpdate, System};

let mut s = System::new();
s.refresh_processes(ProcessesToUpdate::All);
```

#### Global CPU usage

`System::global_cpu_info` was replaced with `System::global_cpu_usage` which returns an `f32`
representing the global CPU usage and no other information.

#### Features

You can now enable/disable parts of `sysinfo` API through its cargo features to have
smaller build (size and time). If you're only interested by network information, then
you'll import `sysinfo` like this:

```toml
sysinfo = { version = "0.31", default-features = false, features = ["network"] }
```

#### Renaming

The `TermalSensorType` type was renamed into `ThermalSensorType`.

## 0.29 to 0.30

With this update, the minimum supported Rust version goes up to 1.69.

### Major changes

There are two major changes in this update. The first one was that all the traits were removed.
It means that now, if you want to use methods on `System`, you don't need to import `SystemExt`
anymore.

So before you had:

```rust
use sysinfo::{System, SystemExt};

// `SystemExt` is needed for both `new` and `refresh_processes`.
let mut s = System::new();
s.refresh_processes();
```

And now you have:

```rust
use sysinfo::System;

// No need for `SystemExt` anymore!
let s = System::new();
s.refresh_processes();
```

The second major change was that the `System` type has been split into smaller types:
 * `Components`
 * `Disks`
 * `Networks`
 * `Users`

The `System` type itself still handles CPU, memory and processes.

### Finer control over what is refreshed

The `*RefreshKind` types now have many more options allowing you to control exactly what is
retrieved. In particular, the `ProcessRefreshKind` now allows you to refresh specifically:
 * `cmd`
 * `cpu`
 * `disk_usage`
 * `environ`
 * `exe`
 * `memory`
 * `root`
 * `user`

In some cases, like `user`, you might want this information to be retrieved only if it hasn't been
already. For them, a new `UpdateKind` enum was added. It contains three variants:
 * `Never`
 * `Always`
 * `OnlyIfNotSet`

Like that, you get yet another extra level of control over what's updated and when.

### Constants in `System` have been moved to crate level

`System::IS_SUPPORTED` is now `sysinfo::IS_SUPPORTED_SYSTEM`.
`System::SUPPORTED_SIGNALS` is now `sysinfo::SUPPORTED_SIGNALS`.
`System::MINIMUM_CPU_UPDATE_INTERVAL` is now `sysinfo::MINIMUM_CPU_UPDATE_INTERVAL`.

### `System` changes

`System::refresh_pids` and `System::refresh_pids_specifics` methods have been added. They allow you
to be able to refresh multiple PIDs while being able to have support for `sysinfo` multi-threading
context (and much better performance in any case even if disabled).

Some methods are now static methods:
 * `boot_time`
 * `cpu_arch`
 * `distribution_id`
 * `host_name`
 * `kernel_version`
 * `load_average`
 * `long_os_version`
 * `name`
 * `os_version`
 * `uptime`

Meaning you can call them without having an instance of `System`:

```rust
println!("host name: {}", System::host_name());
```

A new `System::refresh_memory_specifics` method and a new `MemoryRefreshKind` type were added,
allowing you to control whether you want both RAM and SWAP memories to be updated or only one of
the two. This change was needed because getting SWAP information on Windows is very slow.

`System::tasks` method is now available on all OSes even if it only returns something on Linux. Its
return type is now a `Option<HashSet<Pid>>` instead of `HashMap<Pid, Process>`. The tasks are listed
in `processes`.

### `Disk` changes

`Disk::name` and `Disk::file_system` now returns `&OsStr`.

### cgroups handling

Before, `sysinfo` was handling cgroups internally and the users had no control over it. Now there is
a `System::cgroup_limits` method which allows you to query this information if you need it.

### `Process` changes

`Process::cwd`, `Process::exe` and `Process::root` now return an `Option<&Path>`.

### Removal of `sort_by` methods

If you want to sort `Disks`, `Users` or `Components`, you can do it by calling `sort` (or
equivalents) on the value returned by the `list_mut` methods.

### New `linux-netdevs` feature

By default, `sysinfo` excludes network devices because they can make the retrieval hangs
indefinitely. If you still want to get network devices knowing this risk, you can enable this
feature.

### `Cpu` changes

Information like `Cpu::brand`, `Cpu::vendor_id` or `Cpu::frequency` are not set on the "global" CPU.

## CHANGELOG

If you want the full list of changes, take a look at the
[CHANGELOG](https://github.com/GuillaumeGomez/sysinfo/blob/master/CHANGELOG.md).
