// Take a look at the license at the top of the repository in the LICENSE file.

use std::collections::HashMap;
use std::net::Ipv4Addr;

use crate::common::MacAddr;
use crate::{NetworkExt, NetworksExt, NetworksIter};

#[doc = include_str!("../../md_doc/networks.md")]
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
    fn iter(&self) -> NetworksIter {
        NetworksIter::new(self.interfaces.iter())
    }

    fn refresh_networks_list(&mut self) {}

    fn refresh(&mut self) {}
}

#[doc = include_str!("../../md_doc/network_data.md")]
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

    fn mac_address(&self) -> MacAddr {
        MacAddr::UNSPECIFIED
    }

    fn ipv4_address(&self) -> Ipv4Addr {
        Ipv4Addr::UNSPECIFIED
    }

    fn ipv4_netmask(&self) -> Ipv4Addr {
        Ipv4Addr::UNSPECIFIED
    }
}
