// Take a look at the license at the top of the repository in the LICENSE file.

use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::Path;
use std::process::ExitStatus;
use std::str::FromStr;

use crate::common::impl_get_set::impl_get_set;
use crate::common::DiskUsage;
use crate::{CpuInner, Gid, ProcessInner, SystemInner, Uid};

/// Structs containing system's information such as processes, memory and CPU.
///
/// ```
/// use sysinfo::System;
///
/// if sysinfo::IS_SUPPORTED_SYSTEM {
///     println!("System: {:?}", System::new_all());
/// } else {
///     println!("This OS isn't supported (yet?).");
/// }
/// ```
pub struct System {
    pub(crate) inner: SystemInner,
}

impl Default for System {
    fn default() -> System {
        System::new()
    }
}

impl System {
    /// Creates a new [`System`] instance with nothing loaded.
    ///
    /// Use one of the refresh methods (like [`refresh_all`]) to update its internal information.
    ///
    /// [`System`]: crate::System
    /// [`refresh_all`]: #method.refresh_all
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new();
    /// ```
    pub fn new() -> Self {
        Self::new_with_specifics(RefreshKind::nothing())
    }

    /// Creates a new [`System`] instance with everything loaded.
    ///
    /// It is an equivalent of [`System::new_with_specifics`]`(`[`RefreshKind::everything`]`())`.
    ///
    /// [`System`]: crate::System
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// ```
    pub fn new_all() -> Self {
        Self::new_with_specifics(RefreshKind::everything())
    }

    /// Creates a new [`System`] instance and refresh the data corresponding to the
    /// given [`RefreshKind`].
    ///
    /// [`System`]: crate::System
    ///
    /// ```
    /// use sysinfo::{ProcessRefreshKind, RefreshKind, System};
    ///
    /// // We want to only refresh processes.
    /// let mut system = System::new_with_specifics(
    ///      RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
    /// );
    ///
    /// # if sysinfo::IS_SUPPORTED_SYSTEM && !cfg!(feature = "apple-sandbox") {
    /// assert!(!system.processes().is_empty());
    /// # }
    /// ```
    pub fn new_with_specifics(refreshes: RefreshKind) -> Self {
        let mut s = Self {
            inner: SystemInner::new(),
        };
        s.refresh_specifics(refreshes);
        s
    }

    /// Refreshes according to the given [`RefreshKind`]. It calls the corresponding
    /// "refresh_" methods.
    ///
    /// It will remove dead processes if [`RefreshKind::processes`] returns `Some`.
    /// If you want to keep dead processes, use [`System::refresh_processes_specifics`]
    /// directly.
    ///
    /// ```
    /// use sysinfo::{ProcessRefreshKind, RefreshKind, System};
    ///
    /// let mut s = System::new_all();
    ///
    /// // Let's just update processes:
    /// s.refresh_specifics(
    ///     RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
    /// );
    /// ```
    pub fn refresh_specifics(&mut self, refreshes: RefreshKind) {
        if let Some(kind) = refreshes.memory() {
            self.refresh_memory_specifics(kind);
        }
        if let Some(kind) = refreshes.cpu() {
            self.refresh_cpu_specifics(kind);
        }
        if let Some(kind) = refreshes.processes() {
            self.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
        }
    }

    /// Refreshes all system and processes information.
    ///
    /// It is the same as calling `system.refresh_specifics(RefreshKind::everything())`.
    ///
    /// Don't forget to take a look at [`ProcessRefreshKind::everything`] method to see what it
    /// will update for processes more in details.
    ///
    /// It will remove dead processes. If you want to keep dead processes, use
    /// [`System::refresh_processes_specifics`] directly.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let mut s = System::new();
    /// s.refresh_all();
    /// ```
    pub fn refresh_all(&mut self) {
        self.refresh_specifics(RefreshKind::everything());
    }

    /// Refreshes RAM and SWAP usage.
    ///
    /// It is the same as calling `system.refresh_memory_specifics(MemoryRefreshKind::everything())`.
    ///
    /// If you don't want to refresh both, take a look at [`System::refresh_memory_specifics`].
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let mut s = System::new();
    /// s.refresh_memory();
    /// ```
    pub fn refresh_memory(&mut self) {
        self.refresh_memory_specifics(MemoryRefreshKind::everything())
    }

    /// Refreshes system memory specific information.
    ///
    /// ```no_run
    /// use sysinfo::{MemoryRefreshKind, System};
    ///
    /// let mut s = System::new();
    /// s.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    /// ```
    pub fn refresh_memory_specifics(&mut self, refresh_kind: MemoryRefreshKind) {
        self.inner.refresh_memory_specifics(refresh_kind)
    }

    /// Refreshes CPUs usage.
    ///
    /// ⚠️ Please note that the result will very likely be inaccurate at the first call.
    /// You need to call this method at least twice (with a bit of time between each call, like
    /// 200 ms, take a look at [`MINIMUM_CPU_UPDATE_INTERVAL`] for more information)
    /// to get accurate value as it uses previous results to compute the next value.
    ///
    /// Calling this method is the same as calling
    /// `system.refresh_cpu_specifics(CpuRefreshKind::nothing().with_cpu_usage())`.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let mut s = System::new_all();
    /// // Wait a bit because CPU usage is based on diff.
    /// std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    /// // Refresh CPUs again.
    /// s.refresh_cpu_usage();
    /// ```
    ///
    /// [`MINIMUM_CPU_UPDATE_INTERVAL`]: crate::MINIMUM_CPU_UPDATE_INTERVAL
    pub fn refresh_cpu_usage(&mut self) {
        self.refresh_cpu_specifics(CpuRefreshKind::nothing().with_cpu_usage())
    }

    /// Refreshes CPUs frequency information.
    ///
    /// Calling this method is the same as calling
    /// `system.refresh_cpu_specifics(CpuRefreshKind::nothing().with_frequency())`.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let mut s = System::new_all();
    /// s.refresh_cpu_frequency();
    /// ```
    pub fn refresh_cpu_frequency(&mut self) {
        self.refresh_cpu_specifics(CpuRefreshKind::nothing().with_frequency())
    }

    /// Refreshes the list of CPU.
    ///
    /// Normally, this should almost never be needed as it's pretty rare for a computer
    /// to add a CPU while running, but it's possible on some computers which shutdown
    /// CPU if the load is low enough.
    ///
    /// The `refresh_kind` argument tells what information you want to be retrieved
    /// for each CPU.
    ///
    /// ```no_run
    /// use sysinfo::{CpuRefreshKind, System};
    ///
    /// let mut s = System::new_all();
    /// // We already have the list of CPU filled, but we want to recompute it
    /// // in case new CPUs were added.
    /// s.refresh_cpu_list(CpuRefreshKind::everything());
    /// ```
    pub fn refresh_cpu_list(&mut self, refresh_kind: CpuRefreshKind) {
        self.inner.refresh_cpu_list(refresh_kind);
    }

    /// Refreshes all information related to CPUs information.
    ///
    /// If you only want the CPU usage, use [`System::refresh_cpu_usage`] instead.
    ///
    /// ⚠️ Please note that the result will be inaccurate at the first call.
    /// You need to call this method at least twice (with a bit of time between each call, like
    /// 200 ms, take a look at [`MINIMUM_CPU_UPDATE_INTERVAL`] for more information)
    /// to get accurate value as it uses previous results to compute the next value.
    ///
    /// Calling this method is the same as calling
    /// `system.refresh_cpu_specifics(CpuRefreshKind::everything())`.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let mut s = System::new_all();
    /// s.refresh_cpu_all();
    /// ```
    ///
    /// [`MINIMUM_CPU_UPDATE_INTERVAL`]: crate::MINIMUM_CPU_UPDATE_INTERVAL
    pub fn refresh_cpu_all(&mut self) {
        self.refresh_cpu_specifics(CpuRefreshKind::everything())
    }

    /// Refreshes CPUs specific information.
    ///
    /// ```no_run
    /// use sysinfo::{System, CpuRefreshKind};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_cpu_specifics(CpuRefreshKind::everything());
    /// ```
    pub fn refresh_cpu_specifics(&mut self, refresh_kind: CpuRefreshKind) {
        self.inner.refresh_cpu_specifics(refresh_kind)
    }

