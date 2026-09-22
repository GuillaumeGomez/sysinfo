# Linux Process GPU Refresh: Performance Investigation Notes

## Scope

The area under investigation is the Linux-specific GPU module inside `src/unix/linux/process.rs`, particularly `gpu::compute_gpu_usage`.

This module is an optional part of the wider process-refresh pipeline. It runs for each selected process when GPU usage or GPU memory has been requested.

## Relevant data flow

The shortened flow is:

```text
System::refresh_processes_specifics
    ↓
refresh_procs
    ↓ iterate through every selected PID
_get_process_data
    ↓
create or update ProcessInner
    ↓
update_proc_info
    ↓ when GPU information was requested
gpu::compute_gpu_usage
    ↓
inspect the process's file descriptors
    ↓
calculate and store GPU usage and memory in ProcessInner.gpu_info
```

For a process with PID `1234`, the GPU code works with two matching virtual directories:

```text
/proc/1234/fd
/proc/1234/fdinfo
```

The entries are connected by their file descriptor number. For example:

```text
/proc/1234/fd/9       → /dev/dri/renderD128
/proc/1234/fdinfo/9   → accounting information for descriptor 9
```

The GPU module:

1. Lists the descriptor names in `/proc/<pid>/fdinfo`.
2. For each descriptor, resolves the corresponding link in `/proc/<pid>/fd`.
3. Skips it when the target is not under `/dev/dri/` or `/dev/accel/`.
4. For a GPU descriptor, reads the matching `/proc/<pid>/fdinfo/<fd>` file.
5. Parses and totals the reported GPU engine time and GPU memory.
6. Stores the result in that process's `GpuInfo`.

After all requested PIDs are handled, newly discovered processes are inserted into the system's process map, existing processes have already been updated in place, and `refresh_procs` returns the number of successfully refreshed entries.

## Likely performance bottleneck

The leading hypothesis is the need to resolve every process's file descriptor link with `readlinkat`, even though most descriptors are not GPU-related.

The approximate work is:

```text
number of processes × average descriptors per process
```

For example, 500 processes with an average of 20 descriptors could require roughly 10,000 `readlinkat` calls. Most calls may only establish that a descriptor is not connected to a GPU.

Filtering therefore happens only after the potentially expensive operation:

```text
resolve descriptor link
    ↓
is it a GPU device?
    ├── no: discard it
    └── yes: read and parse its fdinfo
```

This is not necessarily an `O(n²)` algorithm. It is better described as proportional to the total number of descriptors across all inspected processes.

This is currently a hypothesis, not a proven conclusion. Profiling must establish whether `readlinkat` dominates the runtime or whether directory enumeration, file opening, reading, parsing, permission failures, or another operation is more important.

## Investigation plan

Now that the issue and suspected path are understood, the plan is:

1. Set up a representative Linux environment with a GPU driver that exposes DRM accounting fields through `/proc/<pid>/fdinfo`.
2. Establish a repeatable baseline using the maintainer's reproduction.
3. Run the benchmark several times and document the timings and test conditions.
4. Profile the reproduction several times and document syscall counts and time distribution.
5. Compare results across meaningful workloads, including an idle desktop, active GPU use, and many non-GPU descriptors where practical.
6. Use the evidence to identify the actual bottleneck.
7. Research possible solutions based on that conclusion.
8. Compare the options for performance, correctness, portability, complexity, and maintainability.
9. Implement one change at a time and rerun the same benchmark and correctness checks.

Caching known GPU descriptors has been discussed, but it presents a significant correctness risk: descriptors can close, be reused, or be newly opened between refreshes. It should not be selected unless stale or delayed discovery is explicitly acceptable, or a reliable invalidation strategy is found.

## Linux testing constraint

The development machine is a Mac. A generic Linux VM can test compilation and normal `/proc` behavior, but it may not provide representative Linux DRM GPU data. Meaningful measurements will likely require a physical or remote Linux machine with a supported GPU and visible `/dev/dri` or `/dev/accel` devices.

The environment and every recorded result should include at least:

- Commit hash
- Linux kernel version
- CPU and GPU model
- GPU driver
- Process count
- Descriptor count, where available
- Number of GPU descriptors found
- Workload state
- Benchmark timing and variation
- Profile or syscall summary

## Maintainer-provided reproduction

```rust
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

fn main() {
    let mut sys = System::new().unwrap();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        false,
        ProcessRefreshKind::nothing()
            .with_gpu_usage()
            .with_gpu_memory(),
    );
    println!("{}", sys.processes().len());
}
```

This reproduction requests GPU usage and GPU memory for all processes. It is the starting point for both benchmarking and profiling.

## Benchmarking and profiling direction

A benchmark answers: **How long does the operation take?**

A profiler answers: **Where is that time spent?**

The initial measurements should use an optimized release build and execute the compiled binary directly rather than including `cargo run` overhead. Useful Linux tools include:

- Hyperfine or an equivalent repeatable timing harness for total runtime.
- `strace -c` for syscall counts and aggregate syscall time, particularly `readlinkat`, `openat`, `read`, `getdents64`, and `close`.
- `perf stat` for repeated high-level performance counters.
- `perf record` and `perf report` for deeper function-level profiling, using a repeated-refresh harness if the single refresh finishes too quickly to sample reliably.

The first-refresh case from the maintainer's reproduction and repeated refreshes on the same `System` should be measured separately, because they represent different real-world usage patterns.
