// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::{CStr, CString, OsStr, OsString};
use std::io::{BufRead, BufReader};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::{Disk, DiskKind, DiskRefreshKind, DiskUsage, DisksInner, Error};

use super::{ffi, kstat::KstatReader};

const PSEUDO_FILE_SYSTEMS: &[&str] = &[
    "autofs", "bootfs", "ctfs", "dev", "devfs", "fd", "lofs", "mntfs", "objfs", "proc", "sharefs",
    "tmpfs",
];
const SOLID_STATE_PROPERTY: &CStr = c"device-solid-state";
const ROTATIONAL_PROPERTY: &CStr = c"device-rotational";
const REMOVABLE_PROPERTY: &CStr = c"removable-media";

#[derive(Debug)]
struct IoSource {
    module: CString,
    instance: i32,
    name: CString,
}

#[derive(Debug)]
struct DeviceInfo {
    io_source: IoSource,
    kind: DiskKind,
    is_removable: bool,
}

struct DevInfoNode {
    node: *mut ffi::DevInfoNode,
}

impl DevInfoNode {
    fn new(path: &CStr) -> Option<Self> {
        // SAFETY: `path` is NUL-terminated and the returned snapshot handle is checked before use.
        let node = unsafe { ffi::di_init(path.as_ptr(), ffi::DINFO_PROP) };
        (!node.is_null()).then_some(Self { node })
    }

    fn driver_name(&self) -> Option<&CStr> {
        // SAFETY: `node` remains valid for this snapshot's lifetime. libdevinfo owns the returned
        // NUL-terminated string.
        let driver = unsafe { ffi::di_driver_name(self.node) };
        (!driver.is_null()).then(|| unsafe { CStr::from_ptr(driver) })
    }

    fn instance(&self) -> i32 {
        // SAFETY: `node` remains valid for this snapshot's lifetime.
        unsafe { ffi::di_instance(self.node) }
    }

    fn integer_property(&self, name: &CStr) -> Option<i32> {
        let mut value = std::ptr::null_mut();
        // SAFETY: The node and property name are valid, and `value` points to writable storage.
        let count = unsafe {
            ffi::di_prop_lookup_ints(ffi::DDI_DEV_T_ANY, self.node, name.as_ptr(), &mut value)
        };
        if count < 1 || value.is_null() {
            None
        } else {
            // SAFETY: A positive count means libdevinfo returned at least one integer. The data is
            // owned by the snapshot and remains valid for its lifetime.
            Some(unsafe { *value })
        }
    }

    fn has_property(&self, name: &CStr) -> bool {
        let mut value = std::ptr::null_mut();
        // Boolean properties return zero entries when present and -1 when absent.
        // SAFETY: The node and property name are valid, and `value` points to writable storage.
        unsafe {
            ffi::di_prop_lookup_ints(ffi::DDI_DEV_T_ANY, self.node, name.as_ptr(), &mut value) >= 0
        }
    }
}

impl Drop for DevInfoNode {
    fn drop(&mut self) {
        // SAFETY: `node` was returned by `di_init` and is finalized exactly once here.
        unsafe { ffi::di_fini(self.node) };
    }
}

#[derive(Debug)]
pub(crate) struct DiskInner {
    type_: DiskKind,
    name: OsString,
    c_mount_point: Vec<libc::c_char>,
    device_id: Option<u64>,
    mount_point: PathBuf,
    total_space: u64,
    available_space: u64,
    pub(crate) file_system: OsString,
    is_removable: bool,
    is_read_only: bool,
    io_source: Option<IoSource>,
    read_bytes: u64,
    old_read_bytes: u64,
    written_bytes: u64,
    old_written_bytes: u64,
    updated: bool,
}

#[cfg(test)]
impl Default for DiskInner {
    fn default() -> Self {
        Self {
            type_: DiskKind::Unknown(-1),
            name: OsString::new(),
            c_mount_point: Vec::new(),
            device_id: None,
            mount_point: PathBuf::new(),
            total_space: 0,
            available_space: 0,
            file_system: OsString::new(),
            is_removable: false,
            is_read_only: false,
            io_source: None,
            read_bytes: 0,
            old_read_bytes: 0,
            written_bytes: 0,
            old_written_bytes: 0,
            updated: false,
        }
    }
}

impl DiskInner {
    pub(crate) fn kind(&self) -> DiskKind {
        self.type_
    }

    pub(crate) fn name(&self) -> &OsStr {
        &self.name
    }

