// Take a look at the license at the top of the repository in the LICENSE file.

use super::ffi;
use libc::mach_port_t;

pub struct IoObject(ffi::io_object_t);

impl IoObject {
    fn new(obj: ffi::io_object_t) -> Option<Self> {
        if obj == 0 {
            None
        } else {
            Some(Self(obj))
        }
    }

    fn inner(&self) -> ffi::io_object_t {
        self.0
    }
}

impl Drop for IoObject {
    fn drop(&mut self) {
        unsafe {
            ffi::IOObjectRelease(self.0);
        }
    }
}

pub(crate) struct IoService(ffi::io_connect_t);

impl IoService {
    fn new(obj: ffi::io_connect_t) -> Option<Self> {
        if obj == 0 {
            None
        } else {
            Some(Self(obj))
        }
    }

    pub(crate) fn inner(&self) -> ffi::io_connect_t {
        self.0
    }

    // code from https://github.com/Chris911/iStats
    // Not supported on iOS, or in the default macOS
    pub(crate) fn new_connection() -> Option<Self> {
        let mut master_port: mach_port_t = 0;
        let mut iterator: ffi::io_iterator_t = 0;

        unsafe {
            ffi::IOMasterPort(libc::MACH_PORT_NULL, &mut master_port);

            let matching_dictionary = ffi::IOServiceMatching(b"AppleSMC\0".as_ptr() as *const i8);
            let result =
                ffi::IOServiceGetMatchingServices(master_port, matching_dictionary, &mut iterator);
            if result != ffi::KIO_RETURN_SUCCESS {
                sysinfo_debug!("Error: IOServiceGetMatchingServices() = {}", result);
                return None;
            }
            let iterator = match IoObject::new(iterator) {
                Some(i) => i,
                None => {
                    sysinfo_debug!("Error: IOServiceGetMatchingServices() succeeded but returned invalid descriptor");
                    return None;
                }
            };

            let device = match IoObject::new(ffi::IOIteratorNext(iterator.inner())) {
                Some(d) => d,
                None => {
                    sysinfo_debug!("Error: no SMC found");
                    return None;
                }
            };

            let mut conn = 0;
            let result = ffi::IOServiceOpen(device.inner(), libc::mach_task_self(), 0, &mut conn);
            if result != ffi::KIO_RETURN_SUCCESS {
                sysinfo_debug!("Error: IOServiceOpen() = {}", result);
                return None;
            }
            let conn = IoService::new(conn);
            if conn.is_none() {
                sysinfo_debug!(
                    "Error: IOServiceOpen() succeeded but returned invalid descriptor..."
                );
            }
            conn
        }
    }
}

impl Drop for IoService {
    fn drop(&mut self) {
        unsafe {
            ffi::IOServiceClose(self.0);
        }
    }
}
