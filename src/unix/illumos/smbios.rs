// Take a look at the license at the top of the repository in the LICENSE file.

use libc::{c_char, c_int, c_void};
use std::ffi::CStr;
use std::ptr;

const SMB_VERSION_28: c_int = 0x0208;
const SMB_TYPE_BASEBOARD: u32 = 2;
const SMB_ERR: i32 = -1;

#[repr(C)]
struct SmbiosHandle {
    _private: [u8; 0],
}

#[repr(C)]
struct SmbiosStruct {
    id: i32,
    _structure_type: u32,
    _data: *const c_void,
    _size: usize,
}

#[repr(C)]
struct SmbiosInfo {
    manufacturer: *const c_char,
    product: *const c_char,
    version: *const c_char,
    serial: *const c_char,
    asset: *const c_char,
    _location: *const c_char,
    _part: *const c_char,
}

#[repr(C)]
struct SmbiosSystem {
    uuid: *const u8,
    uuid_len: u8,
    _wakeup: u8,
    sku: *const c_char,
    family: *const c_char,
}

#[derive(Default)]
pub(super) struct CommonInfo {
    pub(super) manufacturer: Option<String>,
    pub(super) product: Option<String>,
    pub(super) version: Option<String>,
    pub(super) serial: Option<String>,
    pub(super) asset: Option<String>,
}

#[derive(Default)]
pub(super) struct SystemInfo {
    pub(super) common: CommonInfo,
    pub(super) sku: Option<String>,
    pub(super) family: Option<String>,
    pub(super) uuid: Option<String>,
}

pub(super) struct Smbios {
    handle: *mut SmbiosHandle,
}

impl Smbios {
    pub(super) fn new() -> Option<Self> {
        let mut error = 0;
        // SAFETY: A null path asks libsmbios to open the system SMBIOS device. `error` is a valid
        // output pointer and the returned handle is checked before use.
        let handle = unsafe { smbios_open(ptr::null(), SMB_VERSION_28, 0, &mut error) };
        (!handle.is_null()).then_some(Self { handle })
    }

    pub(super) fn system_info(&self) -> Option<SystemInfo> {
        let mut system = std::mem::MaybeUninit::<SmbiosSystem>::uninit();
        // SAFETY: The handle remains valid for this call and `system` points to writable storage.
        let id = unsafe { smbios_info_system(self.handle, system.as_mut_ptr()) };
        if id == SMB_ERR {
            return None;
        }

        // SAFETY: A successful `smbios_info_system` call initialized the structure.
        let system = unsafe { system.assume_init() };
        Some(SystemInfo {
            common: self.common_info(id)?,
            // SAFETY: libsmbios owns these strings and keeps them alive for the handle's lifetime.
            sku: unsafe { copy_string(system.sku) },
            // SAFETY: See above.
            family: unsafe { copy_string(system.family) },
            // SAFETY: libsmbios reports the UUID buffer length alongside the pointer, and both
            // remain valid for the handle's lifetime.
            uuid: unsafe { copy_uuid(system.uuid, system.uuid_len) },
        })
    }

    pub(super) fn baseboard_info(&self) -> Option<CommonInfo> {
        let mut structure = std::mem::MaybeUninit::<SmbiosStruct>::uninit();
        // SAFETY: The handle remains valid for this call and `structure` points to writable
        // storage. SMB_TYPE_BASEBOARD is a valid SMBIOS structure type.
        if unsafe { smbios_lookup_type(self.handle, SMB_TYPE_BASEBOARD, structure.as_mut_ptr()) }
            < 0
        {
            return None;
        }

        // SAFETY: A successful lookup initialized the structure.
        self.common_info(unsafe { structure.assume_init() }.id)
    }

