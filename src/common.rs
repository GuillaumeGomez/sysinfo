//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use NetworkData;
use Networks;
use NetworksExt;

/// Trait to have a common fallback for the [`Pid`][crate::Pid] type.
pub trait AsU32 {
    /// Allows to convert [`Pid`][crate::Pid] into [`u32`].
    fn as_u32(&self) -> u32;
}

cfg_if! {
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
        doc_comment! {
        concat!("Returns the value of the \"", stringify!($name), "\" refresh kind.

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
/// assert_eq!(system.get_disks().len(), 0);
/// assert!(system.get_processes().len() > 0);
/// ```
///
/// [`System`]: crate::System
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
        RefreshKind {
            networks: false,
            networks_list: false,
            processes: false,
            disks: false,
            disks_list: false,
            memory: false,
            cpu: false,
            components: false,
            components_list: false,
            users_list: false,
        }
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
/// let networks_iter = system.get_networks().iter();
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
/// This type is returned by [`Disk::get_type`][crate::Disk#method.get_type].
///
/// ```no_run
/// use sysinfo::{System, SystemExt, DiskExt};
///
/// let system = System::new_all();
/// for disk in system.get_disks() {
///     println!("{:?}: {:?}", disk.get_name(), disk.get_type());
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

impl From<isize> for DiskType {
    fn from(t: isize) -> DiskType {
        match t {
            0 => DiskType::HDD,
            1 => DiskType::SSD,
            id => DiskType::Unknown(id),
        }
    }
}
