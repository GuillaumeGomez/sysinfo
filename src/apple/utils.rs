// Take a look at the license at the top of the repository in the LICENSE file.

use core_foundation_sys::base::CFRelease;
use libc::c_char;
use std::ptr::NonNull;

// A helper using to auto release the resource got from CoreFoundation.
// More information about the ownership policy for CoreFoundation pelease refer the link below:
// https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFMemoryMgmt/Concepts/Ownership.html#//apple_ref/doc/uid/20001148-CJBEJBHH
#[repr(transparent)]
pub(crate) struct CFReleaser<T>(NonNull<T>);

impl<T> CFReleaser<T> {
    pub(crate) fn new(ptr: *const T) -> Option<Self> {
        // This cast is OK because `NonNull` is a transparent wrapper
        // over a `*const T`. Additionally, mutability doesn't matter with
        // pointers here.
        NonNull::new(ptr as *mut T).map(Self)
    }

    pub(crate) fn inner(&self) -> *const T {
        self.0.as_ptr().cast()
    }
}

impl<T> Drop for CFReleaser<T> {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0.as_ptr().cast()) }
    }
}

// Safety: These are safe to implement because we only wrap non-mutable
// CoreFoundation types, which are generally threadsafe unless noted
// otherwise.
unsafe impl<T> Send for CFReleaser<T> {}
unsafe impl<T> Sync for CFReleaser<T> {}

pub(crate) fn cstr_to_rust(c: *const c_char) -> Option<String> {
    cstr_to_rust_with_size(c, None)
}

pub(crate) fn cstr_to_rust_with_size(c: *const c_char, size: Option<usize>) -> Option<String> {
    if c.is_null() {
        return None;
    }
    let mut s = match size {
        Some(len) => Vec::with_capacity(len),
        None => Vec::new(),
    };
    let mut i = 0;
    unsafe {
        loop {
            let value = *c.offset(i) as u8;
            if value == 0 {
                break;
            }
            s.push(value);
            i += 1;
        }
        String::from_utf8(s).ok()
    }
}

pub(crate) fn vec_to_rust(buf: Vec<i8>) -> Option<String> {
    String::from_utf8(
        buf.into_iter()
            .flat_map(|b| if b > 0 { Some(b as u8) } else { None })
            .collect(),
    )
    .ok()
}
