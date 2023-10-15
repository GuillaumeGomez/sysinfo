// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    ComponentInner, ComponentsInner, CpuInner, NetworkDataInner, NetworksInner, ProcessInner,
    SystemInner, UserInner,
};

use std::cmp::Ordering;
use std::collections::HashMap;
use std::convert::{From, TryFrom};
use std::ffi::OsStr;
use std::fmt;
use std::path::Path;
use std::str::FromStr;

/// Structs containing system's information such as processes, memory and CPU.
///
/// ```
/// use sysinfo::System;
///
/// if sysinfo::IS_SUPPORTED {
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
    /// Use the [`refresh_all`] method to update its internal information (or any of the `refresh_`
    /// method).
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
        Self::new_with_specifics(RefreshKind::new())
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
    ///      RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    /// );
    ///
    /// # if sysinfo::IS_SUPPORTED && !cfg!(feature = "apple-sandbox") {
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
    /// ```
    /// use sysinfo::{ProcessRefreshKind, RefreshKind, System};
    ///
    /// let mut s = System::new_all();
    ///
    /// // Let's just update processes:
    /// s.refresh_specifics(
    ///     RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    /// );
    /// ```
    pub fn refresh_specifics(&mut self, refreshes: RefreshKind) {
        if refreshes.memory() {
            self.refresh_memory();
        }
        if let Some(kind) = refreshes.cpu() {
            self.refresh_cpu_specifics(kind);
        }
        if let Some(kind) = refreshes.processes() {
            self.refresh_processes_specifics(kind);
        }
    }

    /// Refreshes all system and processes information.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let mut s = System::new_all();
    /// s.refresh_all();
    /// ```
    pub fn refresh_all(&mut self) {
        self.refresh_system();
        self.refresh_processes();
    }

    /// Refreshes system information (RAM, swap, CPU usage and components' temperature).
    ///
    /// If you want some more specific refreshes, you might be interested into looking at
    /// [`refresh_memory`] and [`refresh_cpu`].
    ///
    /// [`refresh_memory`]: System::refresh_memory
    /// [`refresh_cpu`]: System::refresh_memory
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let mut s = System::new_all();
    /// s.refresh_system();
    /// ```
    pub fn refresh_system(&mut self) {
        self.refresh_memory();
        self.refresh_cpu_usage();
    }

    /// Refreshes RAM and SWAP usage.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let mut s = System::new_all();
    /// s.refresh_memory();
    /// ```
    pub fn refresh_memory(&mut self) {
        self.inner.refresh_memory()
    }

    /// Refreshes CPUs usage.
    ///
    /// ⚠️ Please note that the result will very likely be inaccurate at the first call.
    /// You need to call this method at least twice (with a bit of time between each call, like
    /// 200 ms, take a look at [`MINIMUM_CPU_UPDATE_INTERVAL`] for more information)
    /// to get accurate value as it uses previous results to compute the next value.
    ///
    /// Calling this method is the same as calling
    /// `refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage())`.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let mut s = System::new_all();
    /// s.refresh_cpu_usage();
    /// ```
    ///
    /// [`MINIMUM_CPU_UPDATE_INTERVAL`]: crate::MINIMUM_CPU_UPDATE_INTERVAL
    pub fn refresh_cpu_usage(&mut self) {
        self.refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage())
    }

    /// Refreshes CPUs frequency information.
    ///
    /// Calling this method is the same as calling
    /// `refresh_cpu_specifics(CpuRefreshKind::new().with_frequency())`.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let mut s = System::new_all();
    /// s.refresh_cpu_frequency();
    /// ```
    pub fn refresh_cpu_frequency(&mut self) {
        self.refresh_cpu_specifics(CpuRefreshKind::new().with_frequency())
    }

    /// Refreshes all information related to CPUs information.
    ///
    /// ⚠️ Please note that the result will very likely be inaccurate at the first call.
    /// You need to call this method at least twice (with a bit of time between each call, like
    /// 200 ms, take a look at [`MINIMUM_CPU_UPDATE_INTERVAL`] for more information)
    /// to get accurate value as it uses previous results to compute the next value.
    ///
    /// Calling this method is the same as calling
    /// `refresh_cpu_specifics(CpuRefreshKind::everything())`.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let mut s = System::new_all();
    /// s.refresh_cpu();
    /// ```
    ///
    /// [`MINIMUM_CPU_UPDATE_INTERVAL`]: crate::MINIMUM_CPU_UPDATE_INTERVAL
    pub fn refresh_cpu(&mut self) {
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

    /// Gets all processes and updates their information.
    ///
    /// It does the same as `system.refresh_processes_specifics(ProcessRefreshKind::everything())`.
    ///
    /// ⚠️ On Linux, `sysinfo` keeps the `stat` files open by default. You can change this behaviour
    /// by using [`set_open_files_limit`][crate::set_open_files_limit].
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let mut s = System::new_all();
    /// s.refresh_processes();
    /// ```
    pub fn refresh_processes(&mut self) {
        self.refresh_processes_specifics(ProcessRefreshKind::everything());
    }

    /// Gets all processes and updates the specified information.
    ///
    /// ⚠️ On Linux, `sysinfo` keeps the `stat` files open by default. You can change this behaviour
    /// by using [`set_open_files_limit`][crate::set_open_files_limit].
    ///
    /// ```no_run
    /// use sysinfo::{ProcessRefreshKind, System};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_processes_specifics(ProcessRefreshKind::new());
    /// ```
    pub fn refresh_processes_specifics(&mut self, refresh_kind: ProcessRefreshKind) {
        self.inner.refresh_processes_specifics(refresh_kind)
    }

    /// Refreshes *only* the process corresponding to `pid`. Returns `false` if the process doesn't
    /// exist (it will **NOT** be removed from the processes if it doesn't exist anymore). If it
    /// isn't listed yet, it'll be added.
    ///
    /// It is the same as calling
    /// `sys.refresh_process_specifics(pid, ProcessRefreshKind::everything())`.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_process(Pid::from(1337));
    /// ```
    pub fn refresh_process(&mut self, pid: Pid) -> bool {
        self.refresh_process_specifics(pid, ProcessRefreshKind::everything())
    }

    /// Refreshes *only* the process corresponding to `pid`. Returns `false` if the process doesn't
    /// exist (it will **NOT** be removed from the processes if it doesn't exist anymore). If it
    /// isn't listed yet, it'll be added.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessRefreshKind, System};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_process_specifics(Pid::from(1337), ProcessRefreshKind::new());
    /// ```
    pub fn refresh_process_specifics(
        &mut self,
        pid: Pid,
        refresh_kind: ProcessRefreshKind,
    ) -> bool {
        self.inner.refresh_process_specifics(pid, refresh_kind)
    }

    /// Returns the process list.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// for (pid, process) in s.processes() {
    ///     println!("{} {}", pid, process.name());
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
    ///     println!("{}", process.name());
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
    /// for process in s.processes_by_name("htop") {
    ///     println!("{} {}", process.pid(), process.name());
    /// }
    /// ```
    // FIXME: replace the returned type with `impl Iterator<Item = &Process>` when it's supported!
    pub fn processes_by_name<'a: 'b, 'b>(
        &'a self,
        name: &'b str,
    ) -> Box<dyn Iterator<Item = &'a Process> + 'b> {
        Box::new(
            self.processes()
                .values()
                .filter(move |val: &&Process| val.name().contains(name)),
        )
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
    /// for process in s.processes_by_exact_name("htop") {
    ///     println!("{} {}", process.pid(), process.name());
    /// }
    /// ```
    // FIXME: replace the returned type with `impl Iterator<Item = &Process>` when it's supported!
    pub fn processes_by_exact_name<'a: 'b, 'b>(
        &'a self,
        name: &'b str,
    ) -> Box<dyn Iterator<Item = &'a Process> + 'b> {
        Box::new(
            self.processes()
                .values()
                .filter(move |val: &&Process| val.name() == name),
        )
    }

    /// Returns "global" CPUs information (aka the addition of all the CPUs).
    ///
    /// To have up-to-date information, you need to call [`System::refresh_cpu`] or
    /// [`System::refresh_specifics`] with `cpu` enabled.
    ///
    /// **⚠️ Important ⚠️**
    ///
    /// Information like [`Cpu::brand`], [`Cpu::vendor_id`] or [`Cpu::frequency`]
    /// are not set on the "global" CPU.
    ///
    /// ```no_run
    /// use sysinfo::{CpuRefreshKind, RefreshKind, System};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// println!("{}%", s.global_cpu_info().cpu_usage());
    /// ```
    ///
    /// [`Cpu::brand`]: crate::Cpu::brand
    /// [`Cpu::vendor_id`]: crate::Cpu::vendor_id
    /// [`Cpu::frequency`]: crate::Cpu::frequency
    pub fn global_cpu_info(&self) -> &Cpu {
        self.inner.global_cpu_info()
    }

    /// Returns the list of the CPUs.
    ///
    /// By default, the list of CPUs is empty until you call [`System::refresh_cpu`] or
    /// [`System::refresh_specifics`] with `cpu` enabled.
    ///
    /// ```no_run
    /// use sysinfo::{CpuRefreshKind, RefreshKind, System};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// for cpu in s.cpus() {
    ///     println!("{}%", cpu.cpu_usage());
    /// }
    /// ```
    pub fn cpus(&self) -> &[Cpu] {
        self.inner.cpus()
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
    /// println!("{:?}", s.physical_core_count());
    /// ```
    pub fn physical_core_count(&self) -> Option<usize> {
        self.inner.physical_core_count()
    }

    /// Returns the RAM size in bytes.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.total_memory());
    /// ```
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

    /// Returns system uptime (in seconds).
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// println!("System running since {} seconds", s.uptime());
    /// ```
    pub fn uptime(&self) -> u64 {
        self.inner.uptime()
    }

    /// Returns the time (in seconds) when the system booted since UNIX epoch.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new();
    /// println!("System booted at {} seconds", s.boot_time());
    /// ```
    pub fn boot_time(&self) -> u64 {
        self.inner.boot_time()
    }

    /// Returns the system load average value.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new_all();
    /// let load_avg = s.load_average();
    /// println!(
    ///     "one minute: {}%, five minutes: {}%, fifteen minutes: {}%",
    ///     load_avg.one,
    ///     load_avg.five,
    ///     load_avg.fifteen,
    /// );
    /// ```
    pub fn load_average(&self) -> LoadAvg {
        self.inner.load_average()
    }

    /// Returns the system name.
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new();
    /// println!("OS: {:?}", s.name());
    /// ```
    pub fn name(&self) -> Option<String> {
        self.inner.name()
    }

    /// Returns the system's kernel version.
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new();
    /// println!("kernel version: {:?}", s.kernel_version());
    /// ```
    pub fn kernel_version(&self) -> Option<String> {
        self.inner.kernel_version()
    }

    /// Returns the system version (e.g. for MacOS this will return 11.1 rather than the kernel version).
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new();
    /// println!("OS version: {:?}", s.os_version());
    /// ```
    pub fn os_version(&self) -> Option<String> {
        self.inner.os_version()
    }

    /// Returns the system long os version (e.g "MacOS 11.2 BigSur").
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new();
    /// println!("Long OS Version: {:?}", s.long_os_version());
    /// ```
    pub fn long_os_version(&self) -> Option<String> {
        self.inner.long_os_version()
    }

    /// Returns the distribution id as defined by os-release,
    /// or [`std::env::consts::OS`].
    ///
    /// See also
    /// - <https://www.freedesktop.org/software/systemd/man/os-release.html#ID=>
    /// - <https://doc.rust-lang.org/std/env/consts/constant.OS.html>
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new();
    /// println!("Distribution ID: {:?}", s.distribution_id());
    /// ```
    pub fn distribution_id(&self) -> String {
        self.inner.distribution_id()
    }

    /// Returns the system hostname based off DNS
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::System;
    ///
    /// let s = System::new();
    /// println!("Hostname: {:?}", s.host_name());
    /// ```
    pub fn host_name(&self) -> Option<String> {
        self.inner.host_name()
    }
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
///     println!("{}", process.name());
/// }
/// ```
pub struct Process {
    pub(crate) inner: ProcessInner,
}

impl Process {
    /// Sends [`Signal::Kill`] to the process (which is the only signal supported on all supported
    /// platforms by this crate).
    ///
    /// If you want to send another signal, take a look at [`Process::kill_with`].
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
    /// it'll do nothing and will return `None`. Otherwise it'll return if the signal was sent
    /// successfully.
    ///
    /// If you just want to kill the process, use [`Process::kill`] directly.
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
    ///         eprintln!("This signal isn't supported on this platform");
    ///     }
    /// }
    /// ```
    pub fn kill_with(&self, signal: Signal) -> Option<bool> {
        self.inner.kill_with(signal)
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
    ///     println!("{}", process.name());
    /// }
    /// ```
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Returns the command line.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.cmd());
    /// }
    /// ```
    pub fn cmd(&self) -> &[String] {
        self.inner.cmd()
    }

    /// Returns the path to the process.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}", process.exe().display());
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
    pub fn exe(&self) -> &Path {
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
    pub fn environ(&self) -> &[String] {
        self.inner.environ()
    }

    /// Returns the current working directory.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}", process.cwd().display());
    /// }
    /// ```
    pub fn cwd(&self) -> &Path {
        self.inner.cwd()
    }

    /// Returns the path of the root directory.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}", process.root().display());
    /// }
    /// ```
    pub fn root(&self) -> &Path {
        self.inner.root()
    }

    /// Returns the memory usage (in bytes).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{} bytes", process.memory());
    /// }
    /// ```
    pub fn memory(&self) -> u64 {
        self.inner.memory()
    }

    /// Returns the virtual memory usage (in bytes).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{} bytes", process.virtual_memory());
    /// }
    /// ```
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
    /// use sysinfo::{Pid, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}%", process.cpu_usage());
    /// }
    /// ```
    pub fn cpu_usage(&self) -> f32 {
        self.inner.cpu_usage()
    }

    /// Returns number of bytes read and written to disk.
    ///
    /// ⚠️ On Windows and FreeBSD, this method actually returns **ALL** I/O
    /// read and written bytes.
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
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     eprintln!("User id for process 1337: {:?}", process.user_id());
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
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     eprintln!("User id for process 1337: {:?}", process.effective_user_id());
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
    ///     eprintln!("Group ID for process 1337: {:?}", process.group_id());
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
    ///     eprintln!("User id for process 1337: {:?}", process.effective_group_id());
    /// }
    /// ```
    pub fn effective_group_id(&self) -> Option<Gid> {
        self.inner.effective_group_id()
    }

    /// Wait for process termination.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     eprintln!("Waiting for pid 1337");
    ///     process.wait();
    ///     eprintln!("Pid 1337 exited");
    /// }
    /// ```
    pub fn wait(&self) {
        self.inner.wait()
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
    ///     eprintln!("Session ID for process 1337: {:?}", process.session_id());
    /// }
    /// ```
    pub fn session_id(&self) -> Option<Pid> {
        self.inner.session_id()
    }

    /// Tasks run by this process.
    ///
    /// ⚠️ This method is only available on Linux.
    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        not(feature = "unknown-ci")
    ))]
    pub fn tasks(&self) -> &HashMap<Pid, Process> {
        &self.inner.tasks
    }
}