    /// Gets all processes and updates their information, along with all the tasks each process has.
    ///
    /// It does the same as:
    ///
    /// ```no_run
    /// # use sysinfo::{ProcessesToUpdate, ProcessRefreshKind, System, UpdateKind};
    /// # let mut system = System::new();
    /// system.refresh_processes_specifics(
    ///     ProcessesToUpdate::All,
    ///     true,
    ///     ProcessRefreshKind::nothing()
    ///         .with_memory()
    ///         .with_cpu()
    ///         .with_disk_usage()
    ///         .with_exe(UpdateKind::OnlyIfNotSet)
    /// );
    /// ```
    ///
    /// ⚠️ `remove_dead_processes` works as follows: if an updated process is dead, then it is
    /// removed. So if you refresh pids 1, 2 and 3. If 2 and 7 are dead, only 2 will be removed
    /// since 7 is not part of the update.
    ///
    /// ⚠️ On Linux, `sysinfo` keeps the `stat` files open by default. You can change this behaviour
    /// by using [`set_open_files_limit`][crate::set_open_files_limit].
    ///
    /// ⚠️ On Linux, if you dont need the tasks of each process, you can use
    /// `refresh_processes_specifics` with `ProcessRefreshKind::everything().without_tasks()`.
    /// Refreshesing all processes and their tasks can be quite expensive. For more information
    /// see [`ProcessRefreshKind`].
    ///
    /// Example:
    ///
    /// ```no_run
    /// use sysinfo::{ProcessesToUpdate, System};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_processes(ProcessesToUpdate::All, true);
    /// ```
    pub fn refresh_processes(
        &mut self,
        processes_to_update: ProcessesToUpdate<'_>,
        remove_dead_processes: bool,
    ) -> usize {
        self.refresh_processes_specifics(
            processes_to_update,
            remove_dead_processes,
            ProcessRefreshKind::nothing()
                .with_memory()
                .with_cpu()
                .with_disk_usage()
                .with_exe(UpdateKind::OnlyIfNotSet)
                .with_tasks(),
        )
    }

    /// Gets all processes and updates the specified information.
    ///
    /// Returns the number of updated processes.
    ///
    /// ⚠️ `remove_dead_processes` works as follows: if an updated process is dead, then it is
    /// removed. So if you refresh pids 1, 2 and 3. If 2 and 7 are dead, only 2 will be removed
    /// since 7 is not part of the update.
    ///
    /// ⚠️ On Linux, `sysinfo` keeps the `stat` files open by default. You can change this behaviour
    /// by using [`set_open_files_limit`][crate::set_open_files_limit].
    ///
    /// ```no_run
    /// use sysinfo::{ProcessesToUpdate, ProcessRefreshKind, System};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_processes_specifics(
    ///     ProcessesToUpdate::All,
    ///     true,
    ///     ProcessRefreshKind::everything(),
    /// );
    /// ```
    pub fn refresh_processes_specifics(
        &mut self,
        processes_to_update: ProcessesToUpdate<'_>,
        remove_dead_processes: bool,
        refresh_kind: ProcessRefreshKind,
    ) -> usize {
        fn update_and_remove(pid: &Pid, processes: &mut HashMap<Pid, Process>) {
            let updated = if let Some(proc) = processes.get_mut(pid) {
                proc.inner.switch_updated()
            } else {
                return;
            };
            if !updated {
                processes.remove(pid);
            }
        }
        fn update(pid: &Pid, processes: &mut HashMap<Pid, Process>) {
            if let Some(proc) = processes.get_mut(pid) {
                proc.inner.switch_updated();
            }
        }

        let nb_updated = self
            .inner
            .refresh_processes_specifics(processes_to_update, refresh_kind);
        let processes = self.inner.processes_mut();
        match processes_to_update {
            ProcessesToUpdate::All => {
                if remove_dead_processes {
                    processes.retain(|_, v| v.inner.switch_updated());
                } else {
                    for proc in processes.values_mut() {
                        proc.inner.switch_updated();
                    }
                }
            }
            ProcessesToUpdate::Some(pids) => {
                let call = if remove_dead_processes {
                    update_and_remove
                } else {
                    update
                };
                for pid in pids {
                    call(pid, processes);
                }
            }
        }
        nb_updated
    }

    /// Returns the process list.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// for (pid, process) in s.processes() {
    ///     println!("{} {:?}", pid, process.name());
    /// }
    /// ```
    pub fn processes(&self) -> &HashMap<Pid, Process> {
        self.inner.processes()
    }

    /// Returns the process corresponding to the given `pid` or `None` if no such process exists.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.name());
    /// }
    /// ```
    pub fn process(&self, pid: Pid) -> Option<&Process> {
        self.inner.process(pid)
    }

