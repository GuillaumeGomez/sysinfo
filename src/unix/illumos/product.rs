// Take a look at the license at the top of the repository in the LICENSE file.

use super::smbios::{Smbios, SystemInfo};
use crate::Error;

pub(crate) struct ProductInner;

impl ProductInner {
    pub(crate) fn family() -> Result<String, Error> {
        system_info()?
            .family
            .ok_or_else(|| Error::Other("failed to retrieve product family".into()))
    }

    pub(crate) fn name() -> Result<String, Error> {
        system_info()?
            .common
            .product
            .ok_or_else(|| Error::Other("failed to retrieve product name".into()))
    }

    pub(crate) fn serial_number() -> Result<String, Error> {
        system_info()?
            .common
            .serial
            .ok_or_else(|| Error::Other("failed to retrieve product serial number".into()))
    }

    pub(crate) fn stock_keeping_unit() -> Result<String, Error> {
        system_info()?
            .sku
            .ok_or_else(|| Error::Other("failed to retrieve product stock keeping unit".into()))
    }

    pub(crate) fn uuid() -> Result<String, Error> {
        system_info()?
            .uuid
            .ok_or_else(|| Error::Other("failed to retrieve product uuid".into()))
    }

    pub(crate) fn version() -> Result<String, Error> {
        system_info()?
            .common
            .version
            .ok_or_else(|| Error::Other("failed to retrieve product version".into()))
    }

    pub(crate) fn vendor_name() -> Result<String, Error> {
        system_info()?
            .common
            .manufacturer
            .ok_or_else(|| Error::Other("failed to retrieve product vendor name".into()))
    }
}

fn system_info() -> Result<SystemInfo, Error> {
    Smbios::new()
        .ok_or_else(|| Error::Other("failed to open system SMBIOS data".into()))?
        .system_info()
        .ok_or_else(|| Error::Other("failed to retrieve system SMBIOS data".into()))
}
