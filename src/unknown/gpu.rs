// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{Gpu, PCI};

pub(crate) struct GpusInner {
    pub(crate) gpus: Vec<Gpu>,
}

impl GpusInner {
    pub(crate) fn new() -> Result<Self, crate::Error> {
        Err(crate::Error::Unsupported)
    }

    pub(crate) fn refresh(&mut self) {
        unreachable!()
    }
}

pub(crate) struct GpuInner {
    pub(crate) updated: bool,
}

impl GpuInner {
    pub(crate) fn pci(&self) -> &PCI {
        unreachable!()
    }
    pub(crate) fn vendor(&self) -> Option<&str> {
        unreachable!()
    }
    pub(crate) fn model(&self) -> Option<&str> {
        unreachable!()
    }
    pub(crate) fn usage(&self) -> Option<f32> {
        unreachable!()
    }
    pub(crate) fn total_memory(&self) -> Option<u64> {
        unreachable!()
    }
    pub(crate) fn used_memory(&self) -> Option<u64> {
        unreachable!()
    }
}
