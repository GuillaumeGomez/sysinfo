// Take a look at the license at the top of the repository in the LICENSE file.

#[cfg(feature = "disk")]
#[link(name = "objc", kind = "dylib")]
unsafe extern "C" {
    pub fn objc_autoreleasePoolPop(pool: *mut libc::c_void);
    pub fn objc_autoreleasePoolPush() -> *mut libc::c_void;
}

// Since `libc` 0.2.190, the following items are only available on macOS, even though they are
// available on iOS as well, so we declare them ourselves.

#[cfg(feature = "system")]
unsafe extern "C" {
    static mach_task_self_: libc::mach_port_t;
    pub fn mach_host_self() -> libc::mach_port_t;
}

#[cfg(feature = "system")]
pub unsafe fn mach_task_self() -> libc::mach_port_t {
    unsafe { mach_task_self_ }
}

// net/route.h
#[cfg(feature = "network")]
pub const RTM_IFINFO2: libc::c_int = 0x12;

// net/if_mib.h
#[cfg(feature = "network")]
pub const NETLINK_GENERIC: libc::c_int = 0;
#[cfg(feature = "network")]
pub const IFMIB_IFDATA: libc::c_int = 2;
#[cfg(feature = "network")]
pub const IFDATA_GENERAL: libc::c_int = 1;

#[cfg(feature = "network")]
#[repr(C)]
pub struct ifmibdata {
    /// Name of interface
    pub ifmd_name: [libc::c_char; libc::IFNAMSIZ],
    /// Number of promiscuous listeners
    pub ifmd_pcount: libc::c_uint,
    /// Interface flags
    pub ifmd_flags: libc::c_uint,
    /// Instantaneous length of send queue
    pub ifmd_snd_len: libc::c_uint,
    /// Maximum length of send queue
    pub ifmd_snd_maxlen: libc::c_uint,
    /// Number of drops in send queue
    pub ifmd_snd_drops: libc::c_uint,
    /// For future expansion
    pub ifmd_filler: [libc::c_uint; 4],
    /// Generic information and statistics
    pub ifmd_data: libc::if_data64,
}
