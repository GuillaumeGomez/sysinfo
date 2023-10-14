// Take a look at the license at the top of the repository in the LICENSE file.

use crate::common::MacAddr;
use crate::{NetworkExt, NetworksIter};

use std::collections::HashMap;

pub(crate) struct NetworksInner {
    pub(crate) interfaces: HashMap<String, NetworkData>,
}

impl NetworksInner {
    pub(crate) fn new() -> Self {
        Self {
            interfaces: HashMap::new(),
        }
    }

    pub(crate) fn iter(&self) -> NetworksIter {
        NetworksIter::new(self.interfaces.iter())
    }

    pub(crate) fn refresh_list(&mut self) {}

    pub(crate) fn refresh(&mut self) {}
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
}
