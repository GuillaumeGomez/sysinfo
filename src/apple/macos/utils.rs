// Take a look at the license at the top of the repository in the LICENSE file.

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
