// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::OsStr;
use std::fmt;
use std::path::Path;

use crate::common::impl_get_set::impl_get_set;
use crate::DiskUsage;

/// Struct containing a disk information.
///
/// ```no_run
/// use sysinfo::Disks;
///
/// let disks = Disks::new_with_refreshed_list();
/// for disk in disks.list() {
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
    /// let disks = Disks::new_with_refreshed_list();
    /// for disk in disks.list() {
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
    /// let disks = Disks::new_with_refreshed_list();
    /// for disk in disks.list() {
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
    /// let disks = Disks::new_with_refreshed_list();
    /// for disk in disks.list() {
    ///     println!("[{:?}] {:?}", disk.name(), disk.file_system());
    /// }
    /// ```
    pub fn file_system(&self) -> &OsStr {
        self.inner.file_system()
    }

    /// Returns the mount point of the disk (`/` for example).
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let disks = Disks::new_with_refreshed_list();
    /// for disk in disks.list() {
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
    /// let disks = Disks::new_with_refreshed_list();
    /// for disk in disks.list() {
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
    /// let disks = Disks::new_with_refreshed_list();
    /// for disk in disks.list() {
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
    /// let disks = Disks::new_with_refreshed_list();
    /// for disk in disks.list() {
    ///     println!("[{:?}] {}", disk.name(), disk.is_removable());
    /// }
    /// ```
    pub fn is_removable(&self) -> bool {
        self.inner.is_removable()
    }

    /// Returns `true` if the disk is read-only.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let disks = Disks::new_with_refreshed_list();
    /// for disk in disks.list() {
    ///     println!("[{:?}] is read-only: {}", disk.name(), disk.is_read_only());
    /// }
    /// ```
    pub fn is_read_only(&self) -> bool {
        self.inner.is_read_only()
    }

    /// Updates the disk' information with everything loaded.
    ///
    /// Equivalent to [`Disk::refresh_specifics`]`(`[`DiskRefreshKind::everything`]`())`.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new_with_refreshed_list();
    /// for disk in disks.list_mut() {
    ///     disk.refresh();
    /// }
    /// ```
    pub fn refresh(&mut self) -> bool {
        self.refresh_specifics(DiskRefreshKind::everything())
    }

    /// Updates the disk's information corresponding to the given [`DiskRefreshKind`].
    ///
    /// ```no_run
    /// use sysinfo::{Disks, DiskRefreshKind};
    ///
    /// let mut disks = Disks::new_with_refreshed_list();
    /// for disk in disks.list_mut() {
    ///     disk.refresh_specifics(DiskRefreshKind::new());
    /// }
    /// ```
    pub fn refresh_specifics(&mut self, refreshes: DiskRefreshKind) -> bool {
        self.inner.refresh_specifics(refreshes)
    }

    /// Returns number of bytes read and written by the disk
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let disks = Disks::new_with_refreshed_list();
    /// for disk in disks.list() {
    ///     println!("[{:?}] disk usage: {:?}", disk.name(), disk.usage());
    /// }
    /// ```
    pub fn usage(&self) -> DiskUsage {
        self.inner.usage()
    }
}

/// Disks interface.
///
/// ```no_run
/// use sysinfo::Disks;
///
/// let disks = Disks::new_with_refreshed_list();
/// for disk in disks.list() {
///     println!("{disk:?}");
/// }
/// ```
///
/// ⚠️ Note that tmpfs mounts are excluded by default under Linux.
/// To display tmpfs mount points, the `linux-tmpfs` feature must be enabled.
///
/// ⚠️ Note that network devices are excluded by default under Linux.
/// To display mount points using the CIFS and NFS protocols, the `linux-netdevs`
/// feature must be enabled. Note, however, that sysinfo may hang under certain
/// circumstances. For example, if a CIFS or NFS share has been mounted with
/// the _hard_ option, but the connection has an error, such as the share server has stopped.
pub struct Disks {
    inner: crate::DisksInner,
}

impl Default for Disks {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Disks> for Vec<Disk> {
    fn from(disks: Disks) -> Vec<Disk> {
        disks.inner.into_vec()
    }
}

impl From<Vec<Disk>> for Disks {
    fn from(disks: Vec<Disk>) -> Self {
        Self {
            inner: crate::DisksInner::from_vec(disks),
        }
    }
}

impl<'a> IntoIterator for &'a Disks {
    type Item = &'a Disk;
    type IntoIter = std::slice::Iter<'a, Disk>;

