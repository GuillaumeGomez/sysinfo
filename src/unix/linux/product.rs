// Take a look at the license at the top of the repository in the LICENSE file.

use crate::Error;

pub(crate) struct ProductInner;

impl ProductInner {
    pub(crate) fn family() -> Result<String, Error> {
        std::fs::read_to_string("/sys/devices/virtual/dmi/id/product_family")
            .map(|s| s.trim().to_owned())
            .map_err(|_| Error::Other("failed to retrieve product family".into()))
    }

    pub(crate) fn name() -> Result<String, Error> {
        std::fs::read_to_string("/sys/devices/virtual/dmi/id/product_name")
            .or_else(|_| {
                std::fs::read_to_string("/sys/firmware/devicetree/base/model")
                    .or_else(|_| {
                        std::fs::read_to_string("/sys/firmware/devicetree/base/banner-name")
                    })
                    .or_else(|_| std::fs::read_to_string("/tmp/sysinfo/model"))
                    .map(|s| s.trim_end_matches('\0').to_owned())
            })
            .map(|s| s.trim().to_owned())
            .map_err(|_| Error::Other("failed to retrieve product name".into()))
    }

    pub(crate) fn serial_number() -> Result<String, Error> {
        std::fs::read_to_string("/sys/devices/virtual/dmi/id/product_serial")
            .or_else(|_| {
                std::fs::read_to_string("/sys/firmware/devicetree/base/serial-number")
                    .map(|s| s.trim_end_matches('\0').to_owned())
            })
            .map(|s| s.trim().to_owned())
            .map_err(|_| Error::Other("failed to retrieve product serial number".into()))
    }

    pub(crate) fn stock_keeping_unit() -> Result<String, Error> {
        std::fs::read_to_string("/sys/devices/virtual/dmi/id/product_sku")
            .map(|s| s.trim().to_owned())
            .map_err(|_| Error::Other("failed to retrieve product stock keeping unit".into()))
    }

    pub(crate) fn uuid() -> Result<String, Error> {
        std::fs::read_to_string("/sys/devices/virtual/dmi/id/product_uuid")
            .map(|s| s.trim().to_owned())
            .map_err(|_| Error::Other("failed to retrieve product uuid".into()))
    }

    pub(crate) fn version() -> Result<String, Error> {
        std::fs::read_to_string("/sys/devices/virtual/dmi/id/product_version")
            .map(|s| s.trim().to_owned())
            .map_err(|_| Error::Other("failed to retrieve product version".into()))
    }

    pub(crate) fn vendor_name() -> Result<String, Error> {
        std::fs::read_to_string("/sys/devices/virtual/dmi/id/sys_vendor")
            .map(|s| s.trim().to_owned())
            .map_err(|_| Error::Other("failed to retrieve product vendor name".into()))
    }
}