    /// Returns an iterator of process containing the given `name`.
    ///
    /// If you want only the processes with exactly the given `name`, take a look at
    /// [`System::processes_by_exact_name`].
    ///
    /// **⚠️ Important ⚠️**
    ///
    /// On **Linux**, there are two things to know about processes' name:
    ///  1. It is limited to 15 characters.
    ///  2. It is not always the exe name.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// for process in s.processes_by_name("htop".as_ref()) {
    ///     println!("{} {:?}", process.pid(), process.name());
    /// }
    /// ```
    pub fn processes_by_name<'a: 'b, 'b>(
        &'a self,
        name: &'b OsStr,
    ) -> impl Iterator<Item = &'a Process> + 'b {
        let finder = memchr::memmem::Finder::new(name.as_encoded_bytes());
        self.processes()
            .values()
            .filter(move |val: &&Process| finder.find(val.name().as_encoded_bytes()).is_some())
    }

    /// Returns an iterator of processes with exactly the given `name`.
    ///
    /// If you instead want the processes containing `name`, take a look at
    /// [`System::processes_by_name`].
    ///
    /// **⚠️ Important ⚠️**
    ///
    /// On **Linux**, there are two things to know about processes' name:
    ///  1. It is limited to 15 characters.
    ///  2. It is not always the exe name.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// for process in s.processes_by_exact_name("htop".as_ref()) {
    ///     println!("{} {:?}", process.pid(), process.name());
    /// }
    /// ```
    pub fn processes_by_exact_name<'a: 'b, 'b>(
        &'a self,
        name: &'b OsStr,
    ) -> impl Iterator<Item = &'a Process> + 'b {
        self.processes()
            .values()
            .filter(move |val: &&Process| val.name() == name)
    }

    /// Returns "global" CPUs usage (aka the addition of all the CPUs).
    ///
    /// To have up-to-date information, you need to call [`System::refresh_cpu_specifics`] or
    /// [`System::refresh_specifics`] with `cpu` enabled.
    ///
    /// ```no_run
    /// use sysinfo::{CpuRefreshKind, RefreshKind, System};
    ///
    /// let mut s = System::new_with_specifics(
    ///     RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// // Wait a bit because CPU usage is based on diff.
    /// std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    /// // Refresh CPUs again to get actual value.
    /// s.refresh_cpu_usage();
    /// println!("{}%", s.global_cpu_usage());
    /// ```
    pub fn global_cpu_usage(&self) -> f32 {
        self.inner.global_cpu_usage()
    }

    /// Returns the list of the CPUs.
    ///
    /// By default, the list of CPUs is empty until you call [`System::refresh_cpu_specifics`] or
    /// [`System::refresh_specifics`] with `cpu` enabled.
    ///
    /// ```no_run
    /// use sysinfo::{CpuRefreshKind, RefreshKind, System};
    ///
    /// let mut s = System::new_with_specifics(
    ///     RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// // Wait a bit because CPU usage is based on diff.
    /// std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    /// // Refresh CPUs again to get actual value.
    /// s.refresh_cpu_usage();
    /// for cpu in s.cpus() {
    ///     println!("{}%", cpu.cpu_usage());
    /// }
    /// ```
    pub fn cpus(&self) -> &[Cpu] {
        self.inner.cpus()
    }

    /// Returns the RAM size in bytes.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.total_memory());
    /// ```
    ///
    /// On Linux, if you want to see this information with the limit of your cgroup, take a look
    /// at [`cgroup_limits`](System::cgroup_limits).
    pub fn total_memory(&self) -> u64 {
        self.inner.total_memory()
    }

    /// Returns the amount of free RAM in bytes.
    ///
    /// Generally, "free" memory refers to unallocated memory whereas "available" memory refers to
    /// memory that is available for (re)use.
    ///
    /// Side note: Windows doesn't report "free" memory so this method returns the same value
    /// as [`available_memory`](System::available_memory).
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.free_memory());
    /// ```
    pub fn free_memory(&self) -> u64 {
        self.inner.free_memory()
    }

    /// Returns the amount of available RAM in bytes.
    ///
    /// Generally, "free" memory refers to unallocated memory whereas "available" memory refers to
    /// memory that is available for (re)use.
    ///
    /// ⚠️ Windows and FreeBSD don't report "available" memory so [`System::free_memory`]
    /// returns the same value as this method.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.available_memory());
    /// ```
    pub fn available_memory(&self) -> u64 {
        self.inner.available_memory()
    }

    /// Returns the amount of used RAM in bytes.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.used_memory());
    /// ```
    pub fn used_memory(&self) -> u64 {
        self.inner.used_memory()
    }

    /// Returns the SWAP size in bytes.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.total_swap());
    /// ```
    pub fn total_swap(&self) -> u64 {
        self.inner.total_swap()
    }

    /// Returns the amount of free SWAP in bytes.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.free_swap());
    /// ```
    pub fn free_swap(&self) -> u64 {
        self.inner.free_swap()
    }

    /// Returns the amount of used SWAP in bytes.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.used_swap());
    /// ```
    pub fn used_swap(&self) -> u64 {
        self.inner.used_swap()
    }

    /// Retrieves the limits for the current cgroup (if any), otherwise it returns `None`.
    ///
    /// This information is computed every time the method is called.
    ///
    /// ⚠️ You need to have run [`refresh_memory`](System::refresh_memory) at least once before
    /// calling this method.
    ///
    /// ⚠️ This method is only implemented for Linux. It always returns `None` for all other
    /// systems.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// println!("limits: {:?}", s.cgroup_limits());
    /// ```
    pub fn cgroup_limits(&self) -> Option<CGroupLimits> {
        self.inner.cgroup_limits()
    }

    /// Returns system uptime (in seconds).
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// println!("System running since {} seconds", System::uptime());
    /// ```
    pub fn uptime() -> u64 {
        SystemInner::uptime()
    }

    /// Returns the time (in seconds) when the system booted since UNIX epoch.
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// println!("System booted at {} seconds", System::boot_time());
    /// ```
    pub fn boot_time() -> u64 {
        SystemInner::boot_time()
    }

    /// Returns the system load average value.
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ⚠️ This is currently not working on **Windows**.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let load_avg = System::load_average();
    /// println!(
    ///     "one minute: {}%, five minutes: {}%, fifteen minutes: {}%",
    ///     load_avg.one,
    ///     load_avg.five,
    ///     load_avg.fifteen,
    /// );
    /// ```
    pub fn load_average() -> LoadAvg {
        SystemInner::load_average()
    }

    /// Returns the system name.
    ///
    /// | example platform | value of `System::name()` |
    /// |---|---|
    /// | linux laptop | "Ubuntu" |
    /// | android phone | "Pixel 9 Pro" |
    /// | apple laptop | "Darwin" |
    /// | windows server | "Windows" |
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// println!("OS: {:?}", System::name());
    /// ```
    pub fn name() -> Option<String> {
        SystemInner::name()
    }

    /// Returns the system's kernel version.
    ///
    /// | example platform | value of `System::kernel_version()` |
    /// |---|---|
    /// | linux laptop | "6.8.0-48-generic" |
    /// | android phone | "6.1.84-android14-11" |
    /// | apple laptop | "24.1.0" |
    /// | windows server | "20348" |
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// println!("kernel version: {:?}", System::kernel_version());
    /// ```
    pub fn kernel_version() -> Option<String> {
        SystemInner::kernel_version()
    }

    /// Returns the system version (e.g. for macOS this will return 15.1 rather than the kernel
    /// version).
    ///
    /// | example platform | value of `System::os_version()` |
    /// |---|---|
    /// | linux laptop | "24.04" |
    /// | android phone | "15" |
    /// | apple laptop | "15.1.1" |
    /// | windows server | "10 (20348)" |
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// println!("OS version: {:?}", System::os_version());
    /// ```
    pub fn os_version() -> Option<String> {
        SystemInner::os_version()
    }

    /// Returns the system long os version.
    ///
    /// | example platform | value of `System::long_os_version()` |
    /// |---|---|
    /// | linux laptop | "Linux (Ubuntu 24.04)" |
    /// | android phone | "Android 15 on Pixel 9 Pro" |
    /// | apple laptop | "macOS 15.1.1 Sequoia" |
    /// | windows server | "Windows Server 2022 Datacenter" |
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// println!("Long OS Version: {:?}", System::long_os_version());
    /// ```
    pub fn long_os_version() -> Option<String> {
        SystemInner::long_os_version()
    }

    /// Returns the distribution id as defined by os-release,
    /// or [`std::env::consts::OS`].
    ///
    /// See also
    /// - <https://www.freedesktop.org/software/systemd/man/os-release.html#ID=>
    /// - <https://doc.rust-lang.org/std/env/consts/constant.OS.html>
    ///
    /// | example platform | value of `System::distribution_id()` |
    /// |---|---|
    /// | linux laptop | "ubuntu" |
    /// | android phone | "android" |
    /// | apple laptop | "macos" |
    /// | windows server | "windows" |
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// println!("Distribution ID: {:?}", System::distribution_id());
    /// ```
    pub fn distribution_id() -> String {
        SystemInner::distribution_id()
    }

    /// Returns the distribution ids of operating systems that are closely
    /// related to the local operating system in regards to packaging and
    /// programming interfaces, for example listing one or more OS identifiers
    /// the local OS is a derivative from.
    ///
    /// See also
    /// - <https://www.freedesktop.org/software/systemd/man/latest/os-release.html#ID_LIKE=>
    ///
    /// | example platform | value of `System::distribution_id_like()` |
    /// |---|---|
    /// | android phone | [] |
    /// | archlinux laptop | [] |
    /// | centos server | ["rhel", "fedora"] |
    /// | ubuntu laptop | ["debian"] |
    /// | windows laptop | [] |
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// println!("Distribution ID_LIKE: {:?}", System::distribution_id_like());
    /// ```
    pub fn distribution_id_like() -> Vec<String> {
        SystemInner::distribution_id_like()
    }

    /// Returns the system hostname based off DNS.
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// println!("Hostname: {:?}", System::host_name());
    /// ```
    pub fn host_name() -> Option<String> {
        SystemInner::host_name()
    }

    /// Returns the CPU architecture (eg. x86, amd64, aarch64, ...).
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// println!("CPU Architecture: {:?}", System::cpu_arch());
    /// ```
    pub fn cpu_arch() -> String {
        SystemInner::cpu_arch().unwrap_or_else(|| std::env::consts::ARCH.to_owned())
    }

    /// Returns the number of physical cores on the CPU or `None` if it couldn't get it.
    ///
    /// In case there are multiple CPUs, it will combine the physical core count of all the CPUs.
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new();
    /// println!("{:?}", System::physical_core_count());
    /// ```
    pub fn physical_core_count() -> Option<usize> {
        SystemInner::physical_core_count()
    }
}

/// A struct representing system load average value.
///
/// It is returned by [`System::load_average`][crate::System::load_average].
///
/// ```no_run
/// use sysinfo::System;
///
/// let load_avg = System::load_average();
/// println!(
///     "one minute: {}%, five minutes: {}%, fifteen minutes: {}%",
///     load_avg.one,
///     load_avg.five,
///     load_avg.fifteen,
/// );
/// ```
#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct LoadAvg {
    /// Average load within one minute.
    pub one: f64,
    /// Average load within five minutes.
    pub five: f64,
    /// Average load within fifteen minutes.
    pub fifteen: f64,
}

/// An enum representing signals on UNIX-like systems.
///
/// On non-unix systems, this enum is mostly useless and is only there to keep coherency between
/// the different OSes.
///
/// If you want the list of the supported signals on the current system, use
/// [`SUPPORTED_SIGNALS`][crate::SUPPORTED_SIGNALS].
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum Signal {
    /// Hangup detected on controlling terminal or death of controlling process.
    Hangup,
    /// Interrupt from keyboard.
    Interrupt,
    /// Quit from keyboard.
    Quit,
    /// Illegal instruction.
    Illegal,
    /// Trace/breakpoint trap.
    Trap,
    /// Abort signal from C abort function.
    Abort,
    /// IOT trap. A synonym for SIGABRT.
    IOT,
    /// Bus error (bad memory access).
    Bus,
    /// Floating point exception.
    FloatingPointException,
    /// Kill signal.
    Kill,
    /// User-defined signal 1.
    User1,
    /// Invalid memory reference.
    Segv,
    /// User-defined signal 2.
    User2,
    /// Broken pipe: write to pipe with no readers.
    Pipe,
    /// Timer signal from C alarm function.
    Alarm,
    /// Termination signal.
    Term,
    /// Child stopped or terminated.
    Child,
    /// Continue if stopped.
    Continue,
    /// Stop process.
    Stop,
    /// Stop typed at terminal.
    TSTP,
    /// Terminal input for background process.
    TTIN,
    /// Terminal output for background process.
    TTOU,
    /// Urgent condition on socket.
    Urgent,
    /// CPU time limit exceeded.
    XCPU,
    /// File size limit exceeded.
    XFSZ,
    /// Virtual alarm clock.
    VirtualAlarm,
    /// Profiling time expired.
    Profiling,
    /// Windows resize signal.
    Winch,
    /// I/O now possible.
    IO,
    /// Pollable event (Sys V). Synonym for IO
    Poll,
    /// Power failure (System V).
    ///
    /// Doesn't exist on apple systems so will be ignored.
    Power,
    /// Bad argument to routine (SVr4).
    Sys,
}

impl std::fmt::Display for Signal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match *self {
            Self::Hangup => "Hangup",
            Self::Interrupt => "Interrupt",
            Self::Quit => "Quit",
            Self::Illegal => "Illegal",
            Self::Trap => "Trap",
            Self::Abort => "Abort",
            Self::IOT => "IOT",
            Self::Bus => "Bus",
            Self::FloatingPointException => "FloatingPointException",
            Self::Kill => "Kill",
            Self::User1 => "User1",
            Self::Segv => "Segv",
            Self::User2 => "User2",
            Self::Pipe => "Pipe",
            Self::Alarm => "Alarm",
            Self::Term => "Term",
            Self::Child => "Child",
            Self::Continue => "Continue",
            Self::Stop => "Stop",
            Self::TSTP => "TSTP",
            Self::TTIN => "TTIN",
            Self::TTOU => "TTOU",
            Self::Urgent => "Urgent",
            Self::XCPU => "XCPU",
            Self::XFSZ => "XFSZ",
            Self::VirtualAlarm => "VirtualAlarm",
            Self::Profiling => "Profiling",
            Self::Winch => "Winch",
            Self::IO => "IO",
            Self::Poll => "Poll",
            Self::Power => "Power",
            Self::Sys => "Sys",
        };
        f.write_str(s)
    }
}

