// Take a look at the license at the top of the repository in the LICENSE file.

use crate::CpuExt;

#[doc = include_str!("../../md_doc/cpu.md")]
pub struct Cpu {}

impl Cpu {
    pub(crate) fn new() -> Cpu {
        Cpu {}
    }
}

impl CpuExt for Cpu {
    fn cpu_usage(&self) -> f32 {
        0.0
    }

    fn name(&self) -> &str {
        ""
    }

    fn frequency(&self) -> u64 {
        0
    }

    fn vendor_id(&self) -> &str {
        ""
    }

    fn brand(&self) -> &str {
        ""
    }
}
