//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use std::collections::HashMap;

use crate::{NetworkExt, NetworksExt, NetworksIter};

/// Network interfaces.
///
/// ```no_run
/// use sysinfo::{NetworksExt, System, SystemExt};
///
/// let s = System::new_all();
/// let networks = s.networks();
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
    fn received(&self) -> u64 {
        0
    }

    fn total_received(&self) -> u64 {
        0
    }

    fn transmitted(&self) -> u64 {
        0
    }

    fn total_transmitted(&self) -> u64 {
        0
    }

    fn packets_received(&self) -> u64 {
        0
    }

    fn total_packets_received(&self) -> u64 {
        0
    }

    fn packets_transmitted(&self) -> u64 {
        0
    }

    fn total_packets_transmitted(&self) -> u64 {
        0
    }

    fn errors_on_received(&self) -> u64 {
        0
    }

    fn total_errors_on_received(&self) -> u64 {
        0
    }

    fn errors_on_transmitted(&self) -> u64 {
        0
    }

    fn total_errors_on_transmitted(&self) -> u64 {
        0
    }
}