    fn into_iter(self) -> Self::IntoIter {
        self.list().iter()
    }
}

impl<'a> IntoIterator for &'a mut Disks {
    type Item = &'a mut Disk;
    type IntoIter = std::slice::IterMut<'a, Disk>;

    fn into_iter(self) -> Self::IntoIter {
        self.list_mut().iter_mut()
    }
}

impl Disks {
    /// Creates a new empty [`Disks`][crate::Disks] type.
    ///
    /// If you want it to be filled directly, take a look at [`Disks::new_with_refreshed_list`].
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// for disk in disks.list() {
    ///     println!("{disk:?}");
    /// }
    /// ```
    pub fn new() -> Self {
        Self {
            inner: crate::DisksInner::new(),
        }
    }

    /// Creates a new [`Disks`][crate::Disks] type with the disk list loaded.
    ///
    /// Equivalent to [`Disks::new_with_refreshed_list_specifics`]`(`[`DiskRefreshKind::everything`]`())`.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new_with_refreshed_list();
    /// for disk in disks.list() {
    ///     println!("{disk:?}");
    /// }
    /// ```
    pub fn new_with_refreshed_list() -> Self {
        Self::new_with_refreshed_list_specifics(DiskRefreshKind::everything())
    }

    /// Creates a new [`Disks`][crate::Disks] type with the disk list loaded
    /// and refreshed according to the given [`DiskRefreshKind`]. It is a combination of
    /// [`Disks::new`] and [`Disks::refresh_list_specifics`].
    ///
    /// ```no_run
    /// use sysinfo::{Disks, DiskRefreshKind};
    ///
    /// let mut disks = Disks::new_with_refreshed_list_specifics(DiskRefreshKind::new());
    /// for disk in disks.list() {
    ///     println!("{disk:?}");
    /// }
    /// ```
    pub fn new_with_refreshed_list_specifics(refreshes: DiskRefreshKind) -> Self {
        let mut disks = Self::new();
        disks.refresh_list_specifics(refreshes);
        disks
    }

    /// Returns the disks list.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let disks = Disks::new_with_refreshed_list();
    /// for disk in disks.list() {
    ///     println!("{disk:?}");
    /// }
    /// ```
    pub fn list(&self) -> &[Disk] {
        self.inner.list()
    }

    /// Returns the disks list.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new_with_refreshed_list();
    /// for disk in disks.list_mut() {
    ///     disk.refresh();
    ///     println!("{disk:?}");
    /// }
    /// ```
    pub fn list_mut(&mut self) -> &mut [Disk] {
        self.inner.list_mut()
    }

    /// Refreshes the listed disks' information.
    ///
    /// Equivalent to [`Disks::refresh_specifics`]`(`[`DiskRefreshKind::everything`]`())`.
    pub fn refresh(&mut self) {
        self.refresh_specifics(DiskRefreshKind::everything());
    }

    /// Refreshes the listed disks' information according to the given [`DiskRefreshKind`].
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
    /// let mut disks = Disks::new_with_refreshed_list();
    /// // We wait some time...?
    /// disks.refresh();
    /// ```
    pub fn refresh_specifics(&mut self, refreshes: DiskRefreshKind) {
        self.inner.refresh_specifics(refreshes);
    }

    /// The disk list will be emptied then completely recomputed.
    ///
    /// Equivalent to [`Disks::refresh_list_specifics`]`(`[`DiskRefreshKind::everything`]`())`.
    ///
    /// ```no_run
    /// use sysinfo::Disks;
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list();
    /// ```
    pub fn refresh_list(&mut self) {
        self.refresh_list_specifics(DiskRefreshKind::everything());
    }

