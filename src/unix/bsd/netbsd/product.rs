// Take a look at the license at the top of the repository in the LICENSE file.

use super::utils::get_sys_value_str_by_name;
use crate::Error;

pub(crate) struct ProductInner;

impl ProductInner {
    pub(crate) fn family() -> Result<String, Error> {
        // FIXME
        Err(Error::Unsupported)
    }

    pub(crate) fn name() -> Result<String, Error> {
        // FIXME: Wrong mib
        get_sys_value_str_by_name(b"machdep.dmi.system-product\0")
            .ok_or_else(|| Error::Other("failed to retrieve product name".into()))
    }

    pub(crate) fn serial_number() -> Result<String, Error> {
        // FIXME
        get_sys_value_str_by_name(b"machdep.dmi.system-serial\0")
            .ok_or_else(|| Error::Other("failed to retrieve product serial number".into()))
    }

    pub(crate) fn stock_keeping_unit() -> Result<String, Error> {
        // FIXME
        Err(Error::Unsupported)
    }

    pub(crate) fn uuid() -> Result<String, Error> {
        // FIXME: Wrong mib
        get_sys_value_str_by_name(b"machdep.dmi.system-uuid\0")
            .ok_or_else(|| Error::Other("failed to retrieve product uuid".into()))
    }

    pub(crate) fn version() -> Result<String, Error> {
        // FIXME: Wrong mib
        get_sys_value_str_by_name(b"machdep.dmi.system-version\0")
            .ok_or_else(|| Error::Other("failed to retrieve product version".into()))
    }

    pub(crate) fn vendor_name() -> Result<String, Error> {
        // FIXME: Wrong mib
        get_sys_value_str_by_name(b"machdep.dmi.system-vendor\0")
            .ok_or_else(|| Error::Other("failed to retrieve product vendor name".into()))
    }
}
