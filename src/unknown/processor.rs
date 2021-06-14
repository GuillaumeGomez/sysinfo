//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use crate::ProcessorExt;

/// Dummy struct that represents a processor.
pub struct Processor {}

impl Processor {
    pub(crate) fn new() -> Processor {
        Processor {}
    }
}

impl ProcessorExt for Processor {
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
