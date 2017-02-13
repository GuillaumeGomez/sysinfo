// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use std::fmt::{self, Debug, Formatter};
use sys::Processor;
use ::ProcessorExt;

impl Debug for Processor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}: {}%", self.get_name(), self.get_cpu_usage())
    }
}
