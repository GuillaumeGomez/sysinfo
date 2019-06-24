// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use ::ProcessorExt;

/// Dummy struct that represents a processor.
pub struct Processor {
}

impl ProcessorExt for Processor {
    fn get_cpu_usage(&self) -> f32 {
        0.0
    }

    fn get_name(&self) -> &str {
        ""
    }
}
