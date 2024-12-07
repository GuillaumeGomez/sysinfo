// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    sys::{
        ffi,
        utils::{self, CFReleaser},
    },
    DiskUsage,
};
use crate::{Disk, DiskKind, DiskRefreshKind};

use core_foundation_sys::array::CFArrayCreate;
use core_foundation_sys::base::kCFAllocatorDefault;
use core_foundation_sys::dictionary::{CFDictionaryGetValueIfPresent, CFDictionaryRef};
use core_foundation_sys::number::{kCFBooleanTrue, CFBooleanRef, CFNumberGetValue};
use core_foundation_sys::string::{self as cfs, CFStringRef};

use libc::c_void;

use std::ffi::{CStr, OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr;

pub(crate) struct DiskInner {
    pub(crate) type_: DiskKind,
    pub(crate) name: OsString,
    #[cfg(target_os = "macos")]
    bsd_name: Option<Vec<u8>>,
    pub(crate) file_system: OsString,
    pub(crate) mount_point: PathBuf,
    volume_url: RetainedCFURL,
    pub(crate) total_space: u64,
    pub(crate) available_space: u64,
    pub(crate) is_removable: bool,
    pub(crate) is_read_only: bool,
    pub(crate) old_written_bytes: u64,
    pub(crate) old_read_bytes: u64,
    pub(crate) written_bytes: u64,
    pub(crate) read_bytes: u64,
    updated: bool,
}

impl DiskInner {
    pub(crate) fn kind(&self) -> DiskKind {
        self.type_
    }

    pub(crate) fn name(&self) -> &OsStr {
        &self.name
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
        self.refresh_kind(refresh_kind);
        self.refresh_io(refresh_kind);

        if refresh_kind.storage() {
            unsafe {
                if let Some(requested_properties) = build_requested_properties(&[
                    ffi::kCFURLVolumeTotalCapacityKey,
                    ffi::kCFURLVolumeAvailableCapacityKey,
                    ffi::kCFURLVolumeAvailableCapacityForImportantUsageKey,
                ]) {
                    match get_disk_properties(&self.volume_url, &requested_properties) {
                        Some(disk_props) => {
                            match get_int_value(
                                disk_props.inner(),
                                DictKey::Extern(ffi::kCFURLVolumeTotalCapacityKey),
                            ) {
                                Some(total_space) => self.total_space = total_space,
                                None => {
                                    sysinfo_debug!("Failed to get disk total space");
                                }
                            }
                            match get_available_volume_space(&disk_props) {
                                Some(available_space) => self.available_space = available_space,
                                None => {
                                    sysinfo_debug!("Failed to get disk available space");
                                }
                            }
                        }
                        None => {
                            sysinfo_debug!("Failed to get disk properties");
                        }
                    }
                } else {
                    sysinfo_debug!("failed to create volume key list, skipping refresh");
                }
            }
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

    fn refresh_kind(&mut self, refresh_kind: DiskRefreshKind) {
        if refresh_kind.kind() && self.type_ == DiskKind::Unknown(-1) {
            #[cfg(target_os = "macos")]
            {
                match self
                    .bsd_name
                    .as_ref()
                    .and_then(|name| crate::sys::inner::disk::get_disk_type(name))
                {
                    Some(type_) => self.type_ = type_,
                    None => {
                        sysinfo_debug!("Failed to retrieve `DiskKind`");
                    }
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                self.type_ = DiskKind::SSD;
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn refresh_io(&mut self, refresh_kind: DiskRefreshKind) {
        if refresh_kind.io_usage() {
            match self
                .bsd_name
                .as_ref()
                .and_then(|name| crate::sys::inner::disk::get_disk_io(name))
            {
                Some((read_bytes, written_bytes)) => {
                    self.old_read_bytes = self.read_bytes;
                    self.old_written_bytes = self.written_bytes;
                    self.read_bytes = read_bytes;
                    self.written_bytes = written_bytes;
                }
                None => {
                    sysinfo_debug!("Failed to update disk i/o stats");
                }
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn refresh_io(&mut self, _refresh_kind: DiskRefreshKind) {}
}

impl crate::DisksInner {
    pub(crate) fn new() -> Self {
        Self {
            disks: Vec::with_capacity(2),
        }
    }

    pub(crate) fn refresh_specifics(
        &mut self,
        remove_not_listed_disks: bool,
        refresh_kind: DiskRefreshKind,
    ) {
        unsafe {
            // SAFETY: We don't keep any Objective-C objects around because we
            // don't make any direct Objective-C calls in this code.
            with_autorelease(|| {
                get_list(&mut self.disks, refresh_kind);
            })
        }

        if remove_not_listed_disks {
            self.disks.retain_mut(|disk| {
                if !disk.inner.updated {
                    return false;
                }
                disk.inner.updated = false;
                true
            });
        } else {
            for c in self.disks.iter_mut() {
                c.inner.updated = false;
            }
        }
    }

    pub(crate) fn list(&self) -> &[Disk] {
        &self.disks
    }

    pub(crate) fn list_mut(&mut self) -> &mut [Disk] {
        &mut self.disks
    }
}

unsafe fn get_list(container: &mut Vec<Disk>, refresh_kind: DiskRefreshKind) {
    let raw_disks = {
        let count = libc::getfsstat(ptr::null_mut(), 0, libc::MNT_NOWAIT);
        if count < 1 {
            return;
        }
        let bufsize = count * std::mem::size_of::<libc::statfs>() as libc::c_int;
        let mut disks = Vec::with_capacity(count as _);
        let count = libc::getfsstat(disks.as_mut_ptr(), bufsize, libc::MNT_NOWAIT);

        if count < 1 {
            return;
        }

        disks.set_len(count as usize);

        disks
    };

    // Currently we query maximum 9 properties.
    let mut properties = Vec::with_capacity(9);
    // "mandatory" information
    properties.push(ffi::kCFURLVolumeNameKey);
    properties.push(ffi::kCFURLVolumeIsBrowsableKey);
    properties.push(ffi::kCFURLVolumeIsLocalKey);

    // is_removable
    properties.push(ffi::kCFURLVolumeIsEjectableKey);
    properties.push(ffi::kCFURLVolumeIsRemovableKey);
    properties.push(ffi::kCFURLVolumeIsInternalKey);

    if refresh_kind.storage() {
        properties.push(ffi::kCFURLVolumeTotalCapacityKey);
        properties.push(ffi::kCFURLVolumeAvailableCapacityForImportantUsageKey);
        properties.push(ffi::kCFURLVolumeAvailableCapacityKey);
    }

    // Create a list of properties about the disk that we want to fetch.
    let requested_properties = match build_requested_properties(&properties) {
        Some(properties) => properties,
        None => {
            sysinfo_debug!("failed to create volume key list");
            return;
        }
    };

    for c_disk in raw_disks {
        let volume_url = match CFReleaser::new(
            core_foundation_sys::url::CFURLCreateFromFileSystemRepresentation(
                kCFAllocatorDefault,
                c_disk.f_mntonname.as_ptr() as *const _,
                c_disk.f_mntonname.len() as _,
                false as _,
            ),
        ) {
            Some(url) => url,
            None => {
                sysinfo_debug!("getfsstat returned incompatible paths");
                continue;
            }
        };

        let prop_dict = match get_disk_properties(&volume_url, &requested_properties) {
            Some(props) => props,
            None => continue,
        };

        // Future note: There is a difference between `kCFURLVolumeIsBrowsableKey` and the
        // `kCFURLEnumeratorSkipInvisibles` option of `CFURLEnumeratorOptions`. Specifically,
        // the first one considers the writable `Data`(`/System/Volumes/Data`) partition to be
        // browsable, while it is classified as "invisible" by CoreFoundation's volume emumerator.
        let browsable = get_bool_value(
            prop_dict.inner(),
            DictKey::Extern(ffi::kCFURLVolumeIsBrowsableKey),
        )
        .unwrap_or_default();

        // Do not return invisible "disks". Most of the time, these are APFS snapshots, hidden
        // system volumes, etc. Browsable is defined to be visible in the system's UI like Finder,
        // disk utility, system information, etc.
        //
        // To avoid seemingly duplicating many disks and creating an inaccurate view of the system's
        // resources, these are skipped entirely.
        if !browsable {
            continue;
        }

        let local_only = get_bool_value(
            prop_dict.inner(),
            DictKey::Extern(ffi::kCFURLVolumeIsLocalKey),
        )
        .unwrap_or(true);

        // Skip any drive that is not locally attached to the system.
        //
        // This includes items like SMB mounts, and matches the other platform's behavior.
        if !local_only {
            continue;
        }

        let mount_point = PathBuf::from(OsStr::from_bytes(
            CStr::from_ptr(c_disk.f_mntonname.as_ptr()).to_bytes(),
        ));

        let disk = container
            .iter_mut()
            .find(|d| d.inner.mount_point == mount_point);
        if let Some(disk) = new_disk(
            disk,
            mount_point,
            volume_url,
            c_disk,
            &prop_dict,
            refresh_kind,
        ) {
            container.push(disk);
        }
    }
}

type RetainedCFArray = CFReleaser<core_foundation_sys::array::__CFArray>;
pub(crate) type RetainedCFDictionary = CFReleaser<core_foundation_sys::dictionary::__CFDictionary>;
type RetainedCFURL = CFReleaser<core_foundation_sys::url::__CFURL>;
#[cfg(target_os = "macos")]
pub(crate) type RetainedCFString = CFReleaser<core_foundation_sys::string::__CFString>;

unsafe fn build_requested_properties(properties: &[CFStringRef]) -> Option<RetainedCFArray> {
    CFReleaser::new(CFArrayCreate(
        ptr::null_mut(),
        properties.as_ptr() as *const *const c_void,
        properties.len() as _,
        &core_foundation_sys::array::kCFTypeArrayCallBacks,
    ))
}

fn get_disk_properties(
    volume_url: &RetainedCFURL,
    requested_properties: &RetainedCFArray,
) -> Option<RetainedCFDictionary> {
    CFReleaser::new(unsafe {
        ffi::CFURLCopyResourcePropertiesForKeys(
            volume_url.inner(),
            requested_properties.inner(),
            ptr::null_mut(),
        )
    })
}

fn get_available_volume_space(disk_props: &RetainedCFDictionary) -> Option<u64> {
    // We prefer `AvailableCapacityForImportantUsage` over `AvailableCapacity` because
    // it takes more of the system's properties into account, like the trash, system-managed caches,
    // etc. It generally also returns higher values too, because of the above, so it's a more
    // accurate representation of what the system _could_ still use.
    unsafe {
        get_int_value(
            disk_props.inner(),
            DictKey::Extern(ffi::kCFURLVolumeAvailableCapacityForImportantUsageKey),
        )
        .filter(|bytes| *bytes != 0)
        .or_else(|| {
            get_int_value(
                disk_props.inner(),
                DictKey::Extern(ffi::kCFURLVolumeAvailableCapacityKey),
            )
        })
    }
}

pub(super) enum DictKey {
    Extern(CFStringRef),
    #[cfg(target_os = "macos")]
    Defined(&'static str),
}

unsafe fn get_dict_value<T, F: FnOnce(*const c_void) -> Option<T>>(
    dict: CFDictionaryRef,
    key: DictKey,
    callback: F,
) -> Option<T> {
    #[cfg(target_os = "macos")]
    let _defined;
    #[allow(clippy::infallible_destructuring_match)]
    let key = match key {
        DictKey::Extern(val) => val,
        #[cfg(target_os = "macos")]
        DictKey::Defined(val) => {
            _defined = CFReleaser::new(cfs::CFStringCreateWithBytesNoCopy(
                kCFAllocatorDefault,
                val.as_ptr(),
                val.len() as _,
                cfs::kCFStringEncodingUTF8,
                false as _,
                core_foundation_sys::base::kCFAllocatorNull,
            ))?;

            _defined.inner()
        }
    };

    let mut value = std::ptr::null();
    if CFDictionaryGetValueIfPresent(dict, key.cast(), &mut value) != 0 {
        callback(value)
    } else {
        None
    }
}

pub(super) unsafe fn get_str_value(dict: CFDictionaryRef, key: DictKey) -> Option<String> {
    get_dict_value(dict, key, |v| {
        let v = v as cfs::CFStringRef;

        let len_utf16 = cfs::CFStringGetLength(v) as usize;
        let len_bytes = len_utf16 * 2; // Two bytes per UTF-16 codepoint.

        let v_ptr = cfs::CFStringGetCStringPtr(v, cfs::kCFStringEncodingUTF8);
        if v_ptr.is_null() {
            // Fallback on CFStringGetString to read the underlying bytes from the CFString.
            let mut buf = vec![0; len_bytes];
            let success = cfs::CFStringGetCString(
                v,
                buf.as_mut_ptr(),
                len_bytes as _,
                cfs::kCFStringEncodingUTF8,
            );

            if success != 0 {
                utils::vec_to_rust(buf)
            } else {
                None
            }
        } else {
            crate::unix::utils::cstr_to_rust_with_size(v_ptr, Some(len_bytes))
        }
    })
}

unsafe fn get_bool_value(dict: CFDictionaryRef, key: DictKey) -> Option<bool> {
    get_dict_value(dict, key, |v| Some(v as CFBooleanRef == kCFBooleanTrue))
}

pub(super) unsafe fn get_int_value(dict: CFDictionaryRef, key: DictKey) -> Option<u64> {
    get_dict_value(dict, key, |v| {
        let mut val: i64 = 0;
        if CFNumberGetValue(
            v.cast(),
            core_foundation_sys::number::kCFNumberSInt64Type,
            &mut val as *mut _ as *mut _,
        ) {
            Some(val as _)
        } else {
            None
        }
    })
}

unsafe fn new_disk(
    disk: Option<&mut Disk>,
    mount_point: PathBuf,
    volume_url: RetainedCFURL,
    c_disk: libc::statfs,
    disk_props: &RetainedCFDictionary,
    refresh_kind: DiskRefreshKind,
) -> Option<Disk> {
    let (total_space, available_space) = if refresh_kind.storage() {
        (
            get_int_value(
                disk_props.inner(),
                DictKey::Extern(ffi::kCFURLVolumeTotalCapacityKey),
            ),
            get_available_volume_space(disk_props),
        )
    } else {
        (None, None)
    };

    // We update the existing disk here to prevent having another call to get `storage` info.
    if let Some(disk) = disk {
        let disk = &mut disk.inner;
        if let Some(total_space) = total_space {
            disk.total_space = total_space;
        }
        if let Some(available_space) = available_space {
            disk.available_space = available_space;
        }
        disk.refresh_io(refresh_kind);
        disk.refresh_kind(refresh_kind);
        disk.updated = true;
        return None;
    }

    // Note: Since we requested these properties from the system, we don't expect
    // these property retrievals to fail.

    let name = get_str_value(
        disk_props.inner(),
        DictKey::Extern(ffi::kCFURLVolumeNameKey),
    )
    .map(OsString::from)?;

    let file_system = {
        let len = c_disk
            .f_fstypename
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(c_disk.f_fstypename.len());
        OsString::from_vec(
            c_disk.f_fstypename[..len]
                .iter()
                .map(|c| *c as u8)
                .collect(),
        )
    };

    #[cfg(target_os = "macos")]
    let bsd_name = get_bsd_name(&c_disk);

    // IOKit is not available on any but the most recent (16+) iOS and iPadOS versions.
    // Due to this, we can't query the medium type and disk i/o stats. All iOS devices use flash-based storage
    // so we just assume the disk type is an SSD and set disk i/o stats to 0 until Rust has a way to conditionally link to
    // IOKit in more recent deployment versions.

    let ejectable = get_bool_value(
        disk_props.inner(),
        DictKey::Extern(ffi::kCFURLVolumeIsEjectableKey),
    )
    .unwrap_or(false);

    let removable = get_bool_value(
        disk_props.inner(),
        DictKey::Extern(ffi::kCFURLVolumeIsRemovableKey),
    )
    .unwrap_or(false);

    let is_removable = if ejectable || removable {
        true
    } else {
        // If neither `ejectable` or `removable` return `true`, fallback to checking
        // if the disk is attached to the internal system.
        let internal = get_bool_value(
            disk_props.inner(),
            DictKey::Extern(ffi::kCFURLVolumeIsInternalKey),
        )
        .unwrap_or_default();

        !internal
    };

    let is_read_only = (c_disk.f_flags & libc::MNT_RDONLY as u32) != 0;

    let mut disk = DiskInner {
        type_: DiskKind::Unknown(-1),
        name,
        #[cfg(target_os = "macos")]
        bsd_name,
        file_system,
        mount_point,
        volume_url,
        total_space: total_space.unwrap_or(0),
        available_space: available_space.unwrap_or(0),
        is_removable,
        is_read_only,
        read_bytes: 0,
        written_bytes: 0,
        old_read_bytes: 0,
        old_written_bytes: 0,
        updated: true,
    };

    disk.refresh_kind(refresh_kind);
    disk.refresh_io(refresh_kind);

    Some(Disk { inner: disk })
}

/// Calls the provided closure in the context of a new autorelease pool that is drained
/// before returning.
///
/// ## SAFETY:
/// You must not return an Objective-C object that is autoreleased from this function since it
/// will be freed before usable.
unsafe fn with_autorelease<T, F: FnOnce() -> T>(call: F) -> T {
    // NB: This struct exists to help prevent memory leaking if `call` were to panic.
    // Otherwise, the call to `objc_autoreleasePoolPop` would never be made as the stack unwinds.
    // `Drop` destructors for existing types on the stack are run during unwinding, so we can
    // ensure the autorelease pool is drained by using a RAII pattern here.
    struct DrainPool {
        ctx: *mut c_void,
    }

    impl Drop for DrainPool {
        fn drop(&mut self) {
            // SAFETY: We have not manipulated `pool_ctx` since it was received from a corresponding
            // pool push call.
            unsafe { ffi::objc_autoreleasePoolPop(self.ctx) }
        }
    }

    // SAFETY: Creating a new pool is safe in any context. They can be arbitrarily nested
    // as long as pool objects are not used in deeper layers, but we only have one and don't
    // allow it to leave this scope.
    let _pool_ctx = DrainPool {
        ctx: unsafe { ffi::objc_autoreleasePoolPush() },
    };
    call()
    // Pool is drained here before returning
}

#[cfg(target_os = "macos")]
fn get_bsd_name(disk: &libc::statfs) -> Option<Vec<u8>> {
    // Removes `/dev/` from the value.
    unsafe {
        CStr::from_ptr(disk.f_mntfromname.as_ptr())
            .to_bytes_with_nul()
            .strip_prefix(b"/dev/")
            .map(|slice| slice.to_vec())
            .or_else(|| {
                sysinfo_debug!("unknown disk mount path format");
                None
            })
    }
}
