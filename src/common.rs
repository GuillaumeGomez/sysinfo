// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{NetworkData, Networks, NetworksExt, UserExt};

use std::convert::From;
use std::fmt;
use std::str::FromStr;

/// Trait to have a common conversions for the [`Pid`][crate::Pid] type.
///
/// ```
/// use sysinfo::{Pid, PidExt};
///
/// let p = Pid::from_u32(0);
/// let value: u32 = p.as_u32();
/// ```
pub trait PidExt<T>: Copy + From<T> + FromStr + fmt::Display {
    /// Allows to convert [`Pid`][crate::Pid] into [`u32`].
    ///
    /// ```
    /// use sysinfo::{Pid, PidExt};
    ///
    /// let p = Pid::from_u32(0);
    /// let value: u32 = p.as_u32();
    /// ```
    fn as_u32(self) -> u32;
    /// Allows to convert a [`u32`] into [`Pid`][crate::Pid].
    ///
    /// ```
    /// use sysinfo::{Pid, PidExt};
    ///
    /// let p = Pid::from_u32(0);
    /// ```
    fn from_u32(v: u32) -> Self;
}

macro_rules! pid_decl {
    ($typ:ty) => {
        #[doc = include_str!("../md_doc/pid.md")]
        #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(transparent)]
        pub struct Pid(pub(crate) $typ);

        impl From<$typ> for Pid {
            fn from(v: $typ) -> Self {
                Self(v)
            }
        }
        impl From<Pid> for $typ {
            fn from(v: Pid) -> Self {
                v.0
            }
        }
        impl PidExt<$typ> for Pid {
            fn as_u32(self) -> u32 {
                self.0 as u32
            }
            fn from_u32(v: u32) -> Self {
                Self(v as _)
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
/// use sysinfo::{ProcessExt, ProcessRefreshKind, System, SystemExt};
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
/// use sysinfo::{CpuExt, CpuRefreshKind, System, SystemExt};
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

/// Used to determine what you want to refresh specifically on the [`System`] type.
///
/// ⚠️ Just like all other refresh types, ruling out a refresh doesn't assure you that
/// the information won't be retrieved if the information is accessible without needing
/// extra computation.
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
    processes: Option<ProcessRefreshKind>,
    disks_list: bool,
    disks: bool,
    memory: bool,
    cpu: Option<CpuRefreshKind>,
    components: bool,
    components_list: bool,
    users_list: bool,
}

impl RefreshKind {
    /// Creates a new `RefreshKind` with every refresh set to `false`/`None`.
    ///
    /// ```
    /// use sysinfo::RefreshKind;
    ///
    /// let r = RefreshKind::new();
    ///
    /// assert_eq!(r.networks(), false);
    /// assert_eq!(r.networks_list(), false);
    /// assert_eq!(r.processes().is_some(), false);
    /// assert_eq!(r.disks_list(), false);
    /// assert_eq!(r.disks(), false);
    /// assert_eq!(r.memory(), false);
    /// assert_eq!(r.cpu().is_some(), false);
    /// assert_eq!(r.components(), false);
    /// assert_eq!(r.components_list(), false);
    /// assert_eq!(r.users_list(), false);
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
    /// assert_eq!(r.networks(), true);
    /// assert_eq!(r.networks_list(), true);
    /// assert_eq!(r.processes().is_some(), true);
    /// assert_eq!(r.disks_list(), true);
    /// assert_eq!(r.disks(), true);
    /// assert_eq!(r.memory(), true);
    /// assert_eq!(r.cpu().is_some(), true);
    /// assert_eq!(r.components(), true);
    /// assert_eq!(r.components_list(), true);
    /// assert_eq!(r.users_list(), true);
    /// ```
    pub fn everything() -> Self {
        Self {
            networks: true,
            networks_list: true,
            processes: Some(ProcessRefreshKind::everything()),
            disks: true,
            disks_list: true,
            memory: true,
            cpu: Some(CpuRefreshKind::everything()),
            components: true,
            components_list: true,
            users_list: true,
        }
    }

    impl_get_set!(
        RefreshKind,
        processes,
        with_processes,
        without_processes,
        ProcessRefreshKind
    );
    impl_get_set!(RefreshKind, networks, with_networks, without_networks);
    impl_get_set!(
        RefreshKind,
        networks_list,
        with_networks_list,
        without_networks_list
    );
    impl_get_set!(RefreshKind, disks, with_disks, without_disks);
    impl_get_set!(RefreshKind, disks_list, with_disks_list, without_disks_list);
    impl_get_set!(RefreshKind, memory, with_memory, without_memory);
    impl_get_set!(RefreshKind, cpu, with_cpu, without_cpu, CpuRefreshKind);
    impl_get_set!(RefreshKind, components, with_components, without_components);
    impl_get_set!(
        RefreshKind,
        components_list,
        with_components_list,
        without_components_list
    );
    impl_get_set!(RefreshKind, users_list, with_users_list, without_users_list);
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
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
///
/// If you want the list of the supported signals on the current system, use
/// [`SystemExt::SUPPORTED_SIGNALS`][crate::SystemExt::SUPPORTED_SIGNALS].
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
        #[repr(transparent)]
        #[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
        pub struct $name(pub(crate) $type);

        impl std::ops::Deref for $name {
            type Target = $type;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

macro_rules! uid {
    ($(#[$outer:meta])+ $type:ty) => {
        xid!($(#[$outer])+ Uid, $type);
    };
}

macro_rules! gid {
    ($(#[$outer:meta])+ $type:ty) => {
        xid!($(#[$outer])+ #[derive(Copy)] Gid, $type);
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
        uid!(
            /// A user id wrapping a platform specific type.
            libc::uid_t
        );
        gid!(
            /// A group id wrapping a platform specific type.
            libc::gid_t
        );
    } else if #[cfg(windows)] {
        uid!(
            /// A user id wrapping a platform specific type.
            Box<str>
        );
        gid!(
            /// A group id wrapping a platform specific type.
            u32
        );
    } else {
        uid!(
            /// A user id wrapping a platform specific type.
            u32
        );
        gid!(
            /// A group id wrapping a platform specific type.
            u32
        );

    }
}

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
    fn id(&self) -> &Uid {
        &self.uid
    }

    fn group_id(&self) -> Gid {
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
    /// ## Linux/FreeBSD
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
    /// ## Linux/FreeBSD
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
    /// ## Linux/FreeBSD
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
    /// ## Linux/FreeBSD
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
    /// ## FreeBSD
    ///
    /// Blocked on a lock.
    ///
    /// ## Other OS
    ///
    /// Not available.
    LockBlocked,
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
                use winapi::um::processthreadsapi::GetCurrentProcessId;

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
