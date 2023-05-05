// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    common::{Gid, MacAddr, Uid},
    sys::{Component, Cpu, Disk, Networks, Process},
};
use crate::{
    CpuRefreshKind, DiskKind, DiskUsage, LoadAvg, NetworksIter, Pid, ProcessRefreshKind,
    ProcessStatus, RefreshKind, Signal, User,
};

use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::Path;
use std::time::Duration;

/// Contains all the methods of the [`Disk`][crate::Disk] struct.
///
/// ```no_run
/// use sysinfo::{DiskExt, System, SystemExt};
///
/// let s = System::new();
/// for disk in s.disks() {
///     println!("{:?}: {:?}", disk.name(), disk.kind());
/// }
/// ```
pub trait DiskExt: Debug {
    /// Returns the kind of disk.
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for disk in s.disks() {
    ///     println!("{:?}", disk.kind());
    /// }
    /// ```
    fn kind(&self) -> DiskKind;

    /// Returns the disk name.
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for disk in s.disks() {
    ///     println!("{:?}", disk.name());
    /// }
    /// ```
    fn name(&self) -> &OsStr;

    /// Returns the file system used on this disk (so for example: `EXT4`, `NTFS`, etc...).
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for disk in s.disks() {
    ///     println!("{:?}", disk.file_system());
    /// }
    /// ```
    fn file_system(&self) -> &[u8];

    /// Returns the mount point of the disk (`/` for example).
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for disk in s.disks() {
    ///     println!("{:?}", disk.mount_point());
    /// }
    /// ```
    fn mount_point(&self) -> &Path;

    /// Returns the total disk size, in bytes.
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for disk in s.disks() {
    ///     println!("{}", disk.total_space());
    /// }
    /// ```
    fn total_space(&self) -> u64;

    /// Returns the available disk size, in bytes.
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for disk in s.disks() {
    ///     println!("{}", disk.available_space());
    /// }
    /// ```
    fn available_space(&self) -> u64;

    /// Returns `true` if the disk is removable.
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for disk in s.disks() {
    ///     println!("{}", disk.is_removable());
    /// }
    /// ```
    fn is_removable(&self) -> bool;

    /// Updates the disk' information.
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// for disk in s.disks_mut() {
    ///     disk.refresh();
    /// }
    /// ```
    fn refresh(&mut self) -> bool;
}

/// Contains all the methods of the [`Process`][crate::Process] struct.
pub trait ProcessExt: Debug {
    /// Sends [`Signal::Kill`] to the process (which is the only signal supported on all supported
    /// platforms by this crate).
    ///
    /// If you want to send another signal, take a look at [`ProcessExt::kill_with`].
    ///
    /// To get the list of the supported signals on this system, use
    /// [`SystemExt::SUPPORTED_SIGNALS`].
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     process.kill();
    /// }
    /// ```
    fn kill(&self) -> bool {
        self.kill_with(Signal::Kill).unwrap_or(false)
    }

    /// Sends the given `signal` to the process. If the signal doesn't exist on this platform,
    /// it'll do nothing and will return `None`. Otherwise it'll return if the signal was sent
    /// successfully.
    ///
    /// If you just want to kill the process, use [`ProcessExt::kill`] directly.
    ///
    /// To get the list of the supported signals on this system, use
    /// [`SystemExt::SUPPORTED_SIGNALS`].
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, Signal, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     if process.kill_with(Signal::Kill).is_none() {
    ///         eprintln!("This signal isn't supported on this platform");
    ///     }
    /// }
    /// ```
    fn kill_with(&self, signal: Signal) -> Option<bool>;

    /// Returns the name of the process.
    ///
    /// **⚠️ Important ⚠️**
    ///
    /// On **Linux**, there are two things to know about processes' name:
    ///  1. It is limited to 15 characters.
    ///  2. It is not always the exe name.
    ///
    /// If you are looking for a specific process, unless you know what you are doing, in most
    /// cases it's better to use [`ProcessExt::exe`] instead (which can be empty sometimes!).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}", process.name());
    /// }
    /// ```
    fn name(&self) -> &str;

    /// Returns the command line.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.cmd());
    /// }
    /// ```
    fn cmd(&self) -> &[String];

    /// Returns the path to the process.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
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
    fn exe(&self) -> &Path;

    /// Returns the PID of the process.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}", process.pid());
    /// }
    /// ```
    fn pid(&self) -> Pid;

    /// Returns the environment variables of the process.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.environ());
    /// }
    /// ```
    fn environ(&self) -> &[String];

    /// Returns the current working directory.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}", process.cwd().display());
    /// }
    /// ```
    fn cwd(&self) -> &Path;

    /// Returns the path of the root directory.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}", process.root().display());
    /// }
    /// ```
    fn root(&self) -> &Path;

    /// Returns the memory usage (in bytes).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{} bytes", process.memory());
    /// }
    /// ```
    fn memory(&self) -> u64;

    /// Returns the virtual memory usage (in bytes).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{} bytes", process.virtual_memory());
    /// }
    /// ```
    fn virtual_memory(&self) -> u64;

    /// Returns the parent PID.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.parent());
    /// }
    /// ```
    fn parent(&self) -> Option<Pid>;

    /// Returns the status of the process.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.status());
    /// }
    /// ```
    fn status(&self) -> ProcessStatus;

    /// Returns the time where the process was started (in seconds) from epoch.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("Started at {} seconds", process.start_time());
    /// }
    /// ```
    fn start_time(&self) -> u64;

    /// Returns for how much time the process has been running (in seconds).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("Running since {} seconds", process.run_time());
    /// }
    /// ```
    fn run_time(&self) -> u64;

    /// Returns the total CPU usage (in %). Notice that it might be bigger than 100 if run on a
    /// multi-core machine.
    ///
    /// If you want a value between 0% and 100%, divide the returned value by the number of CPUs.
    ///
    /// ⚠️ To start to have accurate CPU usage, a process needs to be refreshed **twice** because
    /// CPU usage computation is based on time diff (process time on a given time period divided by
    /// total system time on the same time period).
    ///
    /// ⚠️ If you want accurate CPU usage number, better leave a bit of time
    /// between two calls of this method (take a look at
    /// [`SystemExt::MINIMUM_CPU_UPDATE_INTERVAL`] for more information).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}%", process.cpu_usage());
    /// }
    /// ```
    fn cpu_usage(&self) -> f32;

    /// Returns number of bytes read and written to disk.
    ///
    /// ⚠️ On Windows and FreeBSD, this method actually returns **ALL** I/O read and written bytes.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new();
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
    fn disk_usage(&self) -> DiskUsage;

    /// Returns the ID of the owner user of this process or `None` if this information couldn't
    /// be retrieved. If you want to get the [`User`] from it, take a look at
    /// [`SystemExt::get_user_by_id`].
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     eprintln!("User id for process 1337: {:?}", process.user_id());
    /// }
    /// ```
    fn user_id(&self) -> Option<&Uid>;

    /// Returns the user ID of the effective owner of this process or `None` if this information
    /// couldn't be retrieved. If you want to get the [`User`] from it, take a look at
    /// [`SystemExt::get_user_by_id`].
    ///
    /// If you run something with `sudo`, the real user ID of the launched process will be the ID of
    /// the user you are logged in as but effective user ID will be `0` (i-e root).
    ///
    /// ⚠️ It always returns `None` on Windows.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     eprintln!("User id for process 1337: {:?}", process.effective_user_id());
    /// }
    /// ```
    fn effective_user_id(&self) -> Option<&Uid>;

    /// Returns the process group ID of the process.
    ///
    /// ⚠️ It always returns `None` on Windows.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     eprintln!("Group ID for process 1337: {:?}", process.group_id());
    /// }
    /// ```
    fn group_id(&self) -> Option<Gid>;

    /// Returns the effective group ID of the process.
    ///
    /// If you run something with `sudo`, the real group ID of the launched process will be the
    /// primary group ID you are logged in as but effective group ID will be `0` (i-e root).
    ///
    /// ⚠️ It always returns `None` on Windows.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     eprintln!("User id for process 1337: {:?}", process.effective_group_id());
    /// }
    /// ```
    fn effective_group_id(&self) -> Option<Gid>;

    /// Wait for process termination.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     eprintln!("Waiting for pid 1337");
    ///     process.wait();
    ///     eprintln!("Pid 1337 exited");
    /// }
    /// ```
    fn wait(&self);

    /// Returns the session ID for the current process or `None` if it couldn't be retrieved.
    ///
    /// ⚠️ This information is computed every time this method is called.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     eprintln!("Session ID for process 1337: {:?}", process.session_id());
    /// }
    /// ```
    fn session_id(&self) -> Option<Pid>;
}

/// Contains all the methods of the [`Cpu`][crate::Cpu] struct.
pub trait CpuExt: Debug {
    /// Returns this CPU's usage.
    ///
    /// Note: You'll need to refresh it at least twice (diff between the first and the second is
    /// how CPU usage is computed) at first if you want to have a non-zero value.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for cpu in s.cpus() {
    ///     println!("{}%", cpu.cpu_usage());
    /// }
    /// ```
    fn cpu_usage(&self) -> f32;

    /// Returns this CPU's name.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.name());
    /// }
    /// ```
    fn name(&self) -> &str;

    /// Returns the CPU's vendor id.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.vendor_id());
    /// }
    /// ```
    fn vendor_id(&self) -> &str;

    /// Returns the CPU's brand.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.brand());
    /// }
    /// ```
    fn brand(&self) -> &str;

    /// Returns the CPU's frequency.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.frequency());
    /// }
    /// ```
    fn frequency(&self) -> u64;
}

/// Contains all the methods of the [`System`][crate::System] type.
pub trait SystemExt: Sized + Debug + Default + Send + Sync {
    /// Returns `true` if this OS is supported. Please refer to the
    /// [crate-level documentation](index.html) to get the list of supported OSes.
    ///
    /// ```
    /// use sysinfo::{System, SystemExt};
    ///
    /// if System::IS_SUPPORTED {
    ///     println!("This OS is supported!");
    /// } else {
    ///     println!("This OS isn't supported (yet?).");
    /// }
    /// ```
    const IS_SUPPORTED: bool;

    /// Returns the list of the supported signals on this system (used by
    /// [`ProcessExt::kill_with`]).
    ///
    /// ```
    /// use sysinfo::{System, SystemExt};
    ///
    /// println!("supported signals: {:?}", System::SUPPORTED_SIGNALS);
    /// ```
    const SUPPORTED_SIGNALS: &'static [Signal];

    /// This is the minimum interval time used internally by `sysinfo` to refresh the CPU time.
    ///
    /// ⚠️ This value differs from one OS to another.
    ///
    /// Why is this constant even needed?
    ///
    /// If refreshed too often, the CPU usage of processes will be `0` whereas on Linux it'll
    /// always be the maximum value (`number of CPUs * 100`).
    const MINIMUM_CPU_UPDATE_INTERVAL: Duration;

    /// Creates a new [`System`] instance with nothing loaded. If you want to
    /// load components, network interfaces or the disks, you'll have to use the
    /// `refresh_*_list` methods. [`SystemExt::refresh_networks_list`] for
    /// example.
    ///
    /// Use the [`refresh_all`] method to update its internal information (or any of the `refresh_`
    /// method).
    ///
    /// [`System`]: crate::System
    /// [`refresh_all`]: #method.refresh_all
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// ```
    fn new() -> Self {
        Self::new_with_specifics(RefreshKind::new())
    }

    /// Creates a new [`System`] instance with everything loaded.
    ///
    /// It is an equivalent of [`SystemExt::new_with_specifics`]`(`[`RefreshKind::everything`]`())`.
    ///
    /// [`System`]: crate::System
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new_all();
    /// ```
    fn new_all() -> Self {
        Self::new_with_specifics(RefreshKind::everything())
    }

    /// Creates a new [`System`] instance and refresh the data corresponding to the
    /// given [`RefreshKind`].
    ///
    /// [`System`]: crate::System
    ///
    /// ```
    /// use sysinfo::{RefreshKind, System, SystemExt};
    ///
    /// // We want everything except disks.
    /// let mut system = System::new_with_specifics(RefreshKind::everything().without_disks_list());
    ///
    /// assert_eq!(system.disks().len(), 0);
    /// # if System::IS_SUPPORTED && !cfg!(feature = "apple-sandbox") {
    /// assert!(system.processes().len() > 0);
    /// # }
    ///
    /// // If you want the disks list afterwards, just call the corresponding
    /// // "refresh_disks_list":
    /// system.refresh_disks_list();
    /// let disks = system.disks();
    /// ```
    fn new_with_specifics(refreshes: RefreshKind) -> Self;

    /// Refreshes according to the given [`RefreshKind`]. It calls the corresponding
    /// "refresh_" methods.
    ///
    /// ```
    /// use sysinfo::{ProcessRefreshKind, RefreshKind, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    ///
    /// // Let's just update networks and processes:
    /// s.refresh_specifics(
    ///     RefreshKind::new().with_networks().with_processes(ProcessRefreshKind::everything()),
    /// );
    /// ```
    fn refresh_specifics(&mut self, refreshes: RefreshKind) {
        if refreshes.memory() {
            self.refresh_memory();
        }
        if let Some(kind) = refreshes.cpu() {
            self.refresh_cpu_specifics(kind);
        }
        if refreshes.components_list() {
            self.refresh_components_list();
        } else if refreshes.components() {
            self.refresh_components();
        }
        if refreshes.networks_list() {
            self.refresh_networks_list();
        } else if refreshes.networks() {
            self.refresh_networks();
        }
        if let Some(kind) = refreshes.processes() {
            self.refresh_processes_specifics(kind);
        }
        if refreshes.disks_list() {
            self.refresh_disks_list();
        } else if refreshes.disks() {
            self.refresh_disks();
        }
        if refreshes.users_list() {
            self.refresh_users_list();
        }
    }

    /// Refreshes all system, processes, disks and network interfaces information.
    ///
    /// Please note that it doesn't recompute disks list, components list, network interfaces
    /// list nor users list.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_all();
    /// ```
    fn refresh_all(&mut self) {
        self.refresh_system();
        self.refresh_processes();
        self.refresh_disks();
        self.refresh_networks();
    }

    /// Refreshes system information (RAM, swap, CPU usage and components' temperature).
    ///
    /// If you want some more specific refreshes, you might be interested into looking at
    /// [`refresh_memory`], [`refresh_cpu`] and [`refresh_components`].
    ///
    /// [`refresh_memory`]: SystemExt::refresh_memory
    /// [`refresh_cpu`]: SystemExt::refresh_memory
    /// [`refresh_components`]: SystemExt::refresh_components
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_system();
    /// ```
    fn refresh_system(&mut self) {
        self.refresh_memory();
        self.refresh_cpu();
        self.refresh_components();
    }

    /// Refreshes RAM and SWAP usage.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_memory();
    /// ```
    fn refresh_memory(&mut self);

    /// Refreshes CPUs information.
    ///
    /// ⚠️ Please note that the result will very likely be inaccurate at the first call.
    /// You need to call this method at least twice (with a bit of time between each call, like
    /// 200 ms, take a look at [`SystemExt::MINIMUM_CPU_UPDATE_INTERVAL`] for more information)
    /// to get accurate value as it uses previous results to compute the next value.
    ///
    /// Calling this method is the same as calling
    /// `refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage())`.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_cpu();
    /// ```
    fn refresh_cpu(&mut self) {
        self.refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage())
    }

    /// Refreshes CPUs specific information.
    ///
    /// Please note that it doesn't recompute disks list, components list, network interfaces
    /// list nor users list.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt, CpuRefreshKind};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_cpu_specifics(CpuRefreshKind::everything());
    /// ```
    fn refresh_cpu_specifics(&mut self, refresh_kind: CpuRefreshKind);

    /// Refreshes components' temperature.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_components();
    /// ```
    fn refresh_components(&mut self) {
        for component in self.components_mut() {
            component.refresh();
        }
    }

    /// Refreshes components list.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new();
    /// s.refresh_components_list();
    /// ```
    fn refresh_components_list(&mut self);

    /// Gets all processes and updates their information.
    ///
    /// It does the same as `system.refresh_processes_specifics(ProcessRefreshKind::everything())`.
    ///
    /// ⚠️ On Linux, `sysinfo` keeps the `stat` files open by default. You can change this behaviour
    /// by using [`set_open_files_limit`][crate::set_open_files_limit].
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_processes();
    /// ```
    fn refresh_processes(&mut self) {
        self.refresh_processes_specifics(ProcessRefreshKind::everything());
    }

    /// Gets all processes and updates the specified information.
    ///
    /// ⚠️ On Linux, `sysinfo` keeps the `stat` files open by default. You can change this behaviour
    /// by using [`set_open_files_limit`][crate::set_open_files_limit].
    ///
    /// ```no_run
    /// use sysinfo::{ProcessRefreshKind, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_processes_specifics(ProcessRefreshKind::new());
    /// ```
    fn refresh_processes_specifics(&mut self, refresh_kind: ProcessRefreshKind);

    /// Refreshes *only* the process corresponding to `pid`. Returns `false` if the process doesn't
    /// exist (it will **NOT** be removed from the processes if it doesn't exist anymore). If it
    /// isn't listed yet, it'll be added.
    ///
    /// It is the same as calling
    /// `sys.refresh_process_specifics(pid, ProcessRefreshKind::everything())`.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_process(Pid::from(1337));
    /// ```
    fn refresh_process(&mut self, pid: Pid) -> bool {
        self.refresh_process_specifics(pid, ProcessRefreshKind::everything())
    }

    /// Refreshes *only* the process corresponding to `pid`. Returns `false` if the process doesn't
    /// exist (it will **NOT** be removed from the processes if it doesn't exist anymore). If it
    /// isn't listed yet, it'll be added.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessRefreshKind, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_process_specifics(Pid::from(1337), ProcessRefreshKind::new());
    /// ```
    fn refresh_process_specifics(&mut self, pid: Pid, refresh_kind: ProcessRefreshKind) -> bool;

    /// Refreshes the listed disks' information.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_disks();
    /// ```
    fn refresh_disks(&mut self) {
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
    /// via [`SystemExt::refresh_disks_list`]. This is due to the fact that I/O function
    /// `statvfs` used by [`SystemExt::refresh_disks_list`] is blocking and
    /// [may hang](https://github.com/GuillaumeGomez/sysinfo/pull/876) in some cases,
    /// requiring to call `systemctl stop` to terminate the NFS service from the remote
    /// server in some cases.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_disks_list();
    /// ```
    fn refresh_disks_list(&mut self);

    /// Refreshes users list.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_users_list();
    /// ```
    fn refresh_users_list(&mut self);

    /// Refreshes networks data.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_networks();
    /// ```
    ///
    /// It is a shortcut for:
    ///
    /// ```no_run
    /// use sysinfo::{NetworksExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// let networks = s.networks_mut();
    /// networks.refresh();
    /// ```
    fn refresh_networks(&mut self) {
        self.networks_mut().refresh();
    }

    /// The network list will be updated: removing not existing anymore interfaces and adding new
    /// ones.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// s.refresh_networks_list();
    /// ```
    ///
    /// This is a shortcut for:
    ///
    /// ```no_run
    /// use sysinfo::{NetworksExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// let networks = s.networks_mut();
    /// networks.refresh_networks_list();
    /// ```
    fn refresh_networks_list(&mut self) {
        self.networks_mut().refresh_networks_list();
    }

    /// Returns the process list.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// for (pid, process) in s.processes() {
    ///     println!("{} {}", pid, process.name());
    /// }
    /// ```
    fn processes(&self) -> &HashMap<Pid, Process>;

    /// Returns the process corresponding to the given `pid` or `None` if no such process exists.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}", process.name());
    /// }
    /// ```
    fn process(&self, pid: Pid) -> Option<&Process>;

    /// Returns an iterator of process containing the given `name`.
    ///
    /// If you want only the processes with exactly the given `name`, take a look at
    /// [`SystemExt::processes_by_exact_name`].
    ///
    /// **⚠️ Important ⚠️**
    ///
    /// On **Linux**, there are two things to know about processes' name:
    ///  1. It is limited to 15 characters.
    ///  2. It is not always the exe name.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// for process in s.processes_by_name("htop") {
    ///     println!("{} {}", process.pid(), process.name());
    /// }
    /// ```
    // FIXME: replace the returned type with `impl Iterator<Item = &Process>` when it's supported!
    fn processes_by_name<'a: 'b, 'b>(
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
    /// [`SystemExt::processes_by_name`].
    ///
    /// **⚠️ Important ⚠️**
    ///
    /// On **Linux**, there are two things to know about processes' name:
    ///  1. It is limited to 15 characters.
    ///  2. It is not always the exe name.
    ///
    /// ```no_run
    /// use sysinfo::{ProcessExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// for process in s.processes_by_exact_name("htop") {
    ///     println!("{} {}", process.pid(), process.name());
    /// }
    /// ```
    // FIXME: replace the returned type with `impl Iterator<Item = &Process>` when it's supported!
    fn processes_by_exact_name<'a: 'b, 'b>(
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
    /// To have up-to-date information, you need to call [`SystemExt::refresh_cpu`] or
    /// [`SystemExt::refresh_specifics`] with `cpu` enabled.
    ///
    /// ```no_run
    /// use sysinfo::{CpuRefreshKind, CpuExt, RefreshKind, System, SystemExt};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// println!("{}%", s.global_cpu_info().cpu_usage());
    /// ```
    fn global_cpu_info(&self) -> &Cpu;

    /// Returns the list of the CPUs.
    ///
    /// By default, the list of CPUs is empty until you call [`SystemExt::refresh_cpu`] or
    /// [`SystemExt::refresh_specifics`] with `cpu` enabled.
    ///
    /// ```no_run
    /// use sysinfo::{CpuRefreshKind, CpuExt, RefreshKind, System, SystemExt};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// for cpu in s.cpus() {
    ///     println!("{}%", cpu.cpu_usage());
    /// }
    /// ```
    fn cpus(&self) -> &[Cpu];

    /// Returns the number of physical cores on the CPU or `None` if it couldn't get it.
    ///
    /// In case there are multiple CPUs, it will combine the physical core count of all the CPUs.
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("{:?}", s.physical_core_count());
    /// ```
    fn physical_core_count(&self) -> Option<usize>;

    /// Returns the RAM size in bytes.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.total_memory());
    /// ```
    fn total_memory(&self) -> u64;

    /// Returns the amount of free RAM in bytes.
    ///
    /// Generally, "free" memory refers to unallocated memory whereas "available" memory refers to
    /// memory that is available for (re)use.
    ///
    /// Side note: Windows doesn't report "free" memory so this method returns the same value
    /// as [`get_available_memory`](#tymethod.available_memory).
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.free_memory());
    /// ```
    fn free_memory(&self) -> u64;

    /// Returns the amount of available RAM in bytes.
    ///
    /// Generally, "free" memory refers to unallocated memory whereas "available" memory refers to
    /// memory that is available for (re)use.
    ///
    /// ⚠️ Windows and FreeBSD don't report "available" memory so [`SystemExt::free_memory`]
    /// returns the same value as this method.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.available_memory());
    /// ```
    fn available_memory(&self) -> u64;

    /// Returns the amount of used RAM in bytes.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.used_memory());
    /// ```
    fn used_memory(&self) -> u64;

    /// Returns the SWAP size in bytes.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.total_swap());
    /// ```
    fn total_swap(&self) -> u64;

    /// Returns the amount of free SWAP in bytes.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.free_swap());
    /// ```
    fn free_swap(&self) -> u64;

    /// Returns the amount of used SWAP in bytes.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new_all();
    /// println!("{} bytes", s.used_swap());
    /// ```
    fn used_swap(&self) -> u64;

    /// Returns the components list.
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// for component in s.components() {
    ///     println!("{}: {}°C", component.label(), component.temperature());
    /// }
    /// ```
    fn components(&self) -> &[Component];

    /// Returns a mutable components list.
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// for component in s.components_mut() {
    ///     component.refresh();
    /// }
    /// ```
    fn components_mut(&mut self) -> &mut [Component];

    /// Returns the users list.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt, UserExt};
    ///
    /// let mut s = System::new_all();
    /// for user in s.users() {
    ///     println!("{} is in {} groups", user.name(), user.groups().len());
    /// }
    /// ```
    fn users(&self) -> &[User];

    /// Returns the disks list.
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// for disk in s.disks() {
    ///     println!("{:?}", disk.name());
    /// }
    /// ```
    fn disks(&self) -> &[Disk];

    /// Returns the disks list.
    ///
    /// ```no_run
    /// use sysinfo::{DiskExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// for disk in s.disks_mut() {
    ///     disk.refresh();
    /// }
    /// ```
    fn disks_mut(&mut self) -> &mut [Disk];

    /// Sort the disk list with the provided callback.
    ///
    /// Internally, it is using the [`slice::sort_unstable_by`] function, so please refer to it
    /// for implementation details.
    ///
    /// ⚠️ If you use [`SystemExt::refresh_disks_list`], you need to use this method before using
    /// [`SystemExt::disks`] or [`SystemExt::disks_mut`] if you want them to be sorted.
    fn sort_disks_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Disk, &Disk) -> std::cmp::Ordering;

    /// Returns the network interfaces object.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, data) in networks {
    ///     println!(
    ///         "[{}] in: {}, out: {}",
    ///         interface_name,
    ///         data.received(),
    ///         data.transmitted(),
    ///     );
    /// }
    /// ```
    fn networks(&self) -> &Networks;

    /// Returns a mutable access to network interfaces.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// let networks = s.networks_mut();
    /// networks.refresh_networks_list();
    /// ```
    fn networks_mut(&mut self) -> &mut Networks;

    /// Returns system uptime (in seconds).
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new_all();
    /// println!("System running since {} seconds", s.uptime());
    /// ```
    fn uptime(&self) -> u64;

    /// Returns the time (in seconds) when the system booted since UNIX epoch.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("System booted at {} seconds", s.boot_time());
    /// ```
    fn boot_time(&self) -> u64;

    /// Returns the system load average value.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
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
    fn load_average(&self) -> LoadAvg;

    /// Returns the system name.
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("OS: {:?}", s.name());
    /// ```
    fn name(&self) -> Option<String>;

    /// Returns the system's kernel version.
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("kernel version: {:?}", s.kernel_version());
    /// ```
    fn kernel_version(&self) -> Option<String>;

    /// Returns the system version (e.g. for MacOS this will return 11.1 rather than the kernel version).
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("OS version: {:?}", s.os_version());
    /// ```
    fn os_version(&self) -> Option<String>;

    /// Returns the system long os version (e.g "MacOS 11.2 BigSur").
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("Long OS Version: {:?}", s.long_os_version());
    /// ```
    fn long_os_version(&self) -> Option<String>;

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
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("Distribution ID: {:?}", s.distribution_id());
    /// ```
    fn distribution_id(&self) -> String;

    /// Returns the system hostname based off DNS
    ///
    /// **Important**: this information is computed every time this function is called.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt};
    ///
    /// let s = System::new();
    /// println!("Hostname: {:?}", s.host_name());
    /// ```
    fn host_name(&self) -> Option<String>;

    /// Returns the [`User`] matching the given `user_id`.
    ///
    /// **Important**: The user list must be filled before using this method, otherwise it will
    /// always return `None` (through the `refresh_*` methods).
    ///
    /// It is a shorthand for:
    ///
    /// ```ignore
    /// let s = System::new_all();
    /// s.users().find(|user| user.id() == user_id);
    /// ```
    ///
    /// Full example:
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    ///
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     if let Some(user_id) = process.user_id() {
    ///         eprintln!("User for process 1337: {:?}", s.get_user_by_id(user_id));
    ///     }
    /// }
    /// ```
    fn get_user_by_id(&self, user_id: &Uid) -> Option<&User> {
        self.users().iter().find(|user| user.id() == user_id)
    }
}

/// Getting volume of received and transmitted data.
pub trait NetworkExt: Debug {
    /// Returns the number of received bytes since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, network) in networks {
    ///     println!("in: {} B", network.received());
    /// }
    /// ```
    fn received(&self) -> u64;

    /// Returns the total number of received bytes.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, network) in networks {
    ///     println!("in: {} B", network.total_received());
    /// }
    /// ```
    fn total_received(&self) -> u64;

    /// Returns the number of transmitted bytes since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, network) in networks {
    ///     println!("out: {} B", network.transmitted());
    /// }
    /// ```
    fn transmitted(&self) -> u64;

    /// Returns the total number of transmitted bytes.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, network) in networks {
    ///     println!("out: {} B", network.total_transmitted());
    /// }
    /// ```
    fn total_transmitted(&self) -> u64;

    /// Returns the number of incoming packets since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, network) in networks {
    ///     println!("in: {}", network.packets_received());
    /// }
    /// ```
    fn packets_received(&self) -> u64;

    /// Returns the total number of incoming packets.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, network) in networks {
    ///     println!("in: {}", network.total_packets_received());
    /// }
    /// ```
    fn total_packets_received(&self) -> u64;

    /// Returns the number of outcoming packets since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, network) in networks {
    ///     println!("out: {}", network.packets_transmitted());
    /// }
    /// ```
    fn packets_transmitted(&self) -> u64;

    /// Returns the total number of outcoming packets.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, network) in networks {
    ///     println!("out: {}", network.total_packets_transmitted());
    /// }
    /// ```
    fn total_packets_transmitted(&self) -> u64;

    /// Returns the number of incoming errors since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, network) in networks {
    ///     println!("in: {}", network.errors_on_received());
    /// }
    /// ```
    fn errors_on_received(&self) -> u64;

    /// Returns the total number of incoming errors.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, network) in networks {
    ///     println!("in: {}", network.total_errors_on_received());
    /// }
    /// ```
    fn total_errors_on_received(&self) -> u64;

    /// Returns the number of outcoming errors since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, network) in networks {
    ///     println!("out: {}", network.errors_on_transmitted());
    /// }
    /// ```
    fn errors_on_transmitted(&self) -> u64;

    /// Returns the total number of outcoming errors.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, network) in networks {
    ///     println!("out: {}", network.total_errors_on_transmitted());
    /// }
    /// ```
    fn total_errors_on_transmitted(&self) -> u64;

    /// Returns the MAC address associated to current interface.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, network) in networks {
    ///     println!("MAC address: {}", network.mac_address());
    /// }
    /// ```
    fn mac_address(&self) -> MacAddr;
}

/// Interacting with network interfaces.
pub trait NetworksExt: Debug {
    /// Returns an iterator over the network interfaces.
    ///
    /// ```no_run
    /// use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// let networks = s.networks();
    /// for (interface_name, network) in networks {
    ///     println!("in: {} B", network.received());
    /// }
    /// ```
    fn iter(&self) -> NetworksIter;

    /// Refreshes the network interfaces list.
    ///
    /// ```no_run
    /// use sysinfo::{NetworksExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// let networks = s.networks_mut();
    /// networks.refresh_networks_list();
    /// ```
    fn refresh_networks_list(&mut self);

    /// Refreshes the network interfaces' content.
    ///
    /// ```no_run
    /// use sysinfo::{NetworksExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// let networks = s.networks_mut();
    /// networks.refresh();
    /// ```
    fn refresh(&mut self);
}

/// Getting a component temperature information.
pub trait ComponentExt: Debug {
    /// Returns the temperature of the component (in celsius degree).
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// for component in s.components() {
    ///     println!("{}°C", component.temperature());
    /// }
    /// ```
    ///
    /// ## Linux
    ///
    /// Returns `f32::NAN` if it failed to retrieve it.
    fn temperature(&self) -> f32;

    /// Returns the maximum temperature of the component (in celsius degree).
    ///
    /// Note: if `temperature` is higher than the current `max`,
    /// `max` value will be updated on refresh.
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// for component in s.components() {
    ///     println!("{}°C", component.max());
    /// }
    /// ```
    ///
    /// ## Linux
    ///
    /// May be computed by `sysinfo` from kernel.
    /// Returns `f32::NAN` if it failed to retrieve it.
    fn max(&self) -> f32;

    /// Returns the highest temperature before the component halts (in celsius degree).
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// for component in s.components() {
    ///     println!("{:?}°C", component.critical());
    /// }
    /// ```
    ///
    /// ## Linux
    ///
    /// Critical threshold defined by chip or kernel.
    fn critical(&self) -> Option<f32>;

    /// Returns the label of the component.
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, System, SystemExt};
    ///
    /// let s = System::new_all();
    /// for component in s.components() {
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
    fn label(&self) -> &str;

    /// Refreshes component.
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, System, SystemExt};
    ///
    /// let mut s = System::new_all();
    /// for component in s.components_mut() {
    ///     component.refresh();
    /// }
    /// ```
    fn refresh(&mut self);
}

/// Getting information for a user.
///
/// It is returned from [`SystemExt::users`].
///
/// ```no_run
/// use sysinfo::{System, SystemExt, UserExt};
///
/// let mut s = System::new_all();
/// for user in s.users() {
///     println!("{} is in {} groups", user.name(), user.groups().len());
/// }
/// ```
pub trait UserExt: Debug {
    /// Return the user id of the user.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt, UserExt};
    ///
    /// let mut s = System::new_all();
    /// for user in s.users() {
    ///     println!("{:?}", *user.id());
    /// }
    /// ```
    fn id(&self) -> &Uid;

    /// Return the group id of the user.
    ///
    /// *NOTE* - On Windows, this value defaults to 0.  Windows doesn't have a `username` specific group assigned to the user.
    /// They do however have unique [Security Identifiers](https://docs.microsoft.com/en-us/windows/win32/secauthz/security-identifiers)
    /// made up of various [Components](https://docs.microsoft.com/en-us/windows/win32/secauthz/sid-components).
    /// Pieces of the SID may be a candidate for this field, but it doesn't map well to a single group id.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt, UserExt};
    ///
    /// let mut s = System::new_all();
    /// for user in s.users() {
    ///     println!("{}", *user.group_id());
    /// }
    /// ```
    fn group_id(&self) -> Gid;

    /// Returns the name of the user.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt, UserExt};
    ///
    /// let mut s = System::new_all();
    /// for user in s.users() {
    ///     println!("{}", user.name());
    /// }
    /// ```
    fn name(&self) -> &str;

    /// Returns the groups of the user.
    ///
    /// ```no_run
    /// use sysinfo::{System, SystemExt, UserExt};
    ///
    /// let mut s = System::new_all();
    /// for user in s.users() {
    ///     println!("{} is in {:?}", user.name(), user.groups());
    /// }
    /// ```
    fn groups(&self) -> &[String];
}
