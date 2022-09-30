// Take a look at the license at the top of the repository in the LICENSE file.

use core_foundation_sys::base::CFRelease;

// A helper using to auto release the resource got from CoreFoundation.
// More information about the ownership policy for CoreFoundation pelease refer the link below:
// https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFMemoryMgmt/Concepts/Ownership.html#//apple_ref/doc/uid/20001148-CJBEJBHH
#[repr(transparent)]
pub(crate) struct CFReleaser<T>(*const T);

impl<T> CFReleaser<T> {
    pub(crate) fn new(ptr: *const T) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(Self(ptr))
        }
    }

    pub(crate) fn inner(&self) -> *const T {
        self.0
    }
}

impl<T> Drop for CFReleaser<T> {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0 as _) }
        }
    }
}

unsafe impl<T> Send for CFReleaser<T> {}
unsafe impl<T> Sync for CFReleaser<T> {}

pub(crate) struct IOReleaser(super::ffi::io_object_t);

impl IOReleaser {
    pub(crate) fn new(obj: u32) -> Option<Self> {
        if obj == 0 {
            None
        } else {
            Some(Self(obj))
        }
    }

    pub(crate) fn inner(&self) -> u32 {
        self.0
    }
}

impl Drop for IOReleaser {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe { super::ffi::IOObjectRelease(self.0 as _) };
        }
    }
}
