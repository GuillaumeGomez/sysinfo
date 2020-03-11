//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use ProcessorExt;

/// Dummy struct that represents a processor.
pub struct Processor {}

impl Processor {
    pub(crate) fn new() -> Processor {
        Processor {}
    }
}

impl ProcessorExt for Processor {
    fn get_cpu_usage(&self) -> f32 {
        0.0
    }

    fn get_name(&self) -> &str {
        ""
    }

    fn get_frequency(&self) -> u64 {
        0
    }

    fn get_vendor_id(&self) -> &str {
        ""
    }

    fn get_brand(&self) -> &str {
        ""
    }
}
