// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{NetworkData, Networks, NetworksExt, UserExt};

/// Trait to have a common fallback for the [`Pid`][crate::Pid] type.
pub trait AsU32 {
    /// Allows to convert [`Pid`][crate::Pid] into [`u32`].
    fn as_u32(&self) -> u32;
}

cfg_if::cfg_if! {
    if #[cfg(any(windows, target_os = "unknown", target_arch = "wasm32"))] {
        /// Process id.
        pub type Pid = usize;

        impl AsU32 for Pid {
            fn as_u32(&self) -> u32 {
                *self as u32
            }
        }
    } else {
        use libc::pid_t;

        /// Process id.
        pub type Pid = pid_t;

        impl AsU32 for Pid {
            fn as_u32(&self) -> u32 {
                *self as u32
            }
        }
    }
}

macro_rules! impl_get_set {
    ($name:ident, $with:ident, $without:ident) => {
        #[doc = concat!("Returns the value of the \"", stringify!($name), "\" refresh kind.

```
use sysinfo::RefreshKind;

let r = RefreshKind::new();
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
use sysinfo::RefreshKind;

let r = RefreshKind::new();
assert_eq!(r.", stringify!($name), "(), false);

let r = r.with_", stringify!($name), "();
assert_eq!(r.", stringify!($name), "(), true);
```")]
        pub fn $with(mut self) -> RefreshKind {
            self.$name = true;
            self
        }

        #[doc = concat!("Sets the value of the \"", stringify!($name), "\" refresh kind to `false`.

```
use sysinfo::RefreshKind;

let r = RefreshKind::everything();
assert_eq!(r.", stringify!($name), "(), true);

let r = r.without_", stringify!($name), "();
assert_eq!(r.", stringify!($name), "(), false);
```")]
        pub fn $without(mut self) -> RefreshKind {
            self.$name = false;
            self
        }
    };
}

/// Used to determine what you want to refresh specifically on [`System`] type.
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
/// ```
///
/// [`System`]: crate::System
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RefreshKind {
    networks: bool,
    networks_list: bool,
    processes: bool,
    disks_list: bool,
    disks: bool,
    memory: bool,
    cpu: bool,
    components: bool,
    components_list: bool,
    users_list: bool,
}

impl RefreshKind {
    /// Creates a new `RefreshKind` with every refresh set to `false`.
    ///
    /// ```
    /// use sysinfo::RefreshKind;
    ///
    /// let r = RefreshKind::new();
    ///
    /// assert_eq!(r.networks(), false);
    /// assert_eq!(r.networks_list(), false);
    /// assert_eq!(r.processes(), false);
    /// assert_eq!(r.disks_list(), false);
    /// assert_eq!(r.disks(), false);
    /// assert_eq!(r.memory(), false);
    /// assert_eq!(r.cpu(), false);
    /// assert_eq!(r.components(), false);
    /// assert_eq!(r.components_list(), false);
    /// assert_eq!(r.users_list(), false);
    /// ```
    pub fn new() -> RefreshKind {
        RefreshKind::default()
    }

    /// Creates a new `RefreshKind` with every refresh set to `true`.
    ///
    /// ```
    /// use sysinfo::RefreshKind;
    ///
    /// let r = RefreshKind::everything();
    ///
    /// assert_eq!(r.networks(), true);
    /// assert_eq!(r.networks_list(), true);
    /// assert_eq!(r.processes(), true);
    /// assert_eq!(r.disks_list(), true);
    /// assert_eq!(r.disks(), true);
    /// assert_eq!(r.memory(), true);
    /// assert_eq!(r.cpu(), true);
    /// assert_eq!(r.components(), true);
    /// assert_eq!(r.components_list(), true);
    /// assert_eq!(r.users_list(), true);
    /// ```
    pub fn everything() -> RefreshKind {
        RefreshKind {
            networks: true,
            networks_list: true,
            processes: true,
            disks: true,
            disks_list: true,
            memory: true,
            cpu: true,
            components: true,
            components_list: true,
            users_list: true,
        }
    }