/// Contains memory limits for the current process.
#[derive(Default, Debug, Clone)]
pub struct CGroupLimits {
    /// Total memory (in bytes) for the current cgroup.
    pub total_memory: u64,
    /// Free memory (in bytes) for the current cgroup.
    pub free_memory: u64,
    /// Free swap (in bytes) for the current cgroup.
    pub free_swap: u64,
    /// Resident Set Size (RSS) (in bytes) for the current cgroup.
    pub rss: u64,
}

/// Enum describing the different status of a process.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum ProcessStatus {
    /// ## Linux
    ///
    /// Idle kernel thread.
    ///
    /// ## macOs/FreeBSD
    ///
    /// Process being created by fork.
    ///
    /// ## Other OS
    ///
    /// Not available.
    Idle,
    /// Running.
    Run,
    /// ## Linux
    ///
    /// Sleeping in an interruptible waiting.
    ///
    /// ## macOS/FreeBSD
    ///
    /// Sleeping on an address.
    ///
    /// ## Other OS
    ///
    /// Not available.
    Sleep,
    /// ## Linux
    ///
    /// Stopped (on a signal) or (before Linux 2.6.33) trace stopped.
    ///
    /// ## macOS/FreeBSD
    ///
    /// Process debugging or suspension.
    ///
    /// ## Other OS
    ///
    /// Not available.
    Stop,
    /// ## Linux/FreeBSD/macOS
    ///
    /// Zombie process. Terminated but not reaped by its parent.
    ///
    /// ## Other OS
    ///
    /// Not available.
    Zombie,
    /// ## Linux
    ///
    /// Tracing stop (Linux 2.6.33 onward). Stopped by debugger during the tracing.
    ///
    /// ## Other OS
    ///
    /// Not available.
    Tracing,
    /// ## Linux
    ///
    /// Dead/uninterruptible sleep (usually IO).
    ///
    /// ## FreeBSD
    ///
    /// A process should never end up in this state.
    ///
    /// ## Other OS
    ///
    /// Not available.
    Dead,
    /// ## Linux
    ///
    /// Wakekill (Linux 2.6.33 to 3.13 only).
    ///
    /// ## Other OS
    ///
    /// Not available.
    Wakekill,
    /// ## Linux
    ///
    /// Waking (Linux 2.6.33 to 3.13 only).
    ///
    /// ## Other OS
    ///
    /// Not available.
    Waking,
    /// ## Linux
    ///
    /// Parked (Linux 3.9 to 3.13 only).
    ///
    /// ## macOS
    ///
    /// Halted at a clean point.
    ///
    /// ## Other OS
    ///
    /// Not available.
    Parked,
    /// ## FreeBSD
    ///
    /// Blocked on a lock.
    ///
    /// ## Other OS
    ///
    /// Not available.
    LockBlocked,
    /// ## Linux
    ///
    /// Waiting in uninterruptible disk sleep.
    ///
    /// ## Other OS
    ///
    /// Not available.
    UninterruptibleDiskSleep,
    /// Unknown.
    Unknown(u32),
}

/// Enum describing the different kind of threads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum ThreadKind {
    /// Kernel thread.
    Kernel,
    /// User thread.
    Userland,
}

/// Struct containing information of a process.
///
/// ## iOS
///
/// This information cannot be retrieved on iOS due to sandboxing.
///
/// ## Apple app store
///
/// If you are building a macOS Apple app store, it won't be able
/// to retrieve this information.
///
/// ```no_run
/// use sysinfo::{Pid, System};
///
/// let s = System::new_all();
/// if let Some(process) = s.process(Pid::from(1337)) {
///     println!("{:?}", process.name());
/// }
/// ```
pub struct Process {
    pub(crate) inner: ProcessInner,
}

impl Process {
    /// Sends [`Signal::Kill`] to the process (which is the only signal supported on all supported
    /// platforms by this crate).
    ///
    /// Returns `true` if the signal was sent successfully. If you want to wait for this process
    /// to end, you can use [`Process::wait`].
    ///
    /// ⚠️ Even if this function returns `true`, it doesn't necessarily mean that the process will
    /// be killed. It just means that the signal was sent successfully.
    ///
    /// ⚠️ Please note that some processes might not be "killable", like if they run with higher
    /// levels than the current process for example.
    ///
    /// If you want to use another signal, take a look at [`Process::kill_with`].
    ///
    /// To get the list of the supported signals on this system, use
    /// [`SUPPORTED_SIGNALS`][crate::SUPPORTED_SIGNALS].
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     process.kill();
    /// }
    /// ```
    pub fn kill(&self) -> bool {
        self.kill_with(Signal::Kill).unwrap_or(false)
    }

    /// Sends the given `signal` to the process. If the signal doesn't exist on this platform,
    /// it'll do nothing and will return `None`. Otherwise it'll return `Some(bool)`. The boolean
    /// value will depend on whether or not the signal was sent successfully.
    ///
    /// If you just want to kill the process, use [`Process::kill`] directly. If you want to wait
    /// for this process to end, you can use [`Process::wait`].
    ///
    /// ⚠️ Please note that some processes might not be "killable", like if they run with higher
    /// levels than the current process for example.
    ///
    /// To get the list of the supported signals on this system, use
    /// [`SUPPORTED_SIGNALS`][crate::SUPPORTED_SIGNALS].
    ///
    /// ```no_run
    /// use sysinfo::{Pid, Signal, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     if process.kill_with(Signal::Kill).is_none() {
    ///         println!("This signal isn't supported on this platform");
    ///     }
    /// }
    /// ```
    pub fn kill_with(&self, signal: Signal) -> Option<bool> {
        self.inner.kill_with(signal)
    }

    /// Wait for process termination and returns its [`ExitStatus`] if it could be retrieved,
    /// returns `None` otherwise.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("Waiting for pid 1337");
    ///     let exit_status = process.wait();
    ///     println!("Pid 1337 exited with: {exit_status:?}");
    /// }
    /// ```
    pub fn wait(&self) -> Option<ExitStatus> {
        self.inner.wait()
    }

    /// Returns the name of the process.
    ///
    /// **⚠️ Important ⚠️**
    ///
    /// On **Linux**, there are two things to know about processes' name:
    ///  1. It is limited to 15 characters.
    ///  2. It is not always the exe name.
    ///
    /// If you are looking for a specific process, unless you know what you are
    /// doing, in most cases it's better to use [`Process::exe`] instead (which
    /// can be empty sometimes!).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.name());
    /// }
    /// ```
    pub fn name(&self) -> &OsStr {
        self.inner.name()
    }

    /// Returns the command line.
    ///
    ///  **⚠️ Important ⚠️**
    ///
    /// On **Windows**, you might need to use `administrator` privileges when running your program
    /// to have access to this information.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.cmd());
    /// }
    /// ```
    pub fn cmd(&self) -> &[OsString] {
        self.inner.cmd()
    }

    /// Returns the path to the process.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.exe());
    /// }
    /// ```
    ///
    /// ### Implementation notes
    ///
    /// On Linux, this method will return an empty path if there
    /// was an error trying to read `/proc/<pid>/exe`. This can
    /// happen, for example, if the permission levels or UID namespaces
    /// between the caller and target processes are different.
    ///
    /// It is also the case that `cmd[0]` is _not_ usually a correct
    /// replacement for this.
    /// A process [may change its `cmd[0]` value](https://man7.org/linux/man-pages/man5/proc.5.html)
    /// freely, making this an untrustworthy source of information.
    pub fn exe(&self) -> Option<&Path> {
        self.inner.exe()
    }

    /// Returns the PID of the process.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}", process.pid());
    /// }
    /// ```
    pub fn pid(&self) -> Pid {
        self.inner.pid()
    }

    /// Returns the environment variables of the process.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.environ());
    /// }
    /// ```
    pub fn environ(&self) -> &[OsString] {
        self.inner.environ()
    }

    /// Returns the current working directory.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.cwd());
    /// }
    /// ```
    pub fn cwd(&self) -> Option<&Path> {
        self.inner.cwd()
    }

    /// Returns the path of the root directory.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.root());
    /// }
    /// ```
    pub fn root(&self) -> Option<&Path> {
        self.inner.root()
    }

