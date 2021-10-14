// Take a look at the license at the top of the repository in the LICENSE file.

use crate::ProcessorExt;

/// Dummy struct that represents a processor.
pub struct Processor {
    pub(crate) cpu_usage: f32,
    name: String,
    pub(crate) vendor_id: String,
    frequency: u64,
}

impl Processor {
    pub(crate) fn new(name: String, vendor_id: String, frequency: u64) -> Processor {
        Processor {
            cpu_usage: 0.,
            name,
            vendor_id,
            frequency,
        }
    }
}

impl ProcessorExt for Processor {
    fn cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn frequency(&self) -> u64 {
        self.frequency
    }

    fn vendor_id(&self) -> &str {
        &self.vendor_id
    }

    fn brand(&self) -> &str {
        ""
    }
}
