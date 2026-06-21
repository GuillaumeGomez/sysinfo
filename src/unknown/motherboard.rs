// Take a look at the license at the top of the repository in the LICENSE file.

use crate::Error;

pub(crate) struct MotherboardInner;

impl MotherboardInner {
    pub(crate) fn new() -> Result<Self, Error> {
        Err(Error::Unsupported)
    }

    pub(crate) fn name(&self) -> Option<String> {
        unreachable!()
    }

    pub(crate) fn vendor_name(&self) -> Option<String> {
        unreachable!()
    }

    pub(crate) fn version(&self) -> Option<String> {
        unreachable!()
    }

    pub(crate) fn serial_number(&self) -> Option<String> {
        unreachable!()
    }

    pub(crate) fn asset_tag(&self) -> Option<String> {
        unreachable!()
    }
}