    /// Returns the memory usage (in bytes).
    ///
    /// This method returns the [size of the resident set], that is, the amount of memory that the
    /// process allocated and which is currently mapped in physical RAM. It does not include memory
    /// that is swapped out, or, in some operating systems, that has been allocated but never used.
    ///
    /// Thus, it represents exactly the amount of physical RAM that the process is using at the
    /// present time, but it might not be a good indicator of the total memory that the process will
    /// be using over its lifetime. For that purpose, you can try and use
    /// [`virtual_memory`](Process::virtual_memory).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{} bytes", process.memory());
    /// }
    /// ```
    ///
    /// [size of the resident set]: https://en.wikipedia.org/wiki/Resident_set_size
    pub fn memory(&self) -> u64 {
        self.inner.memory()
    }

    /// Returns the virtual memory usage (in bytes).
    ///
    /// This method returns the [size of virtual memory], that is, the amount of memory that the
    /// process can access, whether it is currently mapped in physical RAM or not. It includes
    /// physical RAM, allocated but not used regions, swapped-out regions, and even memory
    /// associated with [memory-mapped files](https://en.wikipedia.org/wiki/Memory-mapped_file).
    ///
    /// This value has limitations though. Depending on the operating system and type of process,
    /// this value might be a good indicator of the total memory that the process will be using over
    /// its lifetime. However, for example, in the version 14 of macOS this value is in the order of
    /// the hundreds of gigabytes for every process, and thus not very informative. Moreover, if a
    /// process maps into memory a very large file, this value will increase accordingly, even if
    /// the process is not actively using the memory.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{} bytes", process.virtual_memory());
    /// }
    /// ```
    ///
    /// [size of virtual memory]: https://en.wikipedia.org/wiki/Virtual_memory
    pub fn virtual_memory(&self) -> u64 {
        self.inner.virtual_memory()
    }

    /// Returns the parent PID.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.parent());
    /// }
    /// ```
    pub fn parent(&self) -> Option<Pid> {
        self.inner.parent()
    }

    /// Returns the status of the process.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.status());
    /// }
    /// ```
    pub fn status(&self) -> ProcessStatus {
        self.inner.status()
    }

    /// Returns the time where the process was started (in seconds) from epoch.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("Started at {} seconds", process.start_time());
    /// }
    /// ```
    pub fn start_time(&self) -> u64 {
        self.inner.start_time()
    }

    /// Returns for how much time the process has been running (in seconds).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("Running since {} seconds", process.run_time());
    /// }
    /// ```
    pub fn run_time(&self) -> u64 {
        self.inner.run_time()
    }

    /// Returns the total CPU usage (in %). Notice that it might be bigger than
    /// 100 if run on a multi-core machine.
    ///
    /// If you want a value between 0% and 100%, divide the returned value by
    /// the number of CPUs.
    ///
    /// ⚠️ To start to have accurate CPU usage, a process needs to be refreshed
    /// **twice** because CPU usage computation is based on time diff (process
    /// time on a given time period divided by total system time on the same
    /// time period).
    ///
    /// ⚠️ If you want accurate CPU usage number, better leave a bit of time
    /// between two calls of this method (take a look at
    /// [`MINIMUM_CPU_UPDATE_INTERVAL`][crate::MINIMUM_CPU_UPDATE_INTERVAL] for
    /// more information).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessesToUpdate, ProcessRefreshKind, System};
    ///
    /// let mut s = System::new_all();
    /// // Wait a bit because CPU usage is based on diff.
    /// std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    /// // Refresh CPU usage to get actual value.
    /// s.refresh_processes_specifics(
    ///     ProcessesToUpdate::All,
    ///     true,
    ///     ProcessRefreshKind::nothing().with_cpu()
    /// );
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}%", process.cpu_usage());
    /// }
    /// ```
    pub fn cpu_usage(&self) -> f32 {
        self.inner.cpu_usage()
    }

    /// Returns the total accumulated CPU usage (in CPU-milliseconds). Note
    /// that it might be bigger than the total clock run time of a process if
    /// run on a multi-core machine.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}", process.accumulated_cpu_time());
    /// }
    /// ```
    pub fn accumulated_cpu_time(&self) -> u64 {
        self.inner.accumulated_cpu_time()
    }

    /// Returns number of bytes read and written to disk.
    ///
    /// ⚠️ On Windows, this method actually returns **ALL** I/O read and
    /// written bytes.
    ///
    /// ⚠️ Files might be cached in memory by your OS, meaning that reading/writing them might not
    /// increase the `read_bytes`/`written_bytes` values. You can find more information about it
    /// in the `proc_pid_io` manual (`man proc_pid_io` on unix platforms).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     let disk_usage = process.disk_usage();
    ///     println!("read bytes   : new/total => {}/{}",
    ///         disk_usage.read_bytes,
    ///         disk_usage.total_read_bytes,
    ///     );
    ///     println!("written bytes: new/total => {}/{}",
    ///         disk_usage.written_bytes,
    ///         disk_usage.total_written_bytes,
    ///     );
    /// }
    /// ```
    pub fn disk_usage(&self) -> DiskUsage {
        self.inner.disk_usage()
    }

    /// Returns the ID of the owner user of this process or `None` if this
    /// information couldn't be retrieved. If you want to get the [`User`] from
    /// it, take a look at [`Users::get_user_by_id`].
    ///
    /// [`User`]: crate::User
    /// [`Users::get_user_by_id`]: crate::Users::get_user_by_id
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("User id for process 1337: {:?}", process.user_id());
    /// }
    /// ```
    pub fn user_id(&self) -> Option<&Uid> {
        self.inner.user_id()
    }

    /// Returns the user ID of the effective owner of this process or `None` if
    /// this information couldn't be retrieved. If you want to get the [`User`]
    /// from it, take a look at [`Users::get_user_by_id`].
    ///
    /// If you run something with `sudo`, the real user ID of the launched
    /// process will be the ID of the user you are logged in as but effective
    /// user ID will be `0` (i-e root).
    ///
    /// ⚠️ It always returns `None` on Windows.
    ///
    /// [`User`]: crate::User
    /// [`Users::get_user_by_id`]: crate::Users::get_user_by_id
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("User id for process 1337: {:?}", process.effective_user_id());
    /// }
    /// ```
    pub fn effective_user_id(&self) -> Option<&Uid> {
        self.inner.effective_user_id()
    }

    /// Returns the process group ID of the process.
    ///
    /// ⚠️ It always returns `None` on Windows.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("Group ID for process 1337: {:?}", process.group_id());
    /// }
    /// ```
    pub fn group_id(&self) -> Option<Gid> {
        self.inner.group_id()
    }

    /// Returns the effective group ID of the process.
    ///
    /// If you run something with `sudo`, the real group ID of the launched
    /// process will be the primary group ID you are logged in as but effective
    /// group ID will be `0` (i-e root).
    ///
    /// ⚠️ It always returns `None` on Windows.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("User id for process 1337: {:?}", process.effective_group_id());
    /// }
    /// ```
    pub fn effective_group_id(&self) -> Option<Gid> {
        self.inner.effective_group_id()
    }

    /// Returns the session ID for the current process or `None` if it couldn't
    /// be retrieved.
    ///
    /// ⚠️ This information is computed every time this method is called.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("Session ID for process 1337: {:?}", process.session_id());
    /// }
    /// ```
    pub fn session_id(&self) -> Option<Pid> {
        self.inner.session_id()
    }

    /// Tasks run by this process. If there are none, returns `None`.
    ///
    /// ⚠️ This method always returns `None` on other platforms than Linux.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     if let Some(tasks) = process.tasks() {
    ///         println!("Listing tasks for process {:?}", process.pid());
    ///         for task_pid in tasks {
    ///             if let Some(task) = s.process(*task_pid) {
    ///                 println!("Task {:?}: {:?}", task.pid(), task.name());
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn tasks(&self) -> Option<&HashSet<Pid>> {
        cfg_if! {
            if #[cfg(all(
                any(target_os = "linux", target_os = "android"),
                not(feature = "unknown-ci")
            ))] {
                self.inner.tasks.as_ref()
            } else {
                None
            }
        }
    }

    /// If the process is a thread, it'll return `Some` with the kind of thread it is. Returns
    /// `None` otherwise.
    ///
    /// ⚠️ This method always returns `None` on other platforms than Linux.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    ///
    /// for (_, process) in s.processes() {
    ///     if let Some(thread_kind) = process.thread_kind() {
    ///         println!("Process {:?} is a {thread_kind:?} thread", process.pid());
    ///     }
    /// }
    /// ```
    pub fn thread_kind(&self) -> Option<ThreadKind> {
        cfg_if! {
            if #[cfg(all(
                any(target_os = "linux", target_os = "android"),
                not(feature = "unknown-ci")
            ))] {
                self.inner.thread_kind()
            } else {
                None
            }
        }
    }
}

