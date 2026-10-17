// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::CStr;
#[cfg(any(feature = "network", feature = "system"))]
use std::ffi::CString;
use std::ptr;

use super::ffi;

pub(crate) struct KstatReader {
    ctl: *mut ffi::KstatCtl,
}

impl KstatReader {
    pub(crate) fn new() -> Option<Self> {
        let ctl = unsafe { ffi::kstat_open() };
        (!ctl.is_null()).then_some(Self { ctl })
    }

    fn find(&self, module: Option<&CStr>, instance: i32, name: Option<&CStr>) -> *mut ffi::Kstat {
        unsafe {
            ffi::kstat_lookup(
                self.ctl,
                module.map_or(ptr::null(), CStr::as_ptr),
                instance,
                name.map_or(ptr::null(), CStr::as_ptr),
            )
        }
    }

    #[cfg(any(feature = "network", feature = "system"))]
    pub(crate) fn lookup(
        &self,
        module: Option<&CStr>,
        instance: i32,
        name: Option<&CStr>,
    ) -> Option<KstatRecord<'_>> {
        let stat = self.find(module, instance, name);
        if stat.is_null() || unsafe { ffi::kstat_read(self.ctl, stat, ptr::null_mut()) } < 0 {
            None
        } else {
            Some(KstatRecord {
                _reader: self,
                stat,
            })
        }
    }

    #[cfg(feature = "disk")]
    pub(crate) fn read_io(
        &self,
        module: Option<&CStr>,
        instance: i32,
        name: Option<&CStr>,
    ) -> Option<ffi::KstatIo> {
        let stat = self.find(module, instance, name);
        if stat.is_null() {
            return None;
        }
        let mut io = std::mem::MaybeUninit::<ffi::KstatIo>::uninit();
        if unsafe { ffi::kstat_read(self.ctl, stat, io.as_mut_ptr().cast()) } < 0 {
            None
        } else {
            Some(unsafe { io.assume_init() })
        }
    }
}

impl Drop for KstatReader {
    fn drop(&mut self) {
        unsafe {
            ffi::kstat_close(self.ctl);
        }
    }
}

#[cfg(any(feature = "network", feature = "system"))]
pub(crate) struct KstatRecord<'a> {
    _reader: &'a KstatReader,
    stat: *mut ffi::Kstat,
}

#[cfg(any(feature = "network", feature = "system"))]
impl KstatRecord<'_> {
    fn named(&self, name: &str) -> Option<&ffi::KstatNamed> {
        let name = CString::new(name).ok()?;
        let value = unsafe { ffi::kstat_data_lookup(self.stat, name.as_ptr()) };
        unsafe { (value as *const ffi::KstatNamed).as_ref() }
    }

    pub(crate) fn integer(&self, name: &str) -> Option<u64> {
        let named = self.named(name)?;
        unsafe {
            match named.data_type {
                ffi::KSTAT_DATA_INT32 => Some(named.value.i32_.max(0) as u64),
                ffi::KSTAT_DATA_UINT32 => Some(named.value.u32_ as u64),
                ffi::KSTAT_DATA_INT64 => Some(named.value.i64_.max(0) as u64),
                ffi::KSTAT_DATA_UINT64 => Some(named.value.u64_),
                _ => None,
            }
        }
    }

    #[cfg(feature = "system")]
    pub(crate) fn string(&self, name: &str) -> Option<String> {
        let named = self.named(name)?;
        unsafe {
            match named.data_type {
                ffi::KSTAT_DATA_CHAR => Some(
                    CStr::from_ptr(named.value.chars.as_ptr())
                        .to_string_lossy()
                        .into_owned(),
                ),
                ffi::KSTAT_DATA_STRING => {
                    let value = named.value.string;
                    (!value.ptr.is_null())
                        .then(|| CStr::from_ptr(value.ptr).to_string_lossy().into_owned())
                }
                _ => None,
            }
        }
    }
}
