// Take a look at the license at the top of the repository in the LICENSE file.

use super::smbios::{CommonInfo, Smbios};
use crate::Error;

pub(crate) struct MotherboardInner;

impl MotherboardInner {
    pub(crate) fn new() -> Result<Self, Error> {
        Smbios::new()
            .map(|_| Self)
            .ok_or_else(|| Error::Other("failed to open system SMBIOS data".into()))
    }

    pub(crate) fn asset_tag(&self) -> Option<String> {
        baseboard_info()?.asset
    }

    pub(crate) fn name(&self) -> Option<String> {
        baseboard_info()?.product
    }

    pub(crate) fn vendor_name(&self) -> Option<String> {
        baseboard_info()?.manufacturer
    }

    pub(crate) fn version(&self) -> Option<String> {
        baseboard_info()?.version
    }

    pub(crate) fn serial_number(&self) -> Option<String> {
        baseboard_info()?.serial
    }
}

fn baseboard_info() -> Option<CommonInfo> {
    Smbios::new()?.baseboard_info()
}
