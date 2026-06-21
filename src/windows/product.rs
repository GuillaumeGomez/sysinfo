// Take a look at the license at the top of the repository in the LICENSE file.

use crate::Error;

use super::ffi::SMBIOSSystemInformation;
use super::utils::{get_smbios_table, parse_smbios};

pub(crate) struct ProductInner;

impl ProductInner {
    pub(crate) fn family() -> Result<String, Error> {
        if let Some(table) = get_smbios_table()
            && let Some((info, strings)) = parse_smbios::<SMBIOSSystemInformation>(&table, 1)
            && info.family != 0
            && let Some(info) = strings
                .get(info.family as usize - 1)
                .copied()
                .map(str::to_string)
        {
            Ok(info)
        } else {
            Err(Error::Other("failed to retrieve product family".into()))
        }
    }

    pub(crate) fn name() -> Result<String, Error> {
        if let Some(table) = get_smbios_table()
            && let Some((info, strings)) = parse_smbios::<SMBIOSSystemInformation>(&table, 1)
            && info.product_name != 0
            && let Some(info) = strings
                .get(info.product_name as usize - 1)
                .copied()
                .map(str::to_string)
        {
            Ok(info)
        } else {
            Err(Error::Other("failed to retrieve product name".into()))
        }
    }

    pub(crate) fn serial_number() -> Result<String, Error> {
        if let Some(table) = get_smbios_table()
            && let Some((info, strings)) = parse_smbios::<SMBIOSSystemInformation>(&table, 1)
            && info.serial_number != 0
            && let Some(info) = strings
                .get(info.serial_number as usize - 1)
                .copied()
                .map(str::to_string)
        {
            Ok(info)
        } else {
            Err(Error::Other(
                "failed to retrieve product serial number".into(),
            ))
        }
    }

    pub(crate) fn stock_keeping_unit() -> Result<String, Error> {
        if let Some(table) = get_smbios_table()
            && let Some((info, strings)) = parse_smbios::<SMBIOSSystemInformation>(&table, 1)
            && info.sku_number != 0
            && let Some(info) = strings
                .get(info.sku_number as usize - 1)
                .copied()
                .map(str::to_string)
        {
            Ok(info)
        } else {
            Err(Error::Other(
                "failed to retrieve product stock keeping unit".into(),
            ))
        }
    }

    pub(crate) fn uuid() -> Result<String, Error> {
        if let Some(table) = get_smbios_table()
            && let Some(smbios) = parse_smbios::<SMBIOSSystemInformation>(&table, 1)
        {
            Ok(smbios.0.uuid.to_string())
        } else {
            Err(Error::Other("failed to retrieve product uuid".into()))
        }
    }

    pub(crate) fn version() -> Result<String, Error> {
        if let Some(table) = get_smbios_table()
            && let Some((info, strings)) = parse_smbios::<SMBIOSSystemInformation>(&table, 1)
            && info.version != 0
            && let Some(info) = strings
                .get(info.version as usize - 1)
                .copied()
                .map(str::to_string)
        {
            Ok(info)
        } else {
            Err(Error::Other("failed to retrieve product version".into()))
        }
    }

    pub(crate) fn vendor_name() -> Result<String, Error> {
        if let Some(table) = get_smbios_table()
            && let Some((info, strings)) = parse_smbios::<SMBIOSSystemInformation>(&table, 1)
            && info.manufacturer != 0
            && let Some(info) = strings
                .get(info.manufacturer as usize - 1)
                .copied()
                .map(str::to_string)
        {
            Ok(info)
        } else {
            Err(Error::Other(
                "failed to retrieve product vendor name".into(),
            ))
        }
    }
}
