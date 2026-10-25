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