    pub(crate) fn id(&self) -> Option<u64> {
        self.device_id
    }

    pub(crate) fn file_system(&self) -> &OsStr {
        &self.file_system
    }

    pub(crate) fn mount_point(&self) -> &Path {
        &self.mount_point
    }

    pub(crate) fn total_space(&self) -> u64 {
        self.total_space
    }

    pub(crate) fn available_space(&self) -> u64 {
        self.available_space
    }

    pub(crate) fn is_removable(&self) -> bool {
        self.is_removable
    }

    pub(crate) fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    pub(crate) fn refresh_specifics(&mut self, refresh_kind: DiskRefreshKind) -> bool {
        // ZFS kstats describe an entire pool, while each `Disk` represents a mounted dataset.
        // Associating pool counters with every dataset would duplicate and mislabel the I/O.
        let is_zfs = self.file_system.as_bytes() == b"zfs";
        let needs_io_source = refresh_kind.io_usage() && self.io_source.is_none() && !is_zfs;
        let needs_kind = refresh_kind.kind() && self.type_ == DiskKind::Unknown(-1) && !is_zfs;
        if (needs_io_source || needs_kind)
            && let Some(info) = device_info(&self.name)
        {
            if needs_io_source {
                self.io_source = Some(info.io_source);
            }
            if needs_kind {
                self.type_ = info.kind;
                self.is_removable = info.is_removable;
            }
        }
        if refresh_kind.storage() {
            self.refresh_storage();
        }
        if refresh_kind.io_usage() {
            self.refresh_io_usage();
        }
        true
    }

    pub(crate) fn usage(&self) -> DiskUsage {
        DiskUsage {
            read_bytes: self.read_bytes.saturating_sub(self.old_read_bytes),
            total_read_bytes: self.read_bytes,
            written_bytes: self.written_bytes.saturating_sub(self.old_written_bytes),
            total_written_bytes: self.written_bytes,
        }
    }

    fn refresh_storage(&mut self) {
        let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        if unsafe { libc::statvfs(self.c_mount_point.as_ptr(), stat.as_mut_ptr()) } != 0 {
            return;
        }
        let stat = unsafe { stat.assume_init() };
        let block_size = stat.f_frsize;
        self.total_space = stat.f_blocks.saturating_mul(block_size);
        self.available_space = stat.f_bavail.saturating_mul(block_size);
        self.is_read_only = stat.f_flag & libc::ST_RDONLY != 0;
    }

    fn refresh_io_usage(&mut self) {
        self.old_read_bytes = self.read_bytes;
        self.old_written_bytes = self.written_bytes;
        let Some(source) = self.io_source.as_ref() else {
            return;
        };
        let Some(io) = KstatReader::new().and_then(|kstat| {
            kstat.read_io(Some(&source.module), source.instance, Some(&source.name))
        }) else {
            return;
        };
        self.read_bytes = io.nread;
        self.written_bytes = io.nwritten;
    }
}

impl DisksInner {
    pub(crate) fn new() -> Result<Self, Error> {
        Ok(Self {
            disks: Vec::with_capacity(8),
        })
    }

