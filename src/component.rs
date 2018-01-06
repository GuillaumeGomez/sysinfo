// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use std::fmt::{Debug, Error, Formatter};
use sys::Component;
use traits::ComponentExt;

impl Debug for Component {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        if let Some(critical) = self.get_critical() {
            write!(f, "{}: {}°C (max: {}°C / critical: {}°C)",
                   self.get_label(), self.get_temperature(), self.get_max(), critical)
        } else {
            write!(f, "{}: {}°C (max: {}°C)",
                   self.get_label(), self.get_temperature(), self.get_max())
        }
    }
}
