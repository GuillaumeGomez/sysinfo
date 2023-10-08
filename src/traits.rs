// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    common::{Gid, MacAddr, Uid},
    sys::Component,
};
use crate::{DiskUsage, Group, NetworksIter, Pid, ProcessStatus, Signal, User};

use std::fmt::Debug;
use std::path::Path;

/// Contains all the methods of the [`Process`][crate::Process] struct.
///
/// ```no_run
/// use sysinfo::{Pid, ProcessExt, System};
///
/// let s = System::new_all();
/// if let Some(process) = s.process(Pid::from(1337)) {
///     println!("{}", process.name());
/// }
/// ```
pub trait ProcessExt: Debug {
    /// Sends [`Signal::Kill`] to the process (which is the only signal supported on all supported
    /// platforms by this crate).
    ///
    /// If you want to send another signal, take a look at [`ProcessExt::kill_with`].
    ///
    /// To get the list of the supported signals on this system, use
    /// [`SUPPORTED_SIGNALS`][crate::SUPPORTED_SIGNALS].
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
    ///
    /// let s = System::new_all();
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
    /// [`SUPPORTED_SIGNALS`][crate::SUPPORTED_SIGNALS].
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, Signal, System};
    ///
    /// let s = System::new_all();
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
    /// use sysinfo::{Pid, ProcessExt, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}", process.name());
    /// }
    /// ```
    fn name(&self) -> &str;

    /// Returns the command line.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.cmd());
    /// }
    /// ```
    fn cmd(&self) -> &[String];

    /// Returns the path to the process.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
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
    fn exe(&self) -> &Path;

    /// Returns the PID of the process.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}", process.pid());
    /// }
    /// ```
    fn pid(&self) -> Pid;

    /// Returns the environment variables of the process.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.environ());
    /// }
    /// ```
    fn environ(&self) -> &[String];

    /// Returns the current working directory.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}", process.cwd().display());
    /// }
    /// ```
    fn cwd(&self) -> &Path;

    /// Returns the path of the root directory.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{}", process.root().display());
    /// }
    /// ```
    fn root(&self) -> &Path;

    /// Returns the memory usage (in bytes).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{} bytes", process.memory());
    /// }
    /// ```
    fn memory(&self) -> u64;

    /// Returns the virtual memory usage (in bytes).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{} bytes", process.virtual_memory());
    /// }
    /// ```
    fn virtual_memory(&self) -> u64;

    /// Returns the parent PID.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.parent());
    /// }
    /// ```
    fn parent(&self) -> Option<Pid>;

    /// Returns the status of the process.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("{:?}", process.status());
    /// }
    /// ```
    fn status(&self) -> ProcessStatus;

    /// Returns the time where the process was started (in seconds) from epoch.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
    ///
    /// let s = System::new_all();
    /// if let Some(process) = s.process(Pid::from(1337)) {
    ///     println!("Started at {} seconds", process.start_time());
    /// }
    /// ```
    fn start_time(&self) -> u64;

    /// Returns for how much time the process has been running (in seconds).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
    ///
    /// let s = System::new_all();
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
    /// [`MINIMUM_CPU_UPDATE_INTERVAL`][crate::MINIMUM_CPU_UPDATE_INTERVAL] for more information).
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
    ///
    /// let s = System::new_all();
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
    /// use sysinfo::{Pid, ProcessExt, System};
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
    fn disk_usage(&self) -> DiskUsage;

    /// Returns the ID of the owner user of this process or `None` if this information couldn't
    /// be retrieved. If you want to get the [`User`] from it, take a look at
    /// [`UsersExt::get_user_by_id`].
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
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
    /// [`UsersExt::get_user_by_id`].
    ///
    /// If you run something with `sudo`, the real user ID of the launched process will be the ID of
    /// the user you are logged in as but effective user ID will be `0` (i-e root).
    ///
    /// ⚠️ It always returns `None` on Windows.
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System};
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
    /// use sysinfo::{Pid, ProcessExt, System};
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
    /// use sysinfo::{Pid, ProcessExt, System};
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
    /// use sysinfo::{Pid, ProcessExt, System};
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
    /// use sysinfo::{Pid, ProcessExt, System};
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
///
/// ```no_run
/// use sysinfo::{CpuExt, System, RefreshKind, CpuRefreshKind};
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
pub trait CpuExt: Debug {
    /// Returns this CPU's usage.
    ///
    /// Note: You'll need to refresh it at least twice (diff between the first and the second is
    /// how CPU usage is computed) at first if you want to have a non-zero value.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, RefreshKind, CpuRefreshKind};
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
    fn cpu_usage(&self) -> f32;

