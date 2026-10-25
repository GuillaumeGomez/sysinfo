// Take a look at the license at the top of the repository in the LICENSE file.

use std::collections::HashMap;
use std::ffi::OsString;

use crate::network::refresh_networks_addresses;
use crate::{Error, InterfaceOperationalState, IpNetwork, MacAddr, NetworkData};

pub(crate) struct NetworksInner {
    pub(crate) interfaces: HashMap<OsString, NetworkData>,
}

impl NetworksInner {
    pub(crate) fn new() -> Result<Self, Error> {
        if cfg!(target_os = "macos") {
            Ok(Self {
                interfaces: HashMap::new(),
            })
        } else {
            Err(Error::Unsupported)
        }
    }

    pub(crate) fn list(&self) -> &HashMap<OsString, NetworkData> {
        &self.interfaces
    }

    pub(crate) fn refresh(&mut self, remove_not_listed_interfaces: bool) {
        #[cfg(target_os = "macos")]
        crate::unix::apple::inner::network::update_networks(&mut self.interfaces);
        if remove_not_listed_interfaces {
            self.interfaces.retain(|_, i| {
                if !i.inner.updated {
                    return false;
                }
                i.inner.updated = false;
                true
            });
        } else {
            for i in self.interfaces.values_mut() {
                i.inner.updated = false;
            }
        }
        refresh_networks_addresses(&mut self.interfaces);
    }
}

#[derive(PartialEq, Eq)]
pub(crate) struct NetworkDataInner {
    pub(crate) current_in: u64,
    pub(crate) old_in: u64,
    pub(crate) current_out: u64,
    pub(crate) old_out: u64,
    pub(crate) packets_in: u64,
    pub(crate) old_packets_in: u64,
    pub(crate) packets_out: u64,
    pub(crate) old_packets_out: u64,
    pub(crate) errors_in: u64,
    pub(crate) old_errors_in: u64,
    pub(crate) errors_out: u64,
    pub(crate) old_errors_out: u64,
    pub(crate) updated: bool,
    /// MAC address
    pub(crate) mac_addr: MacAddr,
    /// IP networks
    pub(crate) ip_networks: Vec<IpNetwork>,
    /// Interface Maximum Transfer Unit (MTU)
    pub(crate) mtu: u64,
    /// Link speed in bits per second
    pub(crate) link_speed: Option<u64>,
    pub(crate) operational_state: InterfaceOperationalState,
}

impl NetworkDataInner {
    pub(crate) fn received(&self) -> u64 {
        self.current_in.saturating_sub(self.old_in)
    }

    pub(crate) fn total_received(&self) -> u64 {
        self.current_in
    }

    pub(crate) fn transmitted(&self) -> u64 {
        self.current_out.saturating_sub(self.old_out)
    }

    pub(crate) fn total_transmitted(&self) -> u64 {
        self.current_out
    }

    pub(crate) fn packets_received(&self) -> u64 {
        self.packets_in.saturating_sub(self.old_packets_in)
    }

    pub(crate) fn total_packets_received(&self) -> u64 {
        self.packets_in
    }

    pub(crate) fn packets_transmitted(&self) -> u64 {
        self.packets_out.saturating_sub(self.old_packets_out)
    }

    pub(crate) fn total_packets_transmitted(&self) -> u64 {
        self.packets_out
    }

    pub(crate) fn errors_on_received(&self) -> u64 {
        self.errors_in.saturating_sub(self.old_errors_in)
    }

    pub(crate) fn total_errors_on_received(&self) -> u64 {
        self.errors_in
    }

    pub(crate) fn errors_on_transmitted(&self) -> u64 {
        self.errors_out.saturating_sub(self.old_errors_out)
    }

    pub(crate) fn total_errors_on_transmitted(&self) -> u64 {
        self.errors_out
    }

    pub(crate) fn mac_address(&self) -> MacAddr {
        self.mac_addr
    }

    pub(crate) fn ip_networks(&self) -> &[IpNetwork] {
        &self.ip_networks
    }

    pub(crate) fn mtu(&self) -> u64 {
        self.mtu
    }

    pub(crate) fn transmit_link_speed(&self) -> Option<u64> {
        self.link_speed
    }

    pub(crate) fn receive_link_speed(&self) -> Option<u64> {
        self.link_speed
    }

    pub(crate) fn operational_state(&self) -> InterfaceOperationalState {
        self.operational_state
    }
}
