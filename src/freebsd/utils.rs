// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{Pid, Process};
use libc::{c_char, c_int, timeval};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::ffi::CStr;
use std::mem;
use std::time::SystemTime;

/// This struct is used to switch between the "old" and "new" every time you use "get_mut".
#[derive(Debug)]
pub(crate) struct VecSwitcher<T> {
    v1: Vec<T>,
    v2: Vec<T>,
    first: bool,
}

impl<T: Clone> VecSwitcher<T> {
    pub fn new(v1: Vec<T>) -> Self {
        let v2 = v1.clone();

        Self {
            v1,
            v2,
            first: true,
        }
    }

    pub fn get_mut(&mut self) -> &mut [T] {
        self.first = !self.first;
        if self.first {
            // It means that `v2` will be the "new".
            &mut self.v2
        } else {
            // It means that `v1` will be the "new".
            &mut self.v1
        }
    }

    pub fn get_old(&self) -> &[T] {
        if self.first {
            &self.v1
        } else {
            &self.v2
        }
    }

    pub fn get_new(&self) -> &[T] {
        if self.first {
            &self.v2
        } else {
            &self.v1
        }
    }
}

#[inline]
pub unsafe fn init_mib(name: &[u8], mib: &mut [c_int]) {
    let mut len = mib.len();
    libc::sysctlnametomib(name.as_ptr() as _, mib.as_mut_ptr(), &mut len);
}

pub(crate) fn boot_time() -> u64 {
    let mut boot_time = timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let mut len = std::mem::size_of::<timeval>();
    let mut mib: [c_int; 2] = [libc::CTL_KERN, libc::KERN_BOOTTIME];
    unsafe {
        if libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as _,
            &mut boot_time as *mut timeval as *mut _,
            &mut len,
            std::ptr::null_mut(),
            0,
        ) < 0
        {
            0
        } else {
            boot_time.tv_sec as _
        }
    }
}

pub(crate) unsafe fn get_sys_value<T: Sized>(mib: &[c_int], value: &mut T) -> bool {
    let mut len = mem::size_of::<T>() as libc::size_t;
    libc::sysctl(
        mib.as_ptr(),
        mib.len() as _,
        value as *mut _ as *mut _,
        &mut len,
        std::ptr::null_mut(),
        0,
    ) == 0
}

pub(crate) unsafe fn get_sys_value_array<T: Sized>(mib: &[c_int], value: &mut [T]) -> bool {
    let mut len = mem::size_of_val(value) as libc::size_t;
    libc::sysctl(
        mib.as_ptr(),
        mib.len() as _,
        value.as_mut_ptr() as *mut _,
        &mut len as *mut _,
        std::ptr::null_mut(),
        0,
    ) == 0
}

pub(crate) fn c_buf_to_str(buf: &[libc::c_char]) -> Option<&str> {
    unsafe {
        let buf: &[u8] = std::slice::from_raw_parts(buf.as_ptr() as _, buf.len());
        if let Some(pos) = buf.iter().position(|x| *x == 0) {
            // Shrink buffer to terminate the null bytes
            std::str::from_utf8(&buf[..pos]).ok()
        } else {
            std::str::from_utf8(buf).ok()
        }
    }
}

pub(crate) fn c_buf_to_string(buf: &[libc::c_char]) -> Option<String> {
    c_buf_to_str(buf).map(|s| s.to_owned())
}

pub(crate) unsafe fn get_sys_value_str(mib: &[c_int], buf: &mut [libc::c_char]) -> Option<String> {
    let mut len = mem::size_of_val(buf) as libc::size_t;
    if libc::sysctl(
        mib.as_ptr(),
        mib.len() as _,
        buf.as_mut_ptr() as *mut _,
        &mut len,
        std::ptr::null_mut(),
        0,
    ) != 0
    {
        return None;
    }
    c_buf_to_string(&buf[..len / mem::size_of::<libc::c_char>()])
}

pub(crate) unsafe fn get_sys_value_by_name<T: Sized>(name: &[u8], value: &mut T) -> bool {
    let mut len = mem::size_of::<T>() as libc::size_t;
    let original = len;

    libc::sysctlbyname(
        name.as_ptr() as *const c_char,
        value as *mut _ as *mut _,
        &mut len,
        std::ptr::null_mut(),
        0,
    ) == 0
        && original == len
}

pub(crate) fn get_sys_value_str_by_name(name: &[u8]) -> Option<String> {
    let mut size = 0;

    unsafe {
        if libc::sysctlbyname(
            name.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        ) == 0
            && size > 0
        {
            // now create a buffer with the size and get the real value
            let mut buf: Vec<libc::c_char> = vec![0; size as _];

            if libc::sysctlbyname(
                name.as_ptr() as *const c_char,
                buf.as_mut_ptr() as *mut _,
                &mut size,
                std::ptr::null_mut(),
                0,
            ) == 0
                && size > 0
            {
                c_buf_to_string(&buf)
            } else {
                // getting the system value failed
                None
            }
        } else {
            None
        }
    }
}

pub(crate) fn get_system_info(mib: &[c_int], default: Option<&str>) -> Option<String> {
    let mut size = 0;

    unsafe {
        // Call first to get size
        libc::sysctl(
            mib.as_ptr(),
            mib.len() as _,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        );

        // exit early if we did not update the size
        if size == 0 {
            default.map(|s| s.to_owned())
        } else {
            // set the buffer to the correct size
            let mut buf: Vec<libc::c_char> = vec![0; size as _];

            if libc::sysctl(
                mib.as_ptr(),
                mib.len() as _,
                buf.as_mut_ptr() as _,
                &mut size,
                std::ptr::null_mut(),
                0,
            ) == -1
            {
                // If command fails return default
                default.map(|s| s.to_owned())
            } else {
                c_buf_to_string(&buf)
            }
        }
    }
}

pub(crate) unsafe fn from_cstr_array(ptr: *const *const c_char) -> Vec<String> {
    if ptr.is_null() {
        return Vec::new();
    }
    let mut max = 0;
    loop {
        let ptr = ptr.add(max);
        if (*ptr).is_null() {
            break;
        }
        max += 1;
    }
    if max == 0 {
        return Vec::new();
    }
    let mut ret = Vec::with_capacity(max);

    for pos in 0..max {
        let p = ptr.add(pos);
        if let Ok(s) = CStr::from_ptr(*p).to_str() {
            ret.push(s.to_owned());
        }
    }
    ret
}

pub(crate) fn get_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|n| n.as_secs())
        .unwrap_or(0)
}

// All this is needed because `kinfo_proc` doesn't implement `Send` (because it contains pointers).
pub(crate) struct WrapMap<'a>(pub UnsafeCell<&'a mut HashMap<Pid, Process>>);

unsafe impl<'a> Send for WrapMap<'a> {}
unsafe impl<'a> Sync for WrapMap<'a> {}

#[repr(transparent)]
pub(crate) struct KInfoProc(libc::kinfo_proc);
unsafe impl Send for KInfoProc {}
unsafe impl Sync for KInfoProc {}

impl std::ops::Deref for KInfoProc {
    type Target = libc::kinfo_proc;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(crate) unsafe fn get_frequency_for_cpu(cpu_nb: c_int) -> u64 {
    let mut frequency = 0;

    // The information can be missing if it's running inside a VM.
    if !get_sys_value_by_name(
        format!("dev.cpu.{cpu_nb}.freq\0").as_bytes(),
        &mut frequency,
    ) {
        frequency = 0;
    }
    frequency as _
}