    impl_get_set!(networks, with_networks, without_networks);
    impl_get_set!(networks_list, with_networks_list, without_networks_list);
    impl_get_set!(processes, with_processes, without_processes);
    impl_get_set!(disks, with_disks, without_disks);
    impl_get_set!(disks_list, with_disks_list, without_disks_list);
    impl_get_set!(memory, with_memory, without_memory);
    impl_get_set!(cpu, with_cpu, without_cpu);
    impl_get_set!(components, with_components, without_components);
    impl_get_set!(
        components_list,
        with_components_list,
        without_components_list
    );
    impl_get_set!(users_list, with_users_list, without_users_list);
}

/// Iterator over network interfaces.
///
/// It is returned by [`Networks::iter`][crate::Networks#method.iter].
///
/// ```no_run
/// use sysinfo::{System, SystemExt, NetworksExt};
///
/// let system = System::new_all();
/// let networks_iter = system.networks().iter();
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

impl<'a> IntoIterator for &'a Networks {
    type Item = (&'a String, &'a NetworkData);
    type IntoIter = NetworksIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Enum containing the different supported disks types.
///
/// This type is returned by [`Disk::get_type`][crate::Disk#method.type].
///
/// ```no_run
/// use sysinfo::{System, SystemExt, DiskExt};
///
/// let system = System::new_all();
/// for disk in system.disks() {
///     println!("{:?}: {:?}", disk.name(), disk.type_());
/// }
/// ```
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DiskType {
    /// HDD type.
    HDD,
    /// SSD type.
    SSD,
    /// Unknown type.
    Unknown(isize),
}

/// An enum representing signals on UNIX-like systems.
///
/// On non-unix systems, this enum is mostly useless and is only there to keep coherency between
/// the different OSes.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
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

/// A struct representing system load average value.
///
/// It is returned by [`SystemExt::load_average`][crate::SystemExt::load_average].
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
    ($(#[$outer:meta])+ $name:ident, $type:ty) => {
        $(#[$outer])+
        #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
        pub struct $name(pub(crate) $type);

        impl std::ops::Deref for $name {
            type Target = $type;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

#[cfg(not(target_os = "windows"))]
xid!(
    /// A user id wrapping a platform specific type
    Uid,
    libc::uid_t
);

#[cfg(target_os = "windows")]
xid!(
    /// A user id wrapping a platform specific type
    Uid,
    u32
);

#[cfg(not(target_os = "windows"))]
xid!(
    /// A group id wrapping a platform specific type
    Gid,
    libc::gid_t
);

#[cfg(target_os = "windows")]
xid!(
    /// A group id wrapping a platform specific type
    Gid,
    u32
);

/// Type containing user information.
///
/// It is returned by [`SystemExt::users`][crate::SystemExt::users].
///
/// ```no_run
/// use sysinfo::{System, SystemExt};
///
/// let s = System::new_all();
/// println!("users: {:?}", s.users());
/// ```
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct User {
    pub(crate) uid: Uid,
    pub(crate) gid: Gid,
    pub(crate) name: String,
    pub(crate) groups: Vec<String>,
}

impl UserExt for User {
    fn uid(&self) -> Uid {
        self.uid
    }

    fn gid(&self) -> Gid {
        self.gid
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn groups(&self) -> &[String] {
        &self.groups
    }
}

/// Type containing read and written bytes.
///
/// It is returned by [`ProcessExt::disk_usage`][crate::ProcessExt::disk_usage].
///
/// ```no_run
/// use sysinfo::{ProcessExt, System, SystemExt};
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
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProcessStatus {
    /// ## Linux
    ///
    /// Waiting in uninterruptible disk sleep.
    ///
    /// ## macOs
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
    /// ## macOS
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
    /// ## macOS
    ///
    /// Process debugging or suspension.
    ///
    /// ## Other OS
    ///
    /// Not available.
    Stop,
    /// ## Linux
    ///
    /// Zombie process. Terminated but not reaped by its parent.
    ///
    /// ## macOS
    ///
    /// Awaiting collection by parent.
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
    /// ## Other OS
    ///
    /// Not available.
    Parked,
    /// Unknown.
    Unknown(u32),
}

#[cfg(test)]
mod tests {
    use super::ProcessStatus;

    // This test only exists to ensure that the `Display` trait is implemented on the
    // `ProcessStatus` enum on all targets.
    #[test]
    fn check_display_impl_process_status() {
        println!("{} {:?}", ProcessStatus::Parked, ProcessStatus::Idle);
    }
}