macro_rules! pid_decl {
    ($typ:ty) => {
        #[doc = include_str!("../md_doc/pid.md")]
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

cfg_if::cfg_if! {
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

macro_rules! impl_get_set {
    ($ty_name:ident, $name:ident, $with:ident, $without:ident $(, $extra_doc:literal)? $(,)?) => {
        #[doc = concat!("Returns the value of the \"", stringify!($name), "\" refresh kind.")]
        $(#[doc = concat!("
", $extra_doc, "
")])?
        #[doc = concat!("
```
use sysinfo::", stringify!($ty_name), ";

let r = ", stringify!($ty_name), "::new();
assert_eq!(r.", stringify!($name), "(), false);

let r = r.with_", stringify!($name), "();
assert_eq!(r.", stringify!($name), "(), true);

let r = r.without_", stringify!($name), "();
assert_eq!(r.", stringify!($name), "(), false);
```")]
        pub fn $name(&self) -> bool {
            self.$name
        }

        #[doc = concat!("Sets the value of the \"", stringify!($name), "\" refresh kind to `true`.

```
use sysinfo::", stringify!($ty_name), ";

let r = ", stringify!($ty_name), "::new();
assert_eq!(r.", stringify!($name), "(), false);

let r = r.with_", stringify!($name), "();
assert_eq!(r.", stringify!($name), "(), true);
```")]
        #[must_use]
        pub fn $with(mut self) -> Self {
            self.$name = true;
            self
        }

        #[doc = concat!("Sets the value of the \"", stringify!($name), "\" refresh kind to `false`.

```
use sysinfo::", stringify!($ty_name), ";

let r = ", stringify!($ty_name), "::everything();
assert_eq!(r.", stringify!($name), "(), true);

let r = r.without_", stringify!($name), "();
assert_eq!(r.", stringify!($name), "(), false);
```")]
        #[must_use]
        pub fn $without(mut self) -> Self {
            self.$name = false;
            self
        }
    };

    ($ty_name:ident, $name:ident, $with:ident, $without:ident, $typ:ty $(,)?) => {
        #[doc = concat!("Returns the value of the \"", stringify!($name), "\" refresh kind.

```
use sysinfo::{", stringify!($ty_name), ", ", stringify!($typ), "};

let r = ", stringify!($ty_name), "::new();
assert_eq!(r.", stringify!($name), "().is_some(), false);

let r = r.with_", stringify!($name), "(", stringify!($typ), "::everything());
assert_eq!(r.", stringify!($name), "().is_some(), true);

let r = r.without_", stringify!($name), "();
assert_eq!(r.", stringify!($name), "().is_some(), false);
```")]
        pub fn $name(&self) -> Option<$typ> {
            self.$name
        }

        #[doc = concat!("Sets the value of the \"", stringify!($name), "\" refresh kind to `true`.

```
use sysinfo::{", stringify!($ty_name), ", ", stringify!($typ), "};

let r = ", stringify!($ty_name), "::new();
assert_eq!(r.", stringify!($name), "().is_some(), false);

let r = r.with_", stringify!($name), "(", stringify!($typ), "::everything());
assert_eq!(r.", stringify!($name), "().is_some(), true);
```")]
        #[must_use]
        pub fn $with(mut self, kind: $typ) -> Self {
            self.$name = Some(kind);
            self
        }

        #[doc = concat!("Sets the value of the \"", stringify!($name), "\" refresh kind to `false`.

```
use sysinfo::", stringify!($ty_name), ";

let r = ", stringify!($ty_name), "::everything();
assert_eq!(r.", stringify!($name), "().is_some(), true);

let r = r.without_", stringify!($name), "();
assert_eq!(r.", stringify!($name), "().is_some(), false);
```")]
        #[must_use]
        pub fn $without(mut self) -> Self {
            self.$name = None;
            self
        }
    };
}

/// Used to determine what you want to refresh specifically on the [`Process`] type.
///
/// ⚠️ Just like all other refresh types, ruling out a refresh doesn't assure you that
/// the information won't be retrieved if the information is accessible without needing
/// extra computation.
///
/// ```
/// use sysinfo::{ProcessRefreshKind, System};
///
/// let mut system = System::new();
///
/// // We don't want to update the CPU information.
/// system.refresh_processes_specifics(ProcessRefreshKind::everything().without_cpu());
///
/// for (_, proc_) in system.processes() {
///     // We use a `==` comparison on float only because we know it's set to 0 here.
///     assert_eq!(proc_.cpu_usage(), 0.);
/// }
/// ```
///
/// [`Process`]: crate::Process
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProcessRefreshKind {
    cpu: bool,
    disk_usage: bool,
    user: bool,
}

impl ProcessRefreshKind {
    /// Creates a new `ProcessRefreshKind` with every refresh set to `false`.
    ///
    /// ```
    /// use sysinfo::ProcessRefreshKind;
    ///
    /// let r = ProcessRefreshKind::new();
    ///
    /// assert_eq!(r.cpu(), false);
    /// assert_eq!(r.disk_usage(), false);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new `ProcessRefreshKind` with every refresh set to `true`.
    ///
    /// ```
    /// use sysinfo::ProcessRefreshKind;
    ///
    /// let r = ProcessRefreshKind::everything();
    ///
    /// assert_eq!(r.cpu(), true);
    /// assert_eq!(r.disk_usage(), true);
    /// ```
    pub fn everything() -> Self {
        Self {
            cpu: true,
            disk_usage: true,
            user: true,
        }
    }

    impl_get_set!(ProcessRefreshKind, cpu, with_cpu, without_cpu);
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
        r#"This refresh is about `user_id` and `group_id`. Please note that it has an effect mostly
on Windows as other platforms get this information alongside the Process information directly."#,
    );
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
    /// let r = CpuRefreshKind::new();
    ///
    /// assert_eq!(r.frequency(), false);
    /// ```
    pub fn new() -> Self {
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
/// # if sysinfo::IS_SUPPORTED && !cfg!(feature = "apple-sandbox") {
/// assert!(system.processes().len() > 0);
/// # }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RefreshKind {
    processes: Option<ProcessRefreshKind>,
    memory: bool,
    cpu: Option<CpuRefreshKind>,
}

impl RefreshKind {
    /// Creates a new `RefreshKind` with every refresh set to `false`/`None`.
    ///
    /// ```
    /// use sysinfo::RefreshKind;
    ///
    /// let r = RefreshKind::new();
    ///
    /// assert_eq!(r.processes().is_some(), false);
    /// assert_eq!(r.memory(), false);
    /// assert_eq!(r.cpu().is_some(), false);
    /// ```
    pub fn new() -> Self {
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
    /// assert_eq!(r.memory(), true);
    /// assert_eq!(r.cpu().is_some(), true);
    /// ```
    pub fn everything() -> Self {
        Self {
            processes: Some(ProcessRefreshKind::everything()),
            memory: true,
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
    impl_get_set!(RefreshKind, memory, with_memory, without_memory);
    impl_get_set!(RefreshKind, cpu, with_cpu, without_cpu, CpuRefreshKind);
}

/// Interacting with network interfaces.
///
/// ```no_run
/// use sysinfo::Networks;
///
/// let mut networks = Networks::new();
/// networks.refresh_list();
/// for (interface_name, network) in &networks {
///     println!("[{interface_name}]: {network:?}");
/// }
/// ```
pub struct Networks {
    pub(crate) inner: NetworksInner,
}

impl<'a> IntoIterator for &'a Networks {
    type Item = (&'a String, &'a NetworkData);
    type IntoIter = NetworksIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Default for Networks {
    fn default() -> Self {
        Networks::new()
    }
}

impl Networks {
    /// Creates a new [`Networks`][crate::Networks] type.
    ///
    /// ```no_run
    /// use sysinfo::Networks;
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("[{interface_name}]: {network:?}");
    /// }
    /// ```
    pub fn new() -> Self {
        Self {
            inner: NetworksInner::new(),
        }
    }

    /// Returns an iterator over the network interfaces.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, System};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, data) in &networks {
    ///     println!(
    ///         "[{interface_name}] in: {}, out: {}",
    ///         data.received(),
    ///         data.transmitted(),
    ///     );
    /// }
    /// ```
    pub fn iter(&self) -> NetworksIter {
        self.inner.iter()
    }

    /// Refreshes the network interfaces list.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, System};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// ```
    pub fn refresh_list(&mut self) {
        self.inner.refresh_list()
    }

    /// Refreshes the network interfaces' content. If you didn't run [`Networks::refresh_list`]
    /// before, calling this method won't do anything as no interfaces are present.
    ///
    /// ⚠️ If a user is added or removed, this method won't take it into account. Use
    /// [`Networks::refresh_list`] instead.
    ///
    /// ⚠️ If you didn't call [`Networks::refresh_list`] beforehand, this method will do nothing
    /// as the network list will be empty.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, System};
    ///
    /// let mut networks = Networks::new();
    /// // Refreshes the network interfaces list.
    /// networks.refresh_list();
    /// // Wait some time...? Then refresh the data of each network.
    /// networks.refresh();
    /// ```
    pub fn refresh(&mut self) {
        self.inner.refresh()
    }
}

/// Iterator over network interfaces.
///
/// It is returned by [`Networks::iter`][crate::Networks#method.iter].
///
/// ```no_run
/// use sysinfo::Networks;
///
/// let networks = Networks::new();
/// let networks_iter = networks.iter();
/// ```
pub struct NetworksIter<'a> {
    inner: std::collections::hash_map::Iter<'a, String, NetworkData>,
}

impl<'a> NetworksIter<'a> {
    pub(crate) fn new(v: std::collections::hash_map::Iter<'a, String, NetworkData>) -> Self {
        NetworksIter { inner: v }
    }
}

impl<'a> Iterator for NetworksIter<'a> {
    type Item = (&'a String, &'a NetworkData);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Getting volume of received and transmitted data.
///
/// ```no_run
/// use sysinfo::Networks;
///
/// let mut networks = Networks::new();
/// networks.refresh_list();
/// for (interface_name, network) in &networks {
///     println!("[{interface_name}] {network:?}");
/// }
/// ```
pub struct NetworkData {
    pub(crate) inner: NetworkDataInner,
}

impl NetworkData {
    /// Returns the number of received bytes since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::Networks;
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {} B", network.received());
    /// }
    /// ```
    pub fn received(&self) -> u64 {
        self.inner.received()
    }

    /// Returns the total number of received bytes.
    ///
    /// ```no_run
    /// use sysinfo::Networks;
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {} B", network.total_received());
    /// }
    /// ```
    pub fn total_received(&self) -> u64 {
        self.inner.total_received()
    }

    /// Returns the number of transmitted bytes since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::Networks;
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {} B", network.transmitted());
    /// }
    /// ```
    pub fn transmitted(&self) -> u64 {
        self.inner.transmitted()
    }

    /// Returns the total number of transmitted bytes.
    ///
    /// ```no_run
    /// use sysinfo::Networks;
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {} B", network.total_transmitted());
    /// }
    /// ```
    pub fn total_transmitted(&self) -> u64 {
        self.inner.total_transmitted()
    }

    /// Returns the number of incoming packets since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::Networks;
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {}", network.packets_received());
    /// }
    /// ```
    pub fn packets_received(&self) -> u64 {
        self.inner.packets_received()
    }

    /// Returns the total number of incoming packets.
    ///
    /// ```no_run
    /// use sysinfo::Networks;
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {}", network.total_packets_received());
    /// }
    /// ```
    pub fn total_packets_received(&self) -> u64 {
        self.inner.total_packets_received()
    }

    /// Returns the number of outcoming packets since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::Networks;
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {}", network.packets_transmitted());
    /// }
    /// ```
    pub fn packets_transmitted(&self) -> u64 {
        self.inner.packets_transmitted()
    }

    /// Returns the total number of outcoming packets.
    ///
    /// ```no_run
    /// use sysinfo::Networks;
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {}", network.total_packets_transmitted());
    /// }
    /// ```
    pub fn total_packets_transmitted(&self) -> u64 {
        self.inner.total_packets_transmitted()
    }

    /// Returns the number of incoming errors since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::Networks;
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {}", network.errors_on_received());
    /// }
    /// ```
    pub fn errors_on_received(&self) -> u64 {
        self.inner.errors_on_received()
    }

    /// Returns the total number of incoming errors.
    ///
    /// ```no_run
    /// use sysinfo::Networks;
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {}", network.total_errors_on_received());
    /// }
    /// ```
    pub fn total_errors_on_received(&self) -> u64 {
        self.inner.total_errors_on_received()
    }

    /// Returns the number of outcoming errors since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::Networks;
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {}", network.errors_on_transmitted());
    /// }
    /// ```
    pub fn errors_on_transmitted(&self) -> u64 {
        self.inner.errors_on_transmitted()
    }

    /// Returns the total number of outcoming errors.
    ///
    /// ```no_run
    /// use sysinfo::Networks;
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {}", network.total_errors_on_transmitted());
    /// }
    /// ```
    pub fn total_errors_on_transmitted(&self) -> u64 {
        self.inner.total_errors_on_transmitted()
    }

    /// Returns the MAC address associated to current interface.
    ///
    /// ```no_run
    /// use sysinfo::Networks;
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("MAC address: {}", network.mac_address());
    /// }
    /// ```
    pub fn mac_address(&self) -> MacAddr {
        self.inner.mac_address()
    }
}

/// Struct containing a disk information.
///
/// ```no_run
/// use sysinfo::Disks;
///
/// let mut disks = Disks::new();
/// disks.refresh_list();
/// for disk in disks.disks() {
///     println!("{:?}: {:?}", disk.name(), disk.kind());
/// }
/// ```
pub struct Disk {
    pub(crate) inner: crate::DiskInner,
}

impl Disk {
    /// Returns the kind of disk.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// for disk in disks.disks() {
    ///     println!("[{:?}] {:?}", disk.name(), disk.kind());
    /// }
    /// ```
    pub fn kind(&self) -> DiskKind {
        self.inner.kind()
    }

    /// Returns the disk name.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// for disk in disks.disks() {
    ///     println!("{:?}", disk.name());
    /// }
    /// ```
    pub fn name(&self) -> &OsStr {
        self.inner.name()
    }

    /// Returns the file system used on this disk (so for example: `EXT4`, `NTFS`, etc...).
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// for disk in disks.disks() {
    ///     println!("[{:?}] {:?}", disk.name(), disk.file_system());
    /// }
    /// ```
    pub fn file_system(&self) -> &[u8] {
        self.inner.file_system()
    }

    /// Returns the mount point of the disk (`/` for example).
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// for disk in disks.disks() {
    ///     println!("[{:?}] {:?}", disk.name(), disk.mount_point());
    /// }
    /// ```
    pub fn mount_point(&self) -> &Path {
        self.inner.mount_point()
    }

    /// Returns the total disk size, in bytes.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// for disk in disks.disks() {
    ///     println!("[{:?}] {}B", disk.name(), disk.total_space());
    /// }
    /// ```
    pub fn total_space(&self) -> u64 {
        self.inner.total_space()
    }

    /// Returns the available disk size, in bytes.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// for disk in disks.disks() {
    ///     println!("[{:?}] {}B", disk.name(), disk.available_space());
    /// }
    /// ```
    pub fn available_space(&self) -> u64 {
        self.inner.available_space()
    }

    /// Returns `true` if the disk is removable.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// for disk in disks.disks() {
    ///     println!("[{:?}] {}", disk.name(), disk.is_removable());
    /// }
    /// ```
    pub fn is_removable(&self) -> bool {
        self.inner.is_removable()
    }

    /// Updates the disk' information.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// for disk in disks.disks_mut() {
    ///     disk.refresh();
    /// }
    /// ```
    pub fn refresh(&mut self) -> bool {
        self.inner.refresh()
    }
}

/// Disks interface.
///
/// ```no_run
/// use sysinfo::Disks;
///
/// let mut disks = Disks::new();
/// disks.refresh_list();
/// for disk in disks.disks() {
///     println!("{disk:?}");
/// }
/// ```
pub struct Disks {
    inner: crate::DisksInner,
}

impl Default for Disks {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> IntoIterator for &'a Disks {
    type Item = &'a Disk;
    type IntoIter = std::slice::Iter<'a, Disk>;

    fn into_iter(self) -> Self::IntoIter {
        self.disks().iter()
    }
}

impl<'a> IntoIterator for &'a mut Disks {
    type Item = &'a mut Disk;
    type IntoIter = std::slice::IterMut<'a, Disk>;

    fn into_iter(self) -> Self::IntoIter {
        self.disks_mut().iter_mut()
    }
}

impl Disks {
    /// Creates a new [`Disks`][crate::Disks] type.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// for disk in disks.disks() {
    ///     eprintln!("{disk:?}");
    /// }
    /// ```
    pub fn new() -> Self {
        Self {
            inner: crate::DisksInner::new(),
        }
    }

    /// Returns the disks list.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// for disk in disks.disks() {
    ///     eprintln!("{disk:?}");
    /// }
    /// ```
    pub fn disks(&self) -> &[Disk] {
        self.inner.disks()
    }

    /// Returns the disks list.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// for disk in disks.disks_mut() {
    ///     disk.refresh();
    ///     eprintln!("{disk:?}");
    /// }
    /// ```
    pub fn disks_mut(&mut self) -> &mut [Disk] {
        self.inner.disks_mut()
    }

    /// Sort the disk list with the provided callback.
    ///
    /// Internally, it is using the [`slice::sort_unstable_by`] function, so please refer to it
    /// for implementation details.
    ///
    /// You can do the same without this method by calling:
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// disks.sort_by(|disk1, disk2| {
    ///     disk1.name().partial_cmp(disk2.name()).unwrap()
    /// });
    /// ```
    ///
    /// ⚠️ If you use [`Disks::refresh_list`], you will need to call this method to sort the
    /// disks again.
    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Disk, &Disk) -> std::cmp::Ordering,
    {
        self.disks_mut().sort_unstable_by(compare);
    }

    /// Refreshes the listed disks' information.
    ///
    /// ⚠️ If a disk is added or removed, this method won't take it into account. Use
    /// [`Disks::refresh_list`] instead.
    ///
    /// ⚠️ If you didn't call [`Disks::refresh_list`] beforehand, this method will do nothing as
    /// the disk list will be empty.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// // We get the disk list.
    /// disks.refresh_list();
    /// // We wait some time...?
    /// disks.refresh();
    /// ```
    pub fn refresh(&mut self) {
        for disk in self.disks_mut() {
            disk.refresh();
        }
    }

    /// The disk list will be emptied then completely recomputed.
    ///
    /// ## Linux
    ///
    /// ⚠️ On Linux, the [NFS](https://en.wikipedia.org/wiki/Network_File_System) file
    /// systems are ignored and the information of a mounted NFS **cannot** be obtained
    /// via [`Disks::refresh_list`]. This is due to the fact that I/O function
    /// `statvfs` used by [`Disks::refresh_list`] is blocking and
    /// [may hang](https://github.com/GuillaumeGomez/sysinfo/pull/876) in some cases,
    /// requiring to call `systemctl stop` to terminate the NFS service from the remote
    /// server in some cases.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// ```
    pub fn refresh_list(&mut self) {
        self.inner.refresh_list();
    }
}

impl std::ops::Deref for Disks {
    type Target = [Disk];

    fn deref(&self) -> &Self::Target {
        self.disks()
    }
}

impl std::ops::DerefMut for Disks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.disks_mut()
    }
}

/// Enum containing the different supported kinds of disks.
///
/// This type is returned by [`Disk::kind`](`crate::Disk::kind`).
///
/// ```no_run
/// use sysinfo::Disks;
///
/// let mut disks = Disks::new();
/// disks.refresh_list();
/// for disk in disks.disks() {
///     println!("{:?}: {:?}", disk.name(), disk.kind());
/// }
/// ```
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DiskKind {
    /// HDD type.
    HDD,
    /// SSD type.
    SSD,
    /// Unknown type.
    Unknown(isize),
}

/// Interacting with users.
///
/// ```no_run
/// use sysinfo::Users;
///
/// let mut users = Users::new();
/// for user in users.users() {
///     println!("{} is in {} groups", user.name(), user.groups().len());
/// }
/// ```
pub struct Users {
    users: Vec<User>,
}

impl Default for Users {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for Users {
    type Target = [User];

    fn deref(&self) -> &Self::Target {
        self.users()
    }
}

impl std::ops::DerefMut for Users {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.users_mut()
    }
}

impl<'a> IntoIterator for &'a Users {
    type Item = &'a User;
    type IntoIter = std::slice::Iter<'a, User>;

    fn into_iter(self) -> Self::IntoIter {
        self.users().iter()
    }
}

impl<'a> IntoIterator for &'a mut Users {
    type Item = &'a mut User;
    type IntoIter = std::slice::IterMut<'a, User>;

    fn into_iter(self) -> Self::IntoIter {
        self.users_mut().iter_mut()
    }
}

impl Users {
    /// Creates a new [`Components`][crate::Components] type.
    ///
    /// ```no_run
    /// use sysinfo::Users;
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     eprintln!("{user:?}");
    /// }
    /// ```
    pub fn new() -> Self {
        Self { users: Vec::new() }
    }

    /// Returns the users list.
    ///
    /// ```no_run
    /// use sysinfo::Users;
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     eprintln!("{user:?}");
    /// }
    /// ```
    pub fn users(&self) -> &[User] {
        &self.users
    }

    /// Returns the users list.
    ///
    /// ```no_run
    /// use sysinfo::Users;
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// users.users_mut().sort_by(|user1, user2| {
    ///     user1.name().partial_cmp(user2.name()).unwrap()
    /// });
    /// ```
    pub fn users_mut(&mut self) -> &mut [User] {
        &mut self.users
    }

    /// Sort the users list with the provided callback.
    ///
    /// Internally, it is using the [`slice::sort_unstable_by`] function, so please refer to it
    /// for implementation details.
    ///
    /// You can do the same without this method by calling:
    ///
    /// ```no_run
    /// use sysinfo::Users;
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// users.sort_by(|user1, user2| {
    ///     user1.name().partial_cmp(user2.name()).unwrap()
    /// });
    /// ```
    ///
    /// ⚠️ If you use [`Users::refresh_list`], you will need to call this method to sort the
    /// users again.
    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&User, &User) -> std::cmp::Ordering,
    {
        self.users.sort_unstable_by(compare);
    }

    /// The user list will be emptied then completely recomputed.
    ///
    /// ```no_run
    /// use sysinfo::Users;
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// ```
    pub fn refresh_list(&mut self) {
        crate::sys::get_users(&mut self.users);
    }

    /// Returns the [`User`] matching the given `user_id`.
    ///
    /// **Important**: The user list must be filled before using this method, otherwise it will
    /// always return `None` (through the `refresh_*` methods).
    ///
    /// It is a shorthand for:
    ///
    /// ```ignore
    /// # use sysinfo::Users;
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// users.users().find(|user| user.id() == user_id);
    /// ```
    ///
    /// Full example:
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System, Users};
    ///
    /// let mut s = System::new_all();
    /// let mut users = Users::new();
    ///
    /// users.refresh_list();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     if let Some(user_id) = process.user_id() {
    ///         eprintln!("User for process 1337: {:?}", users.get_user_by_id(user_id));
    ///     }
    /// }
    /// ```
    pub fn get_user_by_id(&self, user_id: &Uid) -> Option<&User> {
        self.users.iter().find(|user| user.id() == user_id)
    }
}

/// An enum representing signals on UNIX-like systems.
///
/// On non-unix systems, this enum is mostly useless and is only there to keep coherency between
/// the different OSes.
///
/// If you want the list of the supported signals on the current system, use
/// [`SUPPORTED_SIGNALS`][crate::SUPPORTED_SIGNALS].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Debug)]
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

/// A struct representing system load average value.
///
/// It is returned by [`System::load_average`][crate::System::load_average].
///
/// ```no_run
/// use sysinfo::System;
///
/// let s = System::new_all();
/// let load_avg = s.load_average();
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

macro_rules! xid {
    ($(#[$outer:meta])+ $name:ident, $type:ty $(, $trait:ty)?) => {
        $(#[$outer])+
        #[repr(transparent)]
        #[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
        pub struct $name(pub(crate) $type);

        impl std::ops::Deref for $name {
            type Target = $type;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        $(
        impl TryFrom<usize> for $name {
            type Error = <$type as TryFrom<usize>>::Error;

            fn try_from(t: usize) -> Result<Self, <$type as TryFrom<usize>>::Error> {
                Ok(Self(<$type>::try_from(t)?))
            }
        }

        impl $trait for $name {
            type Err = <$type as FromStr>::Err;

            fn from_str(t: &str) -> Result<Self, <$type as FromStr>::Err> {
                Ok(Self(<$type>::from_str(t)?))
            }
        }
        )?
    };
}

macro_rules! uid {
    ($type:ty$(, $trait:ty)?) => {
        xid!(
            /// A user id wrapping a platform specific type.
            Uid,
            $type
            $(, $trait)?
        );
    };
}

macro_rules! gid {
    ($type:ty) => {
        xid!(
            /// A group id wrapping a platform specific type.
            #[derive(Copy)]
            Gid,
            $type,
            FromStr
        );
    };
}

cfg_if::cfg_if! {
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
        uid!(libc::uid_t, FromStr);
        gid!(libc::gid_t);
    } else if #[cfg(windows)] {
        uid!(crate::windows::Sid);
        gid!(u32);
        // Manual implementation outside of the macro...
        impl FromStr for Uid {
            type Err = <crate::windows::Sid as FromStr>::Err;

            fn from_str(t: &str) -> Result<Self, Self::Err> {
                Ok(Self(t.parse()?))
            }
        }
    } else {
        uid!(u32, FromStr);
        gid!(u32);
    }
}

/// Type containing user information.
///
/// It is returned by [`Users`][crate::Users].
///
/// ```no_run
/// use sysinfo::Users;
///
/// let mut users = Users::new();
/// users.refresh_list();
/// for user in users.users() {
///     println!("{:?}", user);
/// }
/// ```
pub struct User {
    pub(crate) inner: UserInner,
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
            && self.group_id() == other.group_id()
            && self.name() == other.name()
    }
}

impl Eq for User {}

impl PartialOrd for User {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for User {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name().cmp(other.name())
    }
}

impl User {
    /// Returns the ID of the user.
    ///
    /// ```no_run
    /// use sysinfo::Users;
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     println!("{:?}", *user.id());
    /// }
    /// ```
    pub fn id(&self) -> &Uid {
        self.inner.id()
    }

    /// Returns the group ID of the user.
    ///
    /// ⚠️ This information is not set on Windows.  Windows doesn't have a `username` specific
    /// group assigned to the user. They do however have unique
    /// [Security Identifiers](https://docs.microsoft.com/en-us/windows/win32/secauthz/security-identifiers)
    /// made up of various [Components](https://docs.microsoft.com/en-us/windows/win32/secauthz/sid-components).
    /// Pieces of the SID may be a candidate for this field, but it doesn't map well to a single
    /// group ID.
    ///
    /// ```no_run
    /// use sysinfo::Users;
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     println!("{}", *user.group_id());
    /// }
    /// ```
    pub fn group_id(&self) -> Gid {
        self.inner.group_id()
    }

    /// Returns the name of the user.
    ///
    /// ```no_run
    /// use sysinfo::Users;
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     println!("{}", user.name());
    /// }
    /// ```
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Returns the groups of the user.
    ///
    /// ⚠️ This is computed every time this method is called.
    ///
    /// ```no_run
    /// use sysinfo::Users;
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     println!("{} is in {:?}", user.name(), user.groups());
    /// }
    /// ```
    pub fn groups(&self) -> Vec<Group> {
        self.inner.groups()
    }
}

/// Type containing group information.
///
/// It is returned by [`User::groups`].
///
/// ```no_run
/// use sysinfo::Users;
///
/// let mut users = Users::new();
///
/// for user in users.users() {
///     println!(
///         "user: (ID: {:?}, group ID: {:?}, name: {:?})",
///         user.id(),
///         user.group_id(),
///         user.name(),
///     );
///     for group in user.groups() {
///         println!("group: (ID: {:?}, name: {:?})", group.id(), group.name());
///     }
/// }
/// ```
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Group {
    pub(crate) id: Gid,
    pub(crate) name: String,
}

impl Group {
    /// Returns the ID of the group.
    ///
    /// ⚠️ This information is not set on Windows.
    ///
    /// ```no_run
    /// use sysinfo::Users;
    ///
    /// let mut users = Users::new();
    ///
    /// for user in users.users() {
    ///     for group in user.groups() {
    ///         println!("{:?}", group.id());
    ///     }
    /// }
    /// ```
    pub fn id(&self) -> &Gid {
        &self.id
    }

    /// Returns the name of the group.
    ///
    /// ```no_run
    /// use sysinfo::Users;
    ///
    /// let mut users = Users::new();
    ///
    /// for user in users.users() {
    ///     for group in user.groups() {
    ///         println!("{}", group.name());
    ///     }
    /// }
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Type containing read and written bytes.
///
/// It is returned by [`Process::disk_usage`][crate::Process::disk_usage].
///
/// ```no_run
/// use sysinfo::System;
///
/// let s = System::new_all();
/// for (pid, process) in s.processes() {
///     let disk_usage = process.disk_usage();
///     println!("[{}] read bytes   : new/total => {}/{} B",
///         pid,
///         disk_usage.read_bytes,
///         disk_usage.total_read_bytes,
///     );
///     println!("[{}] written bytes: new/total => {}/{} B",
///         pid,
///         disk_usage.written_bytes,
///         disk_usage.total_written_bytes,
///     );
/// }
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct DiskUsage {
    /// Total number of written bytes.
    pub total_written_bytes: u64,
    /// Number of written bytes since the last refresh.
    pub written_bytes: u64,
    /// Total number of read bytes.
    pub total_read_bytes: u64,
    /// Number of read bytes since the last refresh.
    pub read_bytes: u64,
}

/// Enum describing the different status of a process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
///         eprintln!("failed to get current pid: {}", e);
///     }
/// }
/// ```
#[allow(clippy::unnecessary_wraps)]
pub fn get_current_pid() -> Result<Pid, &'static str> {
    cfg_if::cfg_if! {
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

/// MAC address for network interface.
///
/// It is returned by [`NetworkData::mac_address`][crate::NetworkData::mac_address].
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct MacAddr(pub [u8; 6]);

impl MacAddr {
    /// A `MacAddr` with all bytes set to `0`.
    pub const UNSPECIFIED: Self = MacAddr([0; 6]);

    /// Checks if this `MacAddr` has all bytes equal to `0`.
    pub fn is_unspecified(&self) -> bool {
        self == &MacAddr::UNSPECIFIED
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data = &self.0;
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            data[0], data[1], data[2], data[3], data[4], data[5],
        )
    }
}

/// Interacting with components.
///
/// ```no_run
/// use sysinfo::Components;
///
/// let mut components = Components::new();
/// components.refresh_list();
/// for component in &components {
///     eprintln!("{component:?}");
/// }
/// ```
pub struct Components {
    pub(crate) inner: ComponentsInner,
}

impl Default for Components {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for Components {
    type Target = [Component];

    fn deref(&self) -> &Self::Target {
        self.components()
    }
}

impl std::ops::DerefMut for Components {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.components_mut()
    }
}

impl<'a> IntoIterator for &'a Components {
    type Item = &'a Component;
    type IntoIter = std::slice::Iter<'a, Component>;

    fn into_iter(self) -> Self::IntoIter {
        self.components().iter()
    }
}

impl<'a> IntoIterator for &'a mut Components {
    type Item = &'a mut Component;
    type IntoIter = std::slice::IterMut<'a, Component>;

    fn into_iter(self) -> Self::IntoIter {
        self.components_mut().iter_mut()
    }
}

impl Components {
    /// Creates a new [`Components`][crate::Components] type.
    ///
    /// ```no_run
    /// use sysinfo::Components;
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in &components {
    ///     eprintln!("{component:?}");
    /// }
    /// ```
    pub fn new() -> Self {
        Self {
            inner: ComponentsInner::new(),
        }
    }

    /// Returns the components list.
    ///
    /// ```no_run
    /// use sysinfo::Components;
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in components.components() {
    ///     eprintln!("{component:?}");
    /// }
    /// ```
    pub fn components(&self) -> &[Component] {
        self.inner.components()
    }

    /// Returns the components list.
    ///
    /// ```no_run
    /// use sysinfo::Components;
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in components.components_mut() {
    ///     component.refresh();
    ///     eprintln!("{component:?}");
    /// }
    /// ```
    pub fn components_mut(&mut self) -> &mut [Component] {
        self.inner.components_mut()
    }

    /// Sort the components list with the provided callback.
    ///
    /// Internally, it is using the [`slice::sort_unstable_by`] function, so please refer to it
    /// for implementation details.
    ///
    /// You can do the same without this method by calling:
    ///
    /// ```no_run
    /// use sysinfo::Components;
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// components.sort_by(|component1, component2| {
    ///     component2.label().partial_cmp(component2.label()).unwrap()
    /// });
    /// ```
    ///
    /// ⚠️ If you use [`Components::refresh_list`], you will need to call this method to sort the
    /// components again.
    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Component, &Component) -> std::cmp::Ordering,
    {
        self.components_mut().sort_unstable_by(compare);
    }

    /// Refreshes the listed components' information.
    ///
    /// ⚠️ If a component is added or removed, this method won't take it into account. Use
    /// [`Components::refresh_list`] instead.
    ///
    /// ⚠️ If you didn't call [`Components::refresh_list`] beforehand, this method will do
    /// nothing as the component list will be empty.
    ///
    /// ```no_run
    /// use sysinfo::Components;
    ///
    /// let mut components = Components::new();
    /// // We get the component list.
    /// components.refresh_list();
    /// // We wait some time...?
    /// components.refresh();
    /// ```
    pub fn refresh(&mut self) {
        for component in self.components_mut() {
            component.refresh();
        }
    }

    /// The component list will be emptied then completely recomputed.
    ///
    /// ```no_run
    /// use sysinfo::Components;
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// ```
    pub fn refresh_list(&mut self) {
        self.inner.refresh_list()
    }
}

/// Getting a component temperature information.
///
/// ```no_run
/// use sysinfo::Components;
///
/// let mut components = Components::new();
/// components.refresh_list();
/// for component in &components {
///     println!("{}°C", component.temperature());
/// }
/// ```
pub struct Component {
    pub(crate) inner: ComponentInner,
}

impl Component {
    /// Returns the temperature of the component (in celsius degree).
    ///
    /// ## Linux
    ///
    /// Returns `f32::NAN` if it failed to retrieve it.
    ///
    /// ```no_run
    /// use sysinfo::Components;
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in &components {
    ///     println!("{}°C", component.temperature());
    /// }
    /// ```
    pub fn temperature(&self) -> f32 {
        self.inner.temperature()
    }

    /// Returns the maximum temperature of the component (in celsius degree).
    ///
    /// Note: if `temperature` is higher than the current `max`,
    /// `max` value will be updated on refresh.
    ///
    /// ```no_run
    /// use sysinfo::Components;
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in &components {
    ///     println!("{}°C", component.max());
    /// }
    /// ```
    ///
    /// ## Linux
    ///
    /// May be computed by `sysinfo` from kernel.
    /// Returns `f32::NAN` if it failed to retrieve it.
    pub fn max(&self) -> f32 {
        self.inner.max()
    }

    /// Returns the highest temperature before the component halts (in celsius degree).
    ///
    /// ```no_run
    /// use sysinfo::Components;
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in &components {
    ///     println!("{:?}°C", component.critical());
    /// }
    /// ```
    ///
    /// ## Linux
    ///
    /// Critical threshold defined by chip or kernel.
    pub fn critical(&self) -> Option<f32> {
        self.inner.critical()
    }

    /// Returns the label of the component.
    ///
    /// ```no_run
    /// use sysinfo::Components;
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in &components {
    ///     println!("{}", component.label());
    /// }
    /// ```
    ///
    /// ## Linux
    ///
    /// Since components information is retrieved thanks to `hwmon`,
    /// the labels are generated as follows.
    /// Note: it may change and it was inspired by `sensors` own formatting.
    ///
    /// | name | label | device_model | id_sensor | Computed label by `sysinfo` |
    /// |---------|--------|------------|----------|----------------------|
    /// | ✓    | ✓    | ✓  | ✓ | `"{name} {label} {device_model} temp{id}"` |
    /// | ✓    | ✓    | ✗  | ✓ | `"{name} {label} {id}"` |
    /// | ✓    | ✗    | ✓  | ✓ | `"{name} {device_model}"` |
    /// | ✓    | ✗    | ✗  | ✓ | `"{name} temp{id}"` |
    pub fn label(&self) -> &str {
        self.inner.label()
    }

    /// Refreshes component.
    ///
    /// ```no_run
    /// use sysinfo::Components;
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in components.iter_mut() {
    ///     component.refresh();
    /// }
    /// ```
    pub fn refresh(&mut self) {
        self.inner.refresh()
    }
}

/// Contains all the methods of the [`Cpu`][crate::Cpu] struct.
///
/// ```no_run
/// use sysinfo::{System, RefreshKind, CpuRefreshKind};
///
/// let mut s = System::new_with_specifics(
///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
/// );
///
/// // Wait a bit because CPU usage is based on diff.
/// std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
/// // Refresh CPUs again.
/// s.refresh_cpu();
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
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    /// );
    ///
    /// // Wait a bit because CPU usage is based on diff.
    /// std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    /// // Refresh CPUs again.
    /// s.refresh_cpu();
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
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
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
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
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
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
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
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
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
mod tests {
    use super::{MacAddr, ProcessStatus};

    // This test only exists to ensure that the `Display` and `Debug` traits are implemented on the
    // `ProcessStatus` enum on all targets.
    #[test]
    fn check_display_impl_process_status() {
        println!("{} {:?}", ProcessStatus::Parked, ProcessStatus::Idle);
    }

    // Ensure that the `Display` and `Debug` traits are implemented on the `MacAddr` struct
    #[test]
    fn check_display_impl_mac_address() {
        println!(
            "{} {:?}",
            MacAddr([0x1, 0x2, 0x3, 0x4, 0x5, 0x6]),
            MacAddr([0xa, 0xb, 0xc, 0xd, 0xe, 0xf])
        );
    }

    #[test]
    fn check_mac_address_is_unspecified_true() {
        assert!(MacAddr::UNSPECIFIED.is_unspecified());
        assert!(MacAddr([0; 6]).is_unspecified());
    }

    #[test]
    fn check_mac_address_is_unspecified_false() {
        assert!(!MacAddr([1, 2, 3, 4, 5, 6]).is_unspecified());
    }

    // This test exists to ensure that the `TryFrom<usize>` and `FromStr` traits are implemented
    // on `Uid`, `Gid` and `Pid`.
    #[test]
    fn check_uid_gid_from_impls() {
        use std::convert::TryFrom;
        use std::str::FromStr;

        #[cfg(not(windows))]
        {
            assert!(crate::Uid::try_from(0usize).is_ok());
            assert!(crate::Uid::from_str("0").is_ok());
        }
        #[cfg(windows)]
        {
            assert!(crate::Uid::from_str("S-1-5-18").is_ok()); // SECURITY_LOCAL_SYSTEM_RID
            assert!(crate::Uid::from_str("0").is_err());
        }

        assert!(crate::Gid::try_from(0usize).is_ok());
        assert!(crate::Gid::from_str("0").is_ok());

        assert!(crate::Pid::try_from(0usize).is_ok());
        // If it doesn't panic, it's fine.
        let _ = crate::Pid::from(0);
        assert!(crate::Pid::from_str("0").is_ok());
    }
}
