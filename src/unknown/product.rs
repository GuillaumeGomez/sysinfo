// Take a look at the license at the top of the repository in the LICENSE file.

use crate::Error;

pub(crate) struct ProductInner;

impl ProductInner {
    pub(crate) fn family() -> Result<String, Error> {
        Err(Error::Unsupported)
    }

    pub(crate) fn name() -> Result<String, Error> {
        Err(Error::Unsupported)
    }

    pub(crate) fn serial_number() -> Result<String, Error> {
        Err(Error::Unsupported)
    }

    pub(crate) fn stock_keeping_unit() -> Result<String, Error> {
        Err(Error::Unsupported)
    }

    pub(crate) fn uuid() -> Result<String, Error> {
        Err(Error::Unsupported)
    }

    pub(crate) fn version() -> Result<String, Error> {
        Err(Error::Unsupported)
    }

    pub(crate) fn vendor_name() -> Result<String, Error> {
        Err(Error::Unsupported)
    }
}