macro_rules! pid_decl {
    ($typ:ty) => {
        #[doc = include_str!("../../md_doc/pid.md")]
        #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(transparent)]
        pub struct Pid(pub(crate) $typ);

        impl From<usize> for Pid {
            fn from(v: usize) -> Self {
                Self(v as _)
            }
        }
        impl From<Pid> for usize {
            fn from(v: Pid) -> Self {
                v.0 as _
            }
        }
        impl FromStr for Pid {
            type Err = <$typ as FromStr>::Err;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(<$typ>::from_str(s)?))
            }
        }
        impl fmt::Display for Pid {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        impl Pid {
            /// Allows to convert [`Pid`][crate::Pid] into [`u32`].
            ///
            /// ```
            /// use sysinfo::Pid;
            ///
            /// let pid = Pid::from_u32(0);
            /// let value: u32 = pid.as_u32();
            /// ```
            pub fn as_u32(self) -> u32 {
                self.0 as _
            }
            /// Allows to convert a [`u32`] into [`Pid`][crate::Pid].
            ///
            /// ```
            /// use sysinfo::Pid;
            ///
            /// let pid = Pid::from_u32(0);
            /// ```
            pub fn from_u32(v: u32) -> Self {
                Self(v as _)
            }
        }
    };
}

cfg_if! {
    if #[cfg(all(
        not(feature = "unknown-ci"),
        any(
            target_os = "freebsd",
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "ios",
        )
    ))] {
        use libc::pid_t;

        pid_decl!(pid_t);
    } else {
        pid_decl!(usize);
    }
}

/// This enum allows you to specify when you want the related information to be updated.
///
/// For example if you only want the [`Process::exe()`] information to be refreshed only if it's not
/// already set:
///
/// ```no_run
/// use sysinfo::{ProcessesToUpdate, ProcessRefreshKind, System, UpdateKind};
///
/// let mut system = System::new();
/// system.refresh_processes_specifics(
///     ProcessesToUpdate::All,
///     true,
///     ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
/// );
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UpdateKind {
    /// Never update the related information.
    #[default]
    Never,
    /// Always update the related information.
    Always,
    /// Only update the related information if it was not already set at least once.
    OnlyIfNotSet,
}

impl UpdateKind {
    /// If `self` is `OnlyIfNotSet`, `f` is called and its returned value is returned.
    #[allow(dead_code)] // Needed for unsupported targets.
    pub(crate) fn needs_update(self, f: impl Fn() -> bool) -> bool {
        match self {
            Self::Never => false,
            Self::Always => true,
            Self::OnlyIfNotSet => f(),
        }
    }
}

/// This enum allows you to specify if you want all processes to be updated or just
/// some of them.
///
/// Example:
///
/// ```no_run
/// use sysinfo::{ProcessesToUpdate, System, get_current_pid};
///
/// let mut system = System::new();
/// // To refresh all processes:
/// system.refresh_processes(ProcessesToUpdate::All, true);
///
/// // To refresh only the current one:
/// system.refresh_processes(
///     ProcessesToUpdate::Some(&[get_current_pid().unwrap()]),
///     true,
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessesToUpdate<'a> {
    /// To refresh all processes.
    All,
    /// To refresh only the processes with the listed [`Pid`].
    ///
    /// [`Pid`]: crate::Pid
    Some(&'a [Pid]),
}

/// Used to determine what you want to refresh specifically on the [`Process`] type.
///
/// When all refresh are ruled out, a [`Process`] will still retrieve the following information:
///  * Process ID ([`Pid`])
///  * Parent process ID (on Windows it never changes though)
///  * Process name
///  * Start time
///
/// ⚠️ Just like all other refresh types, ruling out a refresh doesn't assure you that
/// the information won't be retrieved if the information is accessible without needing
/// extra computation.
///
/// ⚠️ ** Linux Specific ** ⚠️
/// When using `ProcessRefreshKind::everything()`, in linux we will fetch all relevant
/// information from `/proc/<pid>/` as well as all the information from `/proc/<pid>/task/<tid>/`
/// folders. This makes the refresh mechanism a lot slower depending on the number of tasks
/// each process has.
///  
/// If you don't care about tasks information, use `ProcessRefreshKind::everything().without_tasks()`
/// as much as possible.
///
/// ```
/// use sysinfo::{ProcessesToUpdate, ProcessRefreshKind, System};
///
/// let mut system = System::new();
///
/// // We don't want to update the CPU information.
/// system.refresh_processes_specifics(
///     ProcessesToUpdate::All,
///     true,
///     ProcessRefreshKind::everything().without_cpu(),
/// );
///
/// for (_, proc_) in system.processes() {
///     // We use a `==` comparison on float only because we know it's set to 0 here.
///     assert_eq!(proc_.cpu_usage(), 0.);
/// }
/// ```
///
/// [`Process`]: crate::Process
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessRefreshKind {
    cpu: bool,
    disk_usage: bool,
    memory: bool,
    user: UpdateKind,
    cwd: UpdateKind,
    root: UpdateKind,
    environ: UpdateKind,
    cmd: UpdateKind,
    exe: UpdateKind,
    tasks: bool,
}

/// Creates a new `ProcessRefreshKind` with every refresh set to `false`, except for `tasks`.
/// By default, we want to list all processes and tasks are considered processes on their own
/// in linux so we still fetch them by default. However, the processes information are not
/// refreshed.
impl Default for ProcessRefreshKind {
    fn default() -> Self {
        Self {
            cpu: false,
            disk_usage: false,
            memory: false,
            user: UpdateKind::default(),
            cwd: UpdateKind::default(),
            root: UpdateKind::default(),
            environ: UpdateKind::default(),
            cmd: UpdateKind::default(),
            exe: UpdateKind::default(),
            tasks: true, // Process by default includes all tasks.
        }
    }
}

impl ProcessRefreshKind {
    /// Creates a new `ProcessRefreshKind` with every refresh set to `false`, except for `tasks`.
    /// By default, we want to list all processes and tasks are considered processes on their own
    /// in linux so we still fetch them by default. However, the processes information are not
    /// refreshed.
    /// ```
    /// use sysinfo::{ProcessRefreshKind, UpdateKind};
    ///
    /// let r = ProcessRefreshKind::nothing();
    ///
    /// assert_eq!(r.cpu(), false);
    /// assert_eq!(r.user(), UpdateKind::Never);
    /// ```
    pub fn nothing() -> Self {
        Self::default()
    }

    /// Creates a new `ProcessRefreshKind` with every refresh set to `true` or
    /// [`UpdateKind::OnlyIfNotSet`].
    ///
    /// ```
    /// use sysinfo::{ProcessRefreshKind, UpdateKind};
    ///
    /// let r = ProcessRefreshKind::everything();
    ///
    /// assert_eq!(r.cpu(), true);
    /// assert_eq!(r.user(), UpdateKind::OnlyIfNotSet);
    /// ```
    pub fn everything() -> Self {
        Self {
            cpu: true,
            disk_usage: true,
            memory: true,
            user: UpdateKind::OnlyIfNotSet,
            cwd: UpdateKind::OnlyIfNotSet,
            root: UpdateKind::OnlyIfNotSet,
            environ: UpdateKind::OnlyIfNotSet,
            cmd: UpdateKind::OnlyIfNotSet,
            exe: UpdateKind::OnlyIfNotSet,
            tasks: true,
        }
    }

    impl_get_set!(
        ProcessRefreshKind,
        cpu,
        with_cpu,
        without_cpu,
        "\
It will retrieve both CPU usage and CPU accumulated time,"
    );
    impl_get_set!(
        ProcessRefreshKind,
        disk_usage,
        with_disk_usage,
        without_disk_usage
    );
    impl_get_set!(
        ProcessRefreshKind,
        user,
        with_user,
        without_user,
        UpdateKind,
        "\
It will retrieve the following information:

 * user ID
 * user effective ID (if available on the platform)
 * user group ID (if available on the platform)
 * user effective ID (if available on the platform)"
    );
    impl_get_set!(ProcessRefreshKind, memory, with_memory, without_memory);
    impl_get_set!(ProcessRefreshKind, cwd, with_cwd, without_cwd, UpdateKind);
    impl_get_set!(
        ProcessRefreshKind,
        root,
        with_root,
        without_root,
        UpdateKind
    );
    impl_get_set!(
        ProcessRefreshKind,
        environ,
        with_environ,
        without_environ,
        UpdateKind
    );
    impl_get_set!(ProcessRefreshKind, cmd, with_cmd, without_cmd, UpdateKind);
    impl_get_set!(ProcessRefreshKind, exe, with_exe, without_exe, UpdateKind);
    impl_get_set!(ProcessRefreshKind, tasks, with_tasks, without_tasks);
}

/// Used to determine what you want to refresh specifically on the [`Cpu`] type.
///
/// ⚠️ Just like all other refresh types, ruling out a refresh doesn't assure you that
/// the information won't be retrieved if the information is accessible without needing
/// extra computation.
///
/// ```
/// use sysinfo::{CpuRefreshKind, System};
///
/// let mut system = System::new();
///
/// // We don't want to update all the CPU information.
/// system.refresh_cpu_specifics(CpuRefreshKind::everything().without_frequency());
///
/// for cpu in system.cpus() {
///     assert_eq!(cpu.frequency(), 0);
/// }
/// ```
///
/// [`Cpu`]: crate::Cpu
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CpuRefreshKind {
    cpu_usage: bool,
    frequency: bool,
}

