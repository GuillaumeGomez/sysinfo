// Take a look at the license at the top of the repository in the LICENSE file.

use crate::CpuExt;

#[doc = include_str!("../../md_doc/cpu.md")]
pub struct Cpu {
    pub(crate) cpu_usage: f32,
    name: String,
    pub(crate) vendor_id: String,
    pub(crate) frequency: u64,
}

impl Cpu {
    pub(crate) fn new(name: String, vendor_id: String, frequency: u64) -> Cpu {
        Cpu {
            cpu_usage: 0.,
            name,
            vendor_id,
            frequency,
        }
    }
}

impl CpuExt for Cpu {
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