    fn common_info(&self, id: i32) -> Option<CommonInfo> {
        let mut info = std::mem::MaybeUninit::<SmbiosInfo>::uninit();
        // SAFETY: The handle and structure ID came from libsmbios and `info` points to writable
        // storage.
        if unsafe { smbios_info_common(self.handle, id, info.as_mut_ptr()) } < 0 {
            return None;
        }

        // SAFETY: A successful `smbios_info_common` call initialized the structure. Its string
        // pointers remain valid while the handle is open.
        let info = unsafe { info.assume_init() };
        Some(CommonInfo {
            // SAFETY: Each pointer is either null or a NUL-terminated libsmbios string.
            manufacturer: unsafe { copy_string(info.manufacturer) },
            // SAFETY: See above.
            product: unsafe { copy_string(info.product) },
            // SAFETY: See above.
            version: unsafe { copy_string(info.version) },
            // SAFETY: See above.
            serial: unsafe { copy_string(info.serial) },
            // SAFETY: See above.
            asset: unsafe { copy_string(info.asset) },
        })
    }
}

impl Drop for Smbios {
    fn drop(&mut self) {
        // SAFETY: `handle` was returned by `smbios_open` and is closed exactly once here.
        unsafe { smbios_close(self.handle) };
    }
}

unsafe fn copy_string(value: *const c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }

    // SAFETY: The caller guarantees that `value` points to a NUL-terminated C string.
    let value = unsafe { CStr::from_ptr(value) }.to_string_lossy();
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

unsafe fn copy_uuid(value: *const u8, len: u8) -> Option<String> {
    if value.is_null() || len != 16 {
        return None;
    }

    // SAFETY: libsmbios reported that the UUID buffer contains exactly 16 bytes.
    let bytes = unsafe { std::slice::from_raw_parts(value, 16) };
    if bytes.iter().all(|byte| *byte == 0) || bytes.iter().all(|byte| *byte == u8::MAX) {
        return None;
    }

    Some(format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
        u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
        u16::from_le_bytes(bytes[6..8].try_into().unwrap()),
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15],
    ))
}

#[link(name = "smbios")]
unsafe extern "C" {
    fn smbios_open(
        path: *const c_char,
        version: c_int,
        flags: c_int,
        error: *mut c_int,
    ) -> *mut SmbiosHandle;
    fn smbios_close(handle: *mut SmbiosHandle);
    fn smbios_lookup_type(
        handle: *mut SmbiosHandle,
        structure_type: u32,
        structure: *mut SmbiosStruct,
    ) -> c_int;
    fn smbios_info_common(handle: *mut SmbiosHandle, id: i32, info: *mut SmbiosInfo) -> c_int;
    fn smbios_info_system(handle: *mut SmbiosHandle, system: *mut SmbiosSystem) -> i32;
}

const _: [(); 24] = [(); std::mem::size_of::<SmbiosStruct>()];
const _: [(); 56] = [(); std::mem::size_of::<SmbiosInfo>()];
const _: [(); 32] = [(); std::mem::size_of::<SmbiosSystem>()];

#[cfg(test)]
mod tests {
    use super::copy_uuid;

    #[test]
    fn formats_uuid_using_smbios_byte_order() {
        let bytes = [
            0x75, 0x36, 0x21, 0x57, 0xe8, 0x09, 0x14, 0x44, 0xa3, 0x94, 0x8f, 0x9f, 0xbb, 0x30,
            0xaa, 0x7c,
        ];
        assert_eq!(
            // SAFETY: `bytes` is a valid 16-byte buffer.
            unsafe { copy_uuid(bytes.as_ptr(), bytes.len() as u8) }.as_deref(),
            Some("57213675-09e8-4414-a394-8f9fbb30aa7c"),
        );
    }

    #[test]
    fn rejects_unset_uuids() {
        // SAFETY: Both arrays are valid 16-byte buffers.
        assert_eq!(unsafe { copy_uuid([0; 16].as_ptr(), 16) }, None);
        // SAFETY: See above.
        assert_eq!(unsafe { copy_uuid([u8::MAX; 16].as_ptr(), 16) }, None);
    }
}
