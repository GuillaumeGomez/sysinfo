//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use std::collections::HashMap;

use NetworkExt;
use NetworksExt;
use NetworksIter;

/// Network interfaces.
///
/// ```no_run
/// use sysinfo::{NetworksExt, System, SystemExt};
///
/// let s = System::new_all();
/// let networks = s.get_networks();
/// ```
pub struct Networks {
    interfaces: HashMap<String, NetworkData>,
}

impl Networks {
    pub(crate) fn new() -> Networks {
        Networks {
            interfaces: HashMap::new(),
        }
    }
}

impl NetworksExt for Networks {
    fn iter<'a>(&'a self) -> NetworksIter<'a> {
        NetworksIter::new(self.interfaces.iter())
    }

    fn refresh_networks_list(&mut self) {}

    fn refresh(&mut self) {}
}

/// Contains network information.
pub struct NetworkData;

impl NetworkExt for NetworkData {
    fn get_received(&self) -> u64 {
        0
    }

    fn get_total_received(&self) -> u64 {
        0
    }

    fn get_transmitted(&self) -> u64 {
        0
    }

    fn get_total_transmitted(&self) -> u64 {
        0
    }

    fn get_packets_received(&self) -> u64 {
        0
    }

    fn get_total_packets_received(&self) -> u64 {
        0
    }

    fn get_packets_transmitted(&self) -> u64 {
        0
    }

    fn get_total_packets_transmitted(&self) -> u64 {
        0
    }

    fn get_errors_on_received(&self) -> u64 {
        0
    }

    fn get_total_errors_on_received(&self) -> u64 {
        0
    }

    fn get_errors_on_transmitted(&self) -> u64 {
        0
    }

    fn get_total_errors_on_transmitted(&self) -> u64 {
        0
    }
}
