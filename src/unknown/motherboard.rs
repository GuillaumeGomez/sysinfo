// Take a look at the license at the top of the repository in the LICENSE file.

pub(crate) struct MotherboardInner;

impl MotherboardInner {
    pub(crate) fn new() -> Option<Self> {
        None
    }

    pub(crate) fn name(&self) -> Option<String> {
        unreachable!()
    }

    pub(crate) fn vendor(&self) -> Option<String> {
        unreachable!()
    }

    pub(crate) fn version(&self) -> Option<String> {
        unreachable!()
    }

    pub(crate) fn serial(&self) -> Option<String> {
        unreachable!()
    }

    pub(crate) fn asset_tag(&self) -> Option<String> {
        unreachable!()
    }
}
