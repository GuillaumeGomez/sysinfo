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
    if unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as _,
            &mut boot_time as *mut timeval as *mut _,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    } < 0
    {
        0
    } else {
        boot_time.tv_sec as _
    }
}

pub unsafe fn get_sys_value<T: Sized>(mib: &[c_int], value: &mut T) -> bool {
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

pub unsafe fn get_sys_value_array<T: Sized>(mib: &[c_int], value: &mut [T]) -> bool {
    let mut len = (mem::size_of::<T>() * value.len()) as libc::size_t;
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

pub unsafe fn get_sys_value_str(mib: &[c_int], buf: &mut [libc::c_char]) -> Option<String> {
    let mut len = (mem::size_of::<libc::c_char>() * buf.len()) as libc::size_t;
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

pub unsafe fn get_sys_value_by_name<T: Sized>(name: &[u8], value: &mut T) -> bool {
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
            let mut buf: Vec<libc::c_char> = vec![0; size as usize];

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

    // Call first to get size
    unsafe {
        libc::sysctl(
            mib.as_ptr(),
            mib.len() as _,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };

    // exit early if we did not update the size
    if size == 0 {
        default.map(|s| s.to_owned())
    } else {
        // set the buffer to the correct size
        let mut buf: Vec<libc::c_char> = vec![0; size as usize];

        if unsafe {
            libc::sysctl(
                mib.as_ptr(),
                mib.len() as _,
                buf.as_mut_ptr() as _,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        } == -1
        {
            // If command fails return default
            default.map(|s| s.to_owned())
        } else {
            c_buf_to_string(&buf)
        }
    }
}

pub unsafe fn from_cstr_array(ptr: *const *const c_char) -> Vec<String> {
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

pub(crate) struct ProcList<'a>(pub &'a [libc::kinfo_proc]);
unsafe impl<'a> Send for ProcList<'a> {}

pub(crate) struct WrapItem<'a>(pub &'a libc::kinfo_proc);
unsafe impl<'a> Send for WrapItem<'a> {}
unsafe impl<'a> Sync for WrapItem<'a> {}

pub(crate) struct IntoIter<'a>(std::slice::Iter<'a, libc::kinfo_proc>);
unsafe impl<'a> Send for IntoIter<'a> {}

impl<'a> std::iter::Iterator for IntoIter<'a> {
    type Item = WrapItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(WrapItem)
    }
}

impl<'a> std::iter::ExactSizeIterator for IntoIter<'a> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> std::iter::IntoIterator for ProcList<'a> {
    type Item = WrapItem<'a>;
    type IntoIter = IntoIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.iter())
    }
}

#[cfg(feature = "multithread")]
mod multithread {
    use super::{IntoIter, ProcList, WrapItem};
    use rayon::iter::plumbing::{bridge, Consumer, Producer, ProducerCallback, UnindexedConsumer};
    use rayon::prelude::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

    struct IterProducer<'a>(std::slice::Iter<'a, libc::kinfo_proc>);
    unsafe impl<'a> Send for IterProducer<'a> {}
    unsafe impl<'a> Sync for IterProducer<'a> {}

    impl<'a> Producer for IterProducer<'a> {
        type Item = WrapItem<'a>;
        type IntoIter = IntoIter<'a>;

        fn into_iter(self) -> Self::IntoIter {
            IntoIter(self.0)
        }

        fn split_at(self, index: usize) -> (Self, Self) {
            let (left, right) = self.0.as_slice().split_at(index);
            (IterProducer(left.iter()), IterProducer(right.iter()))
        }
    }

    impl<'a> IntoParallelIterator for ProcList<'a> {
        type Item = WrapItem<'a>;
        type Iter = IntoIter<'a>;

        fn into_par_iter(self) -> Self::Iter {
            IntoIter(self.0.iter())
        }
    }

    impl<'a> std::iter::DoubleEndedIterator for IntoIter<'a> {
        fn next_back(&mut self) -> Option<Self::Item> {
            self.0.next_back().map(WrapItem)
        }
    }

    impl<'a> ParallelIterator for IntoIter<'a> {
        type Item = WrapItem<'a>;

        fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where
            C: UnindexedConsumer<Self::Item>,
        {
            bridge(self, consumer)
        }

        fn opt_len(&self) -> Option<usize> {
            Some(self.0.len())
        }
    }

    impl<'a> IndexedParallelIterator for IntoIter<'a> {
        fn drive<C>(self, consumer: C) -> C::Result
        where
            C: Consumer<Self::Item>,
        {
            bridge(self, consumer)
        }

        fn len(&self) -> usize {
            self.0.len()
        }

        fn with_producer<CB>(self, callback: CB) -> CB::Output
        where
            CB: ProducerCallback<Self::Item>,
        {
            callback.callback(IterProducer(self.0))
        }
    }
}
