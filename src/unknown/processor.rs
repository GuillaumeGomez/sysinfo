//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

use std::default::Default;

use LoadAvg;
use ProcessorExt;

/// Dummy struct that represents a processor.
pub struct Processor {}

impl ProcessorExt for Processor {
    fn get_cpu_usage(&self) -> f32 {
        0.0
    }

    fn get_name(&self) -> &str {
        ""
    }
}

pub fn get_cpu_frequency() -> u64 {
    0
}

/// get_avg_load returns the system load average value.
pub fn get_avg_load() -> LoadAvg {
    LoadAvg::default()
}