    /// Returns this CPU's name.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, RefreshKind, CpuRefreshKind};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.name());
    /// }
    /// ```
    fn name(&self) -> &str;

    /// Returns the CPU's vendor id.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, RefreshKind, CpuRefreshKind};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.vendor_id());
    /// }
    /// ```
    fn vendor_id(&self) -> &str;

    /// Returns the CPU's brand.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, RefreshKind, CpuRefreshKind};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.brand());
    /// }
    /// ```
    fn brand(&self) -> &str;

    /// Returns the CPU's frequency.
    ///
    /// ```no_run
    /// use sysinfo::{CpuExt, System, RefreshKind, CpuRefreshKind};
    ///
    /// let s = System::new_with_specifics(
    ///     RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    /// );
    /// for cpu in s.cpus() {
    ///     println!("{}", cpu.frequency());
    /// }
    /// ```
    fn frequency(&self) -> u64;
}

/// Getting volume of received and transmitted data.
///
/// ```no_run
/// use sysinfo::{Networks, NetworkExt, NetworksExt};
///
/// let mut networks = Networks::new();
/// networks.refresh_list();
/// for (interface_name, network) in &networks {
///     println!("[{interface_name}] {network:?}");
/// }
/// ```
pub trait NetworkExt: Debug {
    /// Returns the number of received bytes since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt, NetworksExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {} B", network.received());
    /// }
    /// ```
    fn received(&self) -> u64;

    /// Returns the total number of received bytes.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt, NetworksExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {} B", network.total_received());
    /// }
    /// ```
    fn total_received(&self) -> u64;

    /// Returns the number of transmitted bytes since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt, NetworksExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {} B", network.transmitted());
    /// }
    /// ```
    fn transmitted(&self) -> u64;

    /// Returns the total number of transmitted bytes.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt, NetworksExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {} B", network.total_transmitted());
    /// }
    /// ```
    fn total_transmitted(&self) -> u64;

    /// Returns the number of incoming packets since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt, NetworksExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {}", network.packets_received());
    /// }
    /// ```
    fn packets_received(&self) -> u64;

    /// Returns the total number of incoming packets.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt, NetworksExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {}", network.total_packets_received());
    /// }
    /// ```
    fn total_packets_received(&self) -> u64;

    /// Returns the number of outcoming packets since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt, NetworksExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {}", network.packets_transmitted());
    /// }
    /// ```
    fn packets_transmitted(&self) -> u64;

    /// Returns the total number of outcoming packets.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt, NetworksExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {}", network.total_packets_transmitted());
    /// }
    /// ```
    fn total_packets_transmitted(&self) -> u64;

    /// Returns the number of incoming errors since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt, NetworksExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {}", network.errors_on_received());
    /// }
    /// ```
    fn errors_on_received(&self) -> u64;

    /// Returns the total number of incoming errors.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt, NetworksExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("in: {}", network.total_errors_on_received());
    /// }
    /// ```
    fn total_errors_on_received(&self) -> u64;

    /// Returns the number of outcoming errors since the last refresh.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt, NetworksExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {}", network.errors_on_transmitted());
    /// }
    /// ```
    fn errors_on_transmitted(&self) -> u64;

    /// Returns the total number of outcoming errors.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt, NetworksExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("out: {}", network.total_errors_on_transmitted());
    /// }
    /// ```
    fn total_errors_on_transmitted(&self) -> u64;

    /// Returns the MAC address associated to current interface.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt, NetworksExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("MAC address: {}", network.mac_address());
    /// }
    /// ```
    fn mac_address(&self) -> MacAddr;
}

/// Interacting with network interfaces.
///
/// ```no_run
/// use sysinfo::{Networks, NetworksExt};
///
/// let mut networks = Networks::new();
/// networks.refresh_list();
/// for (interface_name, network) in &networks {
///     println!("[{interface_name}]: {network:?}");
/// }
/// ```
pub trait NetworksExt: Debug {
    /// Creates a new [`Networks`][crate::Networks] type.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworksExt};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// for (interface_name, network) in &networks {
    ///     println!("[{interface_name}]: {network:?}");
    /// }
    /// ```
    fn new() -> Self;

    /// Returns an iterator over the network interfaces.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworkExt, NetworksExt, System};
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
    fn iter(&self) -> NetworksIter;

    /// Refreshes the network interfaces list.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworksExt, System};
    ///
    /// let mut networks = Networks::new();
    /// networks.refresh_list();
    /// ```
    fn refresh_list(&mut self);

