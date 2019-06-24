//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

/// Trait to have a common fallback for the `Pid` type.
pub trait AsU32 {
    /// Allows to convert `Pid` into `u32`.
    fn as_u32(&self) -> u32;
}

cfg_if!{
    if #[cfg(any(windows, target_os = "unknown"))] {
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
        doc_comment! {
concat!("Returns the value of the \"", stringify!($name), "\" refresh kind.

# Examples

```
use sysinfo::RefreshKind;

let r = RefreshKind::new();
assert_eq!(r.", stringify!($name), "(), false);

let r = r.with_", stringify!($name), "();
assert_eq!(r.", stringify!($name), "(), true);
```"),
            pub fn $name(&self) -> bool {
                self.$name
            }
        }

        doc_comment! {
concat!("Sets the value of the \"", stringify!($name), "\" refresh kind to `true`.

# Examples

```
use sysinfo::RefreshKind;

let r = RefreshKind::new();
assert_eq!(r.", stringify!($name), "(), false);

let r = r.with_", stringify!($name), "();
assert_eq!(r.", stringify!($name), "(), true);
```"),
            pub fn $with(mut self) -> RefreshKind {
                self.$name = true;
                self
            }
        }

        doc_comment! {
concat!("Sets the value of the \"", stringify!($name), "\" refresh kind to `false`.

# Examples

```
use sysinfo::RefreshKind;

let r = RefreshKind::everything();
assert_eq!(r.", stringify!($name), "(), true);

let r = r.without_", stringify!($name), "();
assert_eq!(r.", stringify!($name), "(), false);
```"),
            pub fn $without(mut self) -> RefreshKind {
                self.$name = false;
                self
            }
        }
    }
}

/// Used to determine what you want to refresh specifically on [`System`] type.
///
/// # Example
///
/// ```
/// use sysinfo::{RefreshKind, System, SystemExt};
///
/// // We want everything except disks.
/// let mut system = System::new_with_specifics(RefreshKind::everything().without_disk_list());
///
/// assert_eq!(system.get_disks().len(), 0);
/// assert!(system.get_process_list().len() > 0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefreshKind {
    system: bool,
    network: bool,
    processes: bool,
    disk_list: bool,
    disks: bool,
}

impl RefreshKind {
    /// Creates a new `RefreshKind` with every refresh set to `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// use sysinfo::RefreshKind;
    ///
    /// let r = RefreshKind::new();
    ///
    /// assert_eq!(r.system(), false);
    /// assert_eq!(r.network(), false);
    /// assert_eq!(r.processes(), false);
    /// assert_eq!(r.disk_list(), false);
    /// assert_eq!(r.disks(), false);
    /// ```
    pub fn new() -> RefreshKind {
        RefreshKind {
            system: false,
            network: false,
            processes: false,
            disks: false,
            disk_list: false,
        }
    }

    /// Creates a new `RefreshKind` with every refresh set to `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// use sysinfo::RefreshKind;
    ///
    /// let r = RefreshKind::everything();
    ///
    /// assert_eq!(r.system(), true);
    /// assert_eq!(r.network(), true);
    /// assert_eq!(r.processes(), true);
    /// assert_eq!(r.disk_list(), true);
    /// assert_eq!(r.disks(), true);
    /// ```
    pub fn everything() -> RefreshKind {
        RefreshKind {
            system: true,
            network: true,
            processes: true,
            disks: true,
            disk_list: true,
        }
    }

    impl_get_set!(system, with_system, without_system);
    impl_get_set!(network, with_network, without_network);
    impl_get_set!(processes, with_processes, without_processes);
    impl_get_set!(disks, with_disks, without_disks);
    impl_get_set!(disk_list, with_disk_list, without_disk_list);
}