impl CpuRefreshKind {
    /// Creates a new `CpuRefreshKind` with every refresh set to `false`.
    ///
    /// ```
    /// use sysinfo::CpuRefreshKind;
    ///
    /// let r = CpuRefreshKind::nothing();
    ///
    /// assert_eq!(r.frequency(), false);
    /// assert_eq!(r.cpu_usage(), false);
    /// ```
    pub fn nothing() -> Self {
        Self::default()
    }

    /// Creates a new `CpuRefreshKind` with every refresh set to `true`.
    ///
    /// ```
    /// use sysinfo::CpuRefreshKind;
    ///
    /// let r = CpuRefreshKind::everything();
    ///
    /// assert_eq!(r.frequency(), true);
    /// assert_eq!(r.cpu_usage(), true);
    /// ```
    pub fn everything() -> Self {
        Self {
            cpu_usage: true,
            frequency: true,
        }
    }

    impl_get_set!(CpuRefreshKind, cpu_usage, with_cpu_usage, without_cpu_usage);
    impl_get_set!(CpuRefreshKind, frequency, with_frequency, without_frequency);
}

/// Used to determine which memory you want to refresh specifically.
///
/// ⚠️ Just like all other refresh types, ruling out a refresh doesn't assure you that
/// the information won't be retrieved if the information is accessible without needing
/// extra computation.
///
/// ```
/// use sysinfo::{MemoryRefreshKind, System};
///
/// let mut system = System::new();
///
/// // We don't want to update all memories information.
/// system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
///
/// println!("total RAM: {}", system.total_memory());
/// println!("free RAM:  {}", system.free_memory());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemoryRefreshKind {
    ram: bool,
    swap: bool,
}

impl MemoryRefreshKind {
    /// Creates a new `MemoryRefreshKind` with every refresh set to `false`.
    ///
    /// ```
    /// use sysinfo::MemoryRefreshKind;
    ///
    /// let r = MemoryRefreshKind::nothing();
    ///
    /// assert_eq!(r.ram(), false);
    /// assert_eq!(r.swap(), false);
    /// ```
    pub fn nothing() -> Self {
        Self::default()
    }

    /// Creates a new `MemoryRefreshKind` with every refresh set to `true`.
    ///
    /// ```
    /// use sysinfo::MemoryRefreshKind;
    ///
    /// let r = MemoryRefreshKind::everything();
    ///
    /// assert_eq!(r.ram(), true);
    /// assert_eq!(r.swap(), true);
    /// ```
    pub fn everything() -> Self {
        Self {
            ram: true,
            swap: true,
        }
    }

    impl_get_set!(MemoryRefreshKind, ram, with_ram, without_ram);
    impl_get_set!(MemoryRefreshKind, swap, with_swap, without_swap);
}

/// Used to determine what you want to refresh specifically on the [`System`][crate::System] type.
///
/// ⚠️ Just like all other refresh types, ruling out a refresh doesn't assure you that
/// the information won't be retrieved if the information is accessible without needing
/// extra computation.
///
/// ```
/// use sysinfo::{RefreshKind, System};
///
/// // We want everything except memory.
/// let mut system = System::new_with_specifics(RefreshKind::everything().without_memory());
///
/// assert_eq!(system.total_memory(), 0);
/// # if sysinfo::IS_SUPPORTED_SYSTEM && !cfg!(feature = "apple-sandbox") {
/// assert!(system.processes().len() > 0);
/// # }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RefreshKind {
    processes: Option<ProcessRefreshKind>,
    memory: Option<MemoryRefreshKind>,
    cpu: Option<CpuRefreshKind>,
}

impl RefreshKind {
    /// Creates a new `RefreshKind` with every refresh set to `false`/`None`.
    ///
    /// ```
    /// use sysinfo::RefreshKind;
    ///
    /// let r = RefreshKind::nothing();
    ///
    /// assert_eq!(r.processes().is_some(), false);
    /// assert_eq!(r.memory().is_some(), false);
    /// assert_eq!(r.cpu().is_some(), false);
    /// ```
    pub fn nothing() -> Self {
        Self::default()
    }

    /// Creates a new `RefreshKind` with every refresh set to `true`/`Some(...)`.
    ///
    /// ```
    /// use sysinfo::RefreshKind;
    ///
    /// let r = RefreshKind::everything();
    ///
    /// assert_eq!(r.processes().is_some(), true);
    /// assert_eq!(r.memory().is_some(), true);
    /// assert_eq!(r.cpu().is_some(), true);
    /// ```
    pub fn everything() -> Self {
        Self {
            processes: Some(ProcessRefreshKind::everything()),
            memory: Some(MemoryRefreshKind::everything()),
            cpu: Some(CpuRefreshKind::everything()),
        }
    }

    impl_get_set!(
        RefreshKind,
        processes,
        with_processes,
        without_processes,
        ProcessRefreshKind
    );
    impl_get_set!(
        RefreshKind,
        memory,
        with_memory,
        without_memory,
        MemoryRefreshKind
    );
    impl_get_set!(RefreshKind, cpu, with_cpu, without_cpu, CpuRefreshKind);
}

/// Returns the pid for the current process.
///
/// `Err` is returned in case the platform isn't supported.
///
/// ```no_run
/// use sysinfo::get_current_pid;
///
/// match get_current_pid() {
///     Ok(pid) => {
///         println!("current pid: {}", pid);
///     }
///     Err(e) => {
///         println!("failed to get current pid: {}", e);
///     }
/// }
/// ```
#[allow(clippy::unnecessary_wraps)]
pub fn get_current_pid() -> Result<Pid, &'static str> {
    cfg_if! {
        if #[cfg(feature = "unknown-ci")] {
            fn inner() -> Result<Pid, &'static str> {
                Err("Unknown platform (CI)")
            }
        } else if #[cfg(any(
            target_os = "freebsd",
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "ios",
        ))] {
            fn inner() -> Result<Pid, &'static str> {
                unsafe { Ok(Pid(libc::getpid())) }
            }
        } else if #[cfg(windows)] {
            fn inner() -> Result<Pid, &'static str> {
                use windows::Win32::System::Threading::GetCurrentProcessId;

                unsafe { Ok(Pid(GetCurrentProcessId() as _)) }
            }
        } else {
            fn inner() -> Result<Pid, &'static str> {
                Err("Unknown platform")
            }
        }
    }
    inner()
}

/// Contains all the methods of the [`Cpu`][crate::Cpu] struct.
///
/// ```no_run
/// use sysinfo::{System, RefreshKind, CpuRefreshKind};
///
/// let mut s = System::new_with_specifics(
///     RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
/// );
///
/// // Wait a bit because CPU usage is based on diff.
/// std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
/// // Refresh CPUs again to get actual value.
/// s.refresh_cpu_all();
///
/// for cpu in s.cpus() {
///     println!("{}%", cpu.cpu_usage());
/// }
/// ```
pub struct Cpu {
    pub(crate) inner: CpuInner,
}

impl Cpu {
    /// Returns this CPU's usage.
    ///
    /// Note: You'll need to refresh it at least twice (diff between the first and the second is
    /// how CPU usage is computed) at first if you want to have a non-zero value.
    ///
    /// ```no_run
    /// use sysinfo::{System, RefreshKind, CpuRefreshKind};
    ///
    /// let mut s = System::new_with_specifics(
    ///     RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
    /// );
    ///
    /// // Wait a bit because CPU usage is based on diff.
    /// std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    /// // Refresh CPUs again to get actual value.
    /// s.refresh_cpu_all();
    ///
    /// for cpu in s.cpus() {
    ///     println!("{}%", cpu.cpu_usage());
    /// }
    /// ```
    pub fn cpu_usage(&self) -> f32 {
        self.inner.cpu_usage()
    }

    /// Returns this CPU's name.
    ///
    /// ```no_run
    /// use sysinfo::{System, RefreshKind, CpuRefreshKind};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.name());
    /// }
    /// ```
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Returns the CPU's vendor id.
    ///
    /// ```no_run
    /// use sysinfo::{System, RefreshKind, CpuRefreshKind};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.vendor_id());
    /// }
    /// ```
    pub fn vendor_id(&self) -> &str {
        self.inner.vendor_id()
    }

    /// Returns the CPU's brand.
    ///
    /// ```no_run
    /// use sysinfo::{System, RefreshKind, CpuRefreshKind};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.brand());
    /// }
    /// ```
    pub fn brand(&self) -> &str {
        self.inner.brand()
    }

    /// Returns the CPU's frequency.
    ///
    /// ```no_run
    /// use sysinfo::{System, RefreshKind, CpuRefreshKind};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.frequency());
    /// }
    /// ```
    pub fn frequency(&self) -> u64 {
        self.inner.frequency()
    }
}

#[cfg(test)]
mod test {
    use crate::*;
    use std::str::FromStr;

    // In case `Process::updated` is misused, `System::refresh_processes` might remove them
    // so this test ensures that it doesn't happen.
    #[test]
    fn check_refresh_process_update() {
        if !IS_SUPPORTED_SYSTEM {
            return;
        }
        let mut s = System::new_all();
        let total = s.processes().len() as isize;
        s.refresh_processes(ProcessesToUpdate::All, false);
        let new_total = s.processes().len() as isize;
        // There should be almost no difference in the processes count.
        assert!(
            (new_total - total).abs() <= 5,
            "{} <= 5",
            (new_total - total).abs()
        );
    }