    /// Refreshes the network interfaces' content. If you didn't run [`NetworksExt::refresh_list`]
    /// before, calling this method won't do anything as no interfaces are present.
    ///
    /// ⚠️ If a user is added or removed, this method won't take it into account. Use
    /// [`NetworksExt::refresh_list`] instead.
    ///
    /// ⚠️ If you didn't call [`NetworksExt::refresh_list`] beforehand, this method will do nothing
    /// as the network list will be empty.
    ///
    /// ```no_run
    /// use sysinfo::{Networks, NetworksExt, System};
    ///
    /// let mut networks = Networks::new();
    /// // Refreshes the network interfaces list.
    /// networks.refresh_list();
    /// // Wait some time...? Then refresh the data of each network.
    /// networks.refresh();
    /// ```
    fn refresh(&mut self);
}

/// Getting a component temperature information.
///
/// ```no_run
/// use sysinfo::{ComponentExt, Components, ComponentsExt};
///
/// let mut components = Components::new();
/// components.refresh_list();
/// for component in components.iter() {
///     println!("{}°C", component.temperature());
/// }
/// ```
pub trait ComponentExt: Debug {
    /// Returns the temperature of the component (in celsius degree).
    ///
    /// ## Linux
    ///
    /// Returns `f32::NAN` if it failed to retrieve it.
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in components.iter() {
    ///     println!("{}°C", component.temperature());
    /// }
    /// ```
    fn temperature(&self) -> f32;

    /// Returns the maximum temperature of the component (in celsius degree).
    ///
    /// Note: if `temperature` is higher than the current `max`,
    /// `max` value will be updated on refresh.
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in components.iter() {
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
    /// use sysinfo::{ComponentExt, Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in components.iter() {
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
    /// use sysinfo::{ComponentExt, Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in components.iter() {
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
    /// use sysinfo::{ComponentExt, Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in components.iter_mut() {
    ///     component.refresh();
    /// }
    /// ```
    fn refresh(&mut self);
}

/// Interacting with components.
///
/// ```no_run
/// use sysinfo::{Components, ComponentsExt};
///
/// let mut components = Components::new();
/// components.refresh_list();
/// for component in components.iter() {
///     eprintln!("{component:?}");
/// }
/// ```
pub trait ComponentsExt: Debug {
    /// Creates a new [`Components`][crate::Components] type.
    ///
    /// ```no_run
    /// use sysinfo::{Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in components.iter() {
    ///     eprintln!("{component:?}");
    /// }
    /// ```
    fn new() -> Self;

    /// Returns the components list.
    ///
    /// ```no_run
    /// use sysinfo::{Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in components.components() {
    ///     eprintln!("{component:?}");
    /// }
    /// ```
    fn components(&self) -> &[Component];

    /// Returns the components list.
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// for component in components.components_mut() {
    ///     component.refresh();
    ///     eprintln!("{component:?}");
    /// }
    /// ```
    fn components_mut(&mut self) -> &mut [Component];

    /// Sort the components list with the provided callback.
    ///
    /// Internally, it is using the [`slice::sort_unstable_by`] function, so please refer to it
    /// for implementation details.
    ///
    /// You can do the same without this method by calling:
    ///
    /// ```no_run
    /// use sysinfo::{ComponentExt, Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// components.sort_by(|component1, component2| {
    ///     component2.label().partial_cmp(component2.label()).unwrap()
    /// });
    /// ```
    ///
    /// ⚠️ If you use [`ComponentsExt::refresh_list`], you will need to call this method to sort the
    /// components again.
    fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Component, &Component) -> std::cmp::Ordering,
    {
        self.components_mut().sort_unstable_by(compare);
    }

    /// Refreshes the listed components' information.
    ///
    /// ⚠️ If a component is added or removed, this method won't take it into account. Use
    /// [`ComponentsExt::refresh_list`] instead.
    ///
    /// ⚠️ If you didn't call [`ComponentsExt::refresh_list`] beforehand, this method will do
    /// nothing as the component list will be empty.
    ///
    /// ```no_run
    /// use sysinfo::{Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// // We get the component list.
    /// components.refresh_list();
    /// // We wait some time...?
    /// components.refresh();
    /// ```
    fn refresh(&mut self) {
        for component in self.components_mut() {
            component.refresh();
        }
    }

    /// The component list will be emptied then completely recomputed.
    ///
    /// ```no_run
    /// use sysinfo::{Components, ComponentsExt};
    ///
    /// let mut components = Components::new();
    /// components.refresh_list();
    /// ```
    fn refresh_list(&mut self);
}

/// Getting information for a user.
///
/// It is returned from [`UsersExt::users`].
///
/// ```no_run
/// use sysinfo::{Users, UsersExt, UserExt};
///
/// let mut users = Users::new();
/// users.refresh_list();
/// for user in users.users() {
///     println!("{} is in {} groups", user.name(), user.groups().len());
/// }
/// ```
pub trait UserExt: Debug + PartialEq + Eq + PartialOrd + Ord {
    /// Returns the ID of the user.
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt, UserExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     println!("{:?}", *user.id());
    /// }
    /// ```
    fn id(&self) -> &Uid;

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
    /// use sysinfo::{Users, UsersExt, UserExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     println!("{}", *user.group_id());
    /// }
    /// ```
    fn group_id(&self) -> Gid;