    /// The disk list will be emptied then completely recomputed according to the given
    /// [`DiskRefreshKind`].
    ///
    /// ## Linux
    ///
    /// ⚠️ On Linux, the [NFS](https://en.wikipedia.org/wiki/Network_File_System) file
    /// systems are ignored and the information of a mounted NFS **cannot** be obtained
    /// via [`Disks::refresh_list_specifics`]. This is due to the fact that I/O function
    /// `statvfs` used by [`Disks::refresh_list_specifics`] is blocking and
    /// [may hang](https://github.com/GuillaumeGomez/sysinfo/pull/876) in some cases,
    /// requiring to call `systemctl stop` to terminate the NFS service from the remote
    /// server in some cases.
    ///
    /// ```no_run
    /// use sysinfo::{Disks, DiskRefreshKind};
    ///
    /// let mut disks = Disks::new();
    /// disks.refresh_list_specifics(DiskRefreshKind::new());
    /// ```
    pub fn refresh_list_specifics(&mut self, refreshes: DiskRefreshKind) {
        self.inner.refresh_list_specifics(refreshes);
    }
}

impl std::ops::Deref for Disks {
    type Target = [Disk];

    fn deref(&self) -> &Self::Target {
        self.list()
    }
}

impl std::ops::DerefMut for Disks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.list_mut()
    }
}

/// Enum containing the different supported kinds of disks.
///
/// This type is returned by [`Disk::kind`](`crate::Disk::kind`).
///
/// ```no_run
/// use sysinfo::Disks;
///
/// let disks = Disks::new_with_refreshed_list();
/// for disk in disks.list() {
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

impl fmt::Display for DiskKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            DiskKind::HDD => "HDD",
            DiskKind::SSD => "SSD",
            _ => "Unknown",
        })
    }
}

/// Used to determine what you want to refresh specifically on the [`Disk`] type.
///
/// ⚠️ Just like all other refresh types, ruling out a refresh doesn't assure you that
/// the information won't be retrieved if the information is accessible without needing
/// extra computation.
///
/// ```no_run
/// use sysinfo::{Disks, DiskRefreshKind};
///
/// let mut disks = Disks::new_with_refreshed_list_specifics(DiskRefreshKind::new());
///
/// for disk in disks.list() {
///     assert_eq!(disk.total_space(), 0);
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct DiskRefreshKind {
    kind: bool,
    total_space: bool,
    available_space: bool,
    is_removable: bool,
    is_read_only: bool,
    usage: bool,
}

impl Default for DiskRefreshKind {
    fn default() -> Self {
        Self {
            kind: true,
            total_space: false,
            available_space: false,
            is_removable: false,
            is_read_only: false,
            usage: false,
        }
    }
}

impl DiskRefreshKind {
    /// Creates a new `DiskRefreshKind` with every refresh *except kind* set to false.
    ///
    /// ```
    /// use sysinfo::DiskRefreshKind;
    ///
    /// let r = DiskRefreshKind::new();
    ///
    /// assert_eq!(r.kind(), true);
    /// assert_eq!(r.total_space(), false);
    /// assert_eq!(r.available_space(), false);
    /// assert_eq!(r.is_removable(), false);
    /// assert_eq!(r.is_read_only(), false);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new `DiskRefreshKind` with every refresh set to true.
    ///
    /// ```
    /// use sysinfo::DiskRefreshKind;
    ///
    /// let r = DiskRefreshKind::everything();
    ///
    /// assert_eq!(r.kind(), true);
    /// assert_eq!(r.total_space(), true);
    /// assert_eq!(r.available_space(), true);
    /// assert_eq!(r.is_removable(), true);
    /// assert_eq!(r.is_read_only(), true);
    /// ```
    pub fn everything() -> Self {
        Self {
            kind: true,
            total_space: true,
            available_space: true,
            is_removable: true,
            is_read_only: true,
            usage: true,
        }
    }

    impl_get_set!(DiskRefreshKind, kind, with_kind, without_kind);
    impl_get_set!(
        DiskRefreshKind,
        total_space,
        with_total_space,
        without_total_space
    );
    impl_get_set!(
        DiskRefreshKind,
        available_space,
        with_available_space,
        without_available_space
    );
    impl_get_set!(
        DiskRefreshKind,
        is_removable,
        with_is_removable,
        without_is_removable
    );
    impl_get_set!(
        DiskRefreshKind,
        is_read_only,
        with_is_read_only,
        without_is_read_only
    );
    impl_get_set!(DiskRefreshKind, usage, with_usage, without_usage);
}