    pub(crate) fn refresh_specifics(
        &mut self,
        remove_not_listed_disks: bool,
        refresh_kind: DiskRefreshKind,
    ) {
        for disk in &mut self.disks {
            disk.inner.updated = false;
        }

        let Ok(mnttab) = std::fs::File::open("/etc/mnttab") else {
            return;
        };
        for line in BufReader::new(mnttab).lines().map_while(Result::ok) {
            let mut fields = line.split_whitespace();
            let (Some(name), Some(mount_point), Some(file_system), Some(options)) =
                (fields.next(), fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            if PSEUDO_FILE_SYSTEMS.contains(&file_system) {
                continue;
            }

            let name = unescape_mnttab(name);
            let mount_point = PathBuf::from(OsString::from_vec(unescape_mnttab(mount_point)));
            let file_system = OsString::from(file_system);
            if let Some(disk) = self
                .disks
                .iter_mut()
                .find(|disk| disk.inner.mount_point == mount_point)
            {
                let device_changed = disk.inner.name.as_bytes() != name
                    || disk.inner.file_system.as_bytes() != file_system.as_bytes();
                disk.inner.updated = true;
                disk.inner.name = OsString::from_vec(name);
                disk.inner.file_system = file_system;
                if device_changed {
                    disk.inner.io_source = None;
                    disk.inner.type_ = DiskKind::Unknown(-1);
                    disk.inner.is_removable = false;
                }
                disk.inner.is_read_only = options.split(',').any(|option| option == "ro");
                disk.inner.refresh_specifics(refresh_kind);
                continue;
            }

            let mut c_mount_point = mount_point.as_os_str().as_encoded_bytes().to_vec();
            if c_mount_point.contains(&0) {
                continue;
            }
            c_mount_point.push(0);
            let mut inner = DiskInner {
                type_: DiskKind::Unknown(-1),
                name: OsString::from_vec(name),
                c_mount_point: c_mount_point
                    .into_iter()
                    .map(|byte| byte as libc::c_char)
                    .collect(),
                device_id: std::fs::metadata(&mount_point).ok().map(|meta| meta.dev()),
                mount_point,
                total_space: 0,
                available_space: 0,
                file_system,
                is_removable: false,
                is_read_only: options.split(',').any(|option| option == "ro"),
                io_source: None,
                read_bytes: 0,
                old_read_bytes: 0,
                written_bytes: 0,
                old_written_bytes: 0,
                updated: true,
            };
            inner.refresh_specifics(refresh_kind);
            self.disks.push(Disk { inner });
        }

        if remove_not_listed_disks {
            self.disks.retain(|disk| disk.inner.updated);
        }
    }

    pub(crate) fn list(&self) -> &[Disk] {
        &self.disks
    }

    pub(crate) fn list_mut(&mut self) -> &mut [Disk] {
        &mut self.disks
    }
}

fn device_info(name: &OsStr) -> Option<DeviceInfo> {
    let canonical = std::fs::canonicalize(Path::new(name)).ok()?;
    let devfs_path = devfs_path(canonical.as_os_str().as_bytes())?;
    let node = DevInfoNode::new(&devfs_path)?;
    let driver = node.driver_name()?;
    let instance = node.instance();
    if instance < 0 {
        return None;
    }

    let mut kstat_name = driver.to_bytes().to_vec();
    kstat_name.extend_from_slice(instance.to_string().as_bytes());
    let kind = match (
        node.integer_property(SOLID_STATE_PROPERTY),
        node.integer_property(ROTATIONAL_PROPERTY),
    ) {
        (Some(value), _) if value != 0 => DiskKind::SSD,
        (_, Some(value)) if value != 0 => DiskKind::HDD,
        _ => DiskKind::Unknown(-1),
    };

    Some(DeviceInfo {
        io_source: IoSource {
            module: driver.to_owned(),
            instance,
            name: CString::new(kstat_name).ok()?,
        },
        kind,
        is_removable: node.has_property(REMOVABLE_PROPERTY),
    })
}

fn devfs_path(path: &[u8]) -> Option<CString> {
    let path = path.strip_prefix(b"/devices")?;
    let minor = path.iter().rposition(|byte| *byte == b':')?;
    CString::new(&path[..minor]).ok()
}

fn unescape_mnttab(value: &str) -> Vec<u8> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\'
            && index + 3 < bytes.len()
            && bytes[index + 1..=index + 3].iter().all(u8::is_ascii_digit)
        {
            let value =
                (bytes[index + 1] - b'0') * 64 + (bytes[index + 2] - b'0') * 8 + bytes[index + 3]
                    - b'0';
            output.push(value);
            index += 4;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{DiskInner, PSEUDO_FILE_SYSTEMS, devfs_path};
    use crate::{DiskRefreshKind, DiskUsage};

    #[test]
    fn tmpfs_is_not_a_disk() {
        assert!(PSEUDO_FILE_SYSTEMS.contains(&"tmpfs"));
    }

    #[test]
    fn zfs_dataset_does_not_report_pool_io() {
        let mut disk = DiskInner {
            name: "rpool/example".into(),
            file_system: "zfs".into(),
            ..DiskInner::default()
        };

        disk.refresh_specifics(DiskRefreshKind::nothing().with_io_usage());

        assert!(disk.io_source.is_none());
        assert_eq!(disk.usage(), DiskUsage::default());
    }

    #[test]
    fn extracts_devfs_node_path() {
        assert_eq!(
            devfs_path(b"/devices/pci@0,0/pci1af4,2@3/blkdev@0,0:a")
                .as_deref()
                .map(|path| path.to_bytes()),
            Some(b"/pci@0,0/pci1af4,2@3/blkdev@0,0".as_slice()),
        );
        assert!(devfs_path(b"/dev/dsk/c1t0d0s0").is_none());
    }
}