    /// Returns the name of the user.
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt, UserExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     println!("{}", user.name());
    /// }
    /// ```
    fn name(&self) -> &str;

    /// Returns the groups of the user.
    ///
    /// ⚠️ This is computed every time this method is called.
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt, UserExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     println!("{} is in {:?}", user.name(), user.groups());
    /// }
    /// ```
    fn groups(&self) -> Vec<Group>;
}

/// Interacting with users.
///
/// ```no_run
/// use sysinfo::{Users, UsersExt};
///
/// let mut users = Users::new();
/// users.refresh_list();
/// for user in users.users() {
///     eprintln!("{user:?}");
/// }
/// ```
pub trait UsersExt: Debug {
    /// Creates a new [`Components`][crate::Components] type.
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     eprintln!("{user:?}");
    /// }
    /// ```
    fn new() -> Self;

    /// Returns the users list.
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// for user in users.users() {
    ///     eprintln!("{user:?}");
    /// }
    /// ```
    fn users(&self) -> &[User];

    /// Returns the users list.
    ///
    /// ```no_run
    /// use sysinfo::{UserExt, Users, UsersExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// users.users_mut().sort_by(|user1, user2| {
    ///     user1.name().partial_cmp(user2.name()).unwrap()
    /// });
    /// ```
    fn users_mut(&mut self) -> &mut [User];

    /// Sort the users list with the provided callback.
    ///
    /// Internally, it is using the [`slice::sort_unstable_by`] function, so please refer to it
    /// for implementation details.
    ///
    /// You can do the same without this method by calling:
    ///
    /// ```no_run
    /// use sysinfo::{UserExt, Users, UsersExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// users.sort_by(|user1, user2| {
    ///     user1.name().partial_cmp(user2.name()).unwrap()
    /// });
    /// ```
    ///
    /// ⚠️ If you use [`UsersExt::refresh_list`], you will need to call this method to sort the
    /// users again.
    fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&User, &User) -> std::cmp::Ordering,
    {
        self.users_mut().sort_unstable_by(compare);
    }

    /// The user list will be emptied then completely recomputed.
    ///
    /// ```no_run
    /// use sysinfo::{Users, UsersExt};
    ///
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// ```
    fn refresh_list(&mut self);

    /// Returns the [`User`] matching the given `user_id`.
    ///
    /// **Important**: The user list must be filled before using this method, otherwise it will
    /// always return `None` (through the `refresh_*` methods).
    ///
    /// It is a shorthand for:
    ///
    /// ```ignore
    /// # use sysinfo::{UserExt, Users, UsersExt};
    /// let mut users = Users::new();
    /// users.refresh_list();
    /// users.users().find(|user| user.id() == user_id);
    /// ```
    ///
    /// Full example:
    ///
    /// ```no_run
    /// use sysinfo::{Pid, ProcessExt, System, Users, UsersExt};
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
    fn get_user_by_id(&self, user_id: &Uid) -> Option<&User> {
        self.users().iter().find(|user| user.id() == user_id)
    }
}

/// Getting information for a user group.
///
/// It is returned from [`UserExt::groups`].
///
/// ```no_run
/// use sysinfo::{GroupExt, UserExt, Users, UsersExt};
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
pub trait GroupExt: Debug + PartialEq + Eq + PartialOrd + Ord {
    /// Returns the ID of the group.
    ///
    /// ⚠️ This information is not set on Windows.
    ///
    /// ```no_run
    /// use sysinfo::{GroupExt, UserExt, Users, UsersExt};
    ///
    /// let mut users = Users::new();
    ///
    /// for user in users.users() {
    ///     for group in user.groups() {
    ///         println!("{:?}", group.id());
    ///     }
    /// }
    /// ```
    fn id(&self) -> &Gid;

    /// Returns the name of the group.
    ///
    /// ```no_run
    /// use sysinfo::{GroupExt, UserExt, Users, UsersExt};
    ///
    /// let mut users = Users::new();
    ///
    /// for user in users.users() {
    ///     for group in user.groups() {
    ///         println!("{}", group.name());
    ///     }
    /// }
    /// ```
    fn name(&self) -> &str;
}
