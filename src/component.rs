// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use std::fmt::{Debug, Error, Formatter};
use sys::Component;

impl Debug for Component {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        if let Some(critical) = self.critical {
            write!(f, "{}: {}°C (max: {}°C / critical: {}°C)",
                   self.label, self.temperature, self.max, critical)
        } else {
            write!(f, "{}: {}°C (max: {}°C)",
                   self.label, self.temperature, self.max)
        }
    }
}
