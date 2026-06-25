// Take a look at the license at the top of the repository in the LICENSE file.

use crate::Error;
use crate::sys::utils::get_kenv_var;

pub(crate) struct ProductInner;

impl ProductInner {
    pub(crate) fn family() -> Result<String, Error> {
        get_kenv_var(b"smbios.system.family\0")
            .ok_or_else(|| Error::Other("failed to retrieve product family".into()))
    }

    pub(crate) fn name() -> Result<String, Error> {
        get_kenv_var(b"smbios.system.product\0")
            .ok_or_else(|| Error::Other("failed to retrieve product name".into()))
    }

    pub(crate) fn serial_number() -> Result<String, Error> {
        get_kenv_var(b"smbios.system.serial\0")
            .ok_or_else(|| Error::Other("failed to retrieve product serial number".into()))
    }

    pub(crate) fn stock_keeping_unit() -> Result<String, Error> {
        get_kenv_var(b"smbios.system.sku\0")
            .ok_or_else(|| Error::Other("failed to retrieve product stock keeping unit".into()))
    }

    pub(crate) fn uuid() -> Result<String, Error> {
        get_kenv_var(b"smbios.system.uuid\0")
            .ok_or_else(|| Error::Other("failed to retrieve product uuid".into()))
    }

    pub(crate) fn version() -> Result<String, Error> {
        get_kenv_var(b"smbios.system.version\0")
            .ok_or_else(|| Error::Other("failed to retrieve product version".into()))
    }

    pub(crate) fn vendor_name() -> Result<String, Error> {
        get_kenv_var(b"smbios.system.maker\0")
            .ok_or_else(|| Error::Other("failed to retrieve product vendor name".into()))
    }
}