    #[test]
    fn check_cpu_arch() {
        assert!(!System::cpu_arch().is_empty());
    }

    // Ensure that the CPUs frequency isn't retrieved until we ask for it.
    #[test]
    fn check_cpu_frequency() {
        if !IS_SUPPORTED_SYSTEM {
            return;
        }
        let mut s = System::new();
        s.refresh_processes(ProcessesToUpdate::All, false);
        for proc_ in s.cpus() {
            assert_eq!(proc_.frequency(), 0);
        }
        s.refresh_cpu_usage();
        for proc_ in s.cpus() {
            assert_eq!(proc_.frequency(), 0);
        }
        // In a VM, it'll fail.
        if std::env::var("APPLE_CI").is_err() && std::env::var("FREEBSD_CI").is_err() {
            s.refresh_cpu_specifics(CpuRefreshKind::everything());
            for proc_ in s.cpus() {
                assert_ne!(proc_.frequency(), 0);
            }
        }
    }

    #[test]
    fn check_process_memory_usage() {
        let mut s = System::new();
        s.refresh_specifics(RefreshKind::everything());

        if IS_SUPPORTED_SYSTEM {
            // No process should have 0 as memory usage.
            #[cfg(not(feature = "apple-sandbox"))]
            assert!(!s.processes().iter().all(|(_, proc_)| proc_.memory() == 0));
        } else {
            // There should be no process, but if there is one, its memory usage should be 0.
            assert!(s.processes().iter().all(|(_, proc_)| proc_.memory() == 0));
        }
    }

    #[test]
    fn check_system_implemented_traits() {
        fn check<T: Sized + std::fmt::Debug + Default + Send + Sync>(_: T) {}

        check(System::new());
    }

    #[test]
    fn check_memory_usage() {
        let mut s = System::new();

        assert_eq!(s.total_memory(), 0);
        assert_eq!(s.free_memory(), 0);
        assert_eq!(s.available_memory(), 0);
        assert_eq!(s.used_memory(), 0);
        assert_eq!(s.total_swap(), 0);
        assert_eq!(s.free_swap(), 0);
        assert_eq!(s.used_swap(), 0);

        s.refresh_memory();
        if IS_SUPPORTED_SYSTEM {
            assert!(s.total_memory() > 0);
            assert!(s.used_memory() > 0);
            if s.total_swap() > 0 {
                // I think it's pretty safe to assume that there is still some swap left...
                assert!(s.free_swap() > 0);
            }
        } else {
            assert_eq!(s.total_memory(), 0);
            assert_eq!(s.used_memory(), 0);
            assert_eq!(s.total_swap(), 0);
            assert_eq!(s.free_swap(), 0);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn check_processes_cpu_usage() {
        if !IS_SUPPORTED_SYSTEM {
            return;
        }
        let mut s = System::new();

        s.refresh_processes(ProcessesToUpdate::All, false);
        // All CPU usage will start at zero until the second refresh
        assert!(s
            .processes()
            .iter()
            .all(|(_, proc_)| proc_.cpu_usage() == 0.0));

        // Wait a bit to update CPU usage values
        std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
        s.refresh_processes(ProcessesToUpdate::All, true);
        assert!(s
            .processes()
            .iter()
            .all(|(_, proc_)| proc_.cpu_usage() >= 0.0
                && proc_.cpu_usage() <= (s.cpus().len() as f32) * 100.0));
        assert!(s
            .processes()
            .iter()
            .any(|(_, proc_)| proc_.cpu_usage() > 0.0));
    }

    #[test]
    fn check_cpu_usage() {
        if !IS_SUPPORTED_SYSTEM {
            return;
        }
        let mut s = System::new();
        for _ in 0..10 {
            s.refresh_cpu_usage();
            // Wait a bit to update CPU usage values
            std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
            if s.cpus().iter().any(|c| c.cpu_usage() > 0.0) {
                // All good!
                return;
            }
        }
        panic!("CPU usage is always zero...");
    }

    #[test]
    fn check_system_info() {
        // We don't want to test on unsupported systems.
        if IS_SUPPORTED_SYSTEM {
            assert!(!System::name()
                .expect("Failed to get system name")
                .is_empty());

            assert!(!System::kernel_version()
                .expect("Failed to get kernel version")
                .is_empty());

            assert!(!System::os_version()
                .expect("Failed to get os version")
                .is_empty());

            assert!(!System::long_os_version()
                .expect("Failed to get long OS version")
                .is_empty());
        }

        assert!(!System::distribution_id().is_empty());
    }

    #[test]
    fn check_host_name() {
        // We don't want to test on unsupported systems.
        if IS_SUPPORTED_SYSTEM {
            assert!(System::host_name().is_some());
        }
    }

    #[test]
    fn check_refresh_process_return_value() {
        // We don't want to test on unsupported systems.
        if IS_SUPPORTED_SYSTEM {
            let _pid = get_current_pid().expect("Failed to get current PID");

            #[cfg(not(feature = "apple-sandbox"))]
            {
                let mut s = System::new();
                // First check what happens in case the process isn't already in our process list.
                assert_eq!(
                    s.refresh_processes(ProcessesToUpdate::Some(&[_pid]), true),
                    1
                );
                // Then check that it still returns 1 if the process is already in our process list.
                assert_eq!(
                    s.refresh_processes(ProcessesToUpdate::Some(&[_pid]), true),
                    1
                );
            }
        }
    }

    #[test]
    fn check_cpus_number() {
        let mut s = System::new();

        // This information isn't retrieved by default.
        assert!(s.cpus().is_empty());
        if IS_SUPPORTED_SYSTEM {
            // The physical cores count is recomputed every time the function is called, so the
            // information must be relevant even with nothing initialized.
            let physical_cores_count =
                System::physical_core_count().expect("failed to get number of physical cores");

            s.refresh_cpu_usage();
            // The cpus shouldn't be empty anymore.
            assert!(!s.cpus().is_empty());

            // In case we are running inside a VM, it's possible to not have a physical core, only
            // logical ones, which is why we don't test `physical_cores_count > 0`.
            let physical_cores_count2 =
                System::physical_core_count().expect("failed to get number of physical cores");
            assert!(physical_cores_count2 <= s.cpus().len());
            assert_eq!(physical_cores_count, physical_cores_count2);
        } else {
            assert_eq!(System::physical_core_count(), None);
        }
        assert!(System::physical_core_count().unwrap_or(0) <= s.cpus().len());
    }

    // This test only exists to ensure that the `Display` and `Debug` traits are implemented on the
    // `ProcessStatus` enum on all targets.
    #[test]
    fn check_display_impl_process_status() {
        println!("{} {:?}", ProcessStatus::Parked, ProcessStatus::Idle);
    }

    #[test]
    #[allow(clippy::unnecessary_fallible_conversions)]
    fn check_pid_from_impls() {
        assert!(crate::Pid::try_from(0usize).is_ok());
        // If it doesn't panic, it's fine.
        let _ = crate::Pid::from(0);
        assert!(crate::Pid::from_str("0").is_ok());
    }

    #[test]
    #[allow(clippy::const_is_empty)]
    fn check_nb_supported_signals() {
        if IS_SUPPORTED_SYSTEM {
            assert!(
                !SUPPORTED_SIGNALS.is_empty(),
                "SUPPORTED_SIGNALS shouldn't be empty on supported systems!"
            );
        } else {
            assert!(
                SUPPORTED_SIGNALS.is_empty(),
                "SUPPORTED_SIGNALS should be empty on not support systems!"
            );
        }
    }
}

#[cfg(doctest)]
mod doctest {
    // FIXME: Can be removed once negative trait bounds are supported.
    /// Check that `Process` doesn't implement `Clone`.
    ///
    /// First we check that the "basic" code works:
    ///
    /// ```no_run
    /// use sysinfo::{Process, System};
    ///
    /// let mut s = System::new_all();
    /// let p: &Process = s.processes().values().next().unwrap();
    /// ```
    ///
    /// And now we check if it fails when we try to clone it:
    ///
    /// ```compile_fail
    /// use sysinfo::{Process, System};
    ///
    /// let mut s = System::new_all();
    /// let p: &Process = s.processes().values().next().unwrap();
    /// let p = (*p).clone();
    /// ```
    mod process_clone {}

    // FIXME: Can be removed once negative trait bounds are supported.
    /// Check that `System` doesn't implement `Clone`.
    ///
    /// First we check that the "basic" code works:
    ///
    /// ```no_run
    /// use sysinfo::{Process, System};
    ///
    /// let s = System::new();
    /// ```
    ///
    /// And now we check if it fails when we try to clone it:
    ///
    /// ```compile_fail
    /// use sysinfo::{Process, System};
    ///
    /// let s = System::new();
    /// let s = s.clone();
    /// ```
    mod system_clone {}
}
