// Take a look at the license at the top of the repository in the LICENSE file.

use crate::Error;
use crate::sys::cpu::get_sysctl_str;
#[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
use crate::sys::macos::system::get_io_platform_property;

pub(crate) struct ProductInner;

impl ProductInner {
    pub(crate) fn family() -> Result<String, Error> {
        match get_sysctl_str(b"hw.model\0") {
            Some(info) => Ok(info),
            None => Err(Error::Other("failed to retrieve product family".into())),
        }
    }

    pub(crate) fn name() -> Result<String, Error> {
        match Self::family() {
            Ok(family) => Ok(family),
            Err(_) => {
                cfg_select! {
                    all(target_os = "macos", not(feature = "apple-sandbox")) => {
                        match get_io_platform_property("product-name") {
                            Some(info) => Ok(info),
                            None => Err(Error::Other("failed to retrieve product name".into())),
                        }
                    }
                    _ => Err(Error::Unsupported),
                }
            }
        }
    }

    pub(crate) fn serial_number() -> Result<String, Error> {
        cfg_select! {
            all(target_os = "macos", not(feature = "apple-sandbox")) => {
                use objc2_io_kit::kIOPlatformSerialNumberKey;
                match get_io_platform_property(
                    unsafe { std::str::from_utf8_unchecked(kIOPlatformSerialNumberKey.to_bytes()) },
                ) {
                    Some(info) => Ok(info),
                    None => Err(Error::Other("failed to retrieve product serial number".into())),
                }
            }
            _ => Err(Error::Unsupported),
        }
    }

    pub(crate) fn stock_keeping_unit() -> Result<String, Error> {
        Err(Error::Unsupported)
    }

    pub(crate) fn uuid() -> Result<String, Error> {
        cfg_select! {
            all(target_os = "macos", not(feature = "apple-sandbox")) => {
                use objc2_io_kit::kIOPlatformUUIDKey;
                match get_io_platform_property(
                    unsafe { std::str::from_utf8_unchecked(kIOPlatformUUIDKey.to_bytes()) }
                ) {
                    Some(info) => Ok(info),
                    None => Err(Error::Other("failed to retrieve product uuid".into())),
                }
            }
            _ => Err(Error::Unsupported),
        }
    }

    pub(crate) fn version() -> Result<String, Error> {
        cfg_select! {
            all(target_os = "macos", not(feature = "apple-sandbox")) => {
                match get_io_platform_property("version") {
                    Some(info) => Ok(info),
                    None => Err(Error::Other("failed to retrieve product version".into())),
                }
            }
            _ => Err(Error::Unsupported),
        }
    }

    pub(crate) fn vendor_name() -> Result<String, Error> {
        crate::Motherboard::new()?
            .vendor_name()
            .ok_or_else(|| Error::Other("failed to retrieve vendor name".into()))
    }
}
