// Take a look at the license at the top of the repository in the LICENSE file.

use std::collections::{HashMap, hash_map};
use std::mem::MaybeUninit;

use super::utils;
use crate::network::refresh_networks_addresses;
use crate::{IpNetwork, MacAddr, NetworkData};

macro_rules! old_and_new {
    ($ty_:expr, $name:ident, $old:ident, $data:expr) => {{
        $ty_.$old = $ty_.$name;
        $ty_.$name = $data.$name;
    }};
}

pub(crate) struct NetworksInner {
    pub(crate) interfaces: HashMap<String, NetworkData>,
}

impl NetworksInner {
    pub(crate) fn new() -> Self {
        Self {
            interfaces: HashMap::new(),
        }
    }

    pub(crate) fn list(&self) -> &HashMap<String, NetworkData> {
        &self.interfaces
    }

    pub(crate) fn refresh(&mut self, remove_not_listed_interfaces: bool) {
        unsafe {
            self.refresh_interfaces(true);
        }
        if remove_not_listed_interfaces {
            // Remove interfaces which are gone.
            self.interfaces.retain(|_, i| {
                if !i.inner.updated {
                    return false;
                }
                i.inner.updated = false;
                true
            });
        }
        refresh_networks_addresses(&mut self.interfaces);
    }

    unsafe fn refresh_interfaces(&mut self, refresh_all: bool) {
        // struct ifaddrs *ifaddrs = NULL;
        // struct ifaddrs *ifa = NULL;
        // int found = 0;
        // int num = 0;

        // let ret = getifaddrs(&ifaddrs);
        // if ret != 0 {
        //     sysinfo_debug!("`getifaddrs` failed with value {ret}");
        //     return;
        // }

        // for (ifa = ifaddrs; ifa != NULL; ifa = ifa->ifa_next) {
        //   if (ifa->ifa_addr &&
        //       ifa->ifa_addr->sa_family == AF_LINK) {
        //     num++;

        //     /*
        //      * expecting rows to start with 1 not 0,
        //      * see freebsd "man ifmib"
        //      */
        //     if (num == row) {
        //       found = 1;
        //       data->ifdr_data = *(struct if_data *)ifa->ifa_data;
        //       strncpy(data->ifdr_name, ifa->ifa_name, IF_NAMESIZE);
        //       break;
        //     }
        //   }
        // }

        // freeifaddrs(ifaddrs);

        // if (!found) {
        //   IFSTAT_ERR(2, "getifmibdata() error finding row");
        // }
    }
}

pub(crate) struct NetworkDataInner {
    /// Total number of bytes received over interface.
    ifi_ibytes: u64,
    old_ifi_ibytes: u64,
    /// Total number of bytes transmitted over interface.
    ifi_obytes: u64,
    old_ifi_obytes: u64,
    /// Total number of packets received.
    ifi_ipackets: u64,
    old_ifi_ipackets: u64,
    /// Total number of packets transmitted.
    ifi_opackets: u64,
    old_ifi_opackets: u64,
    /// Shows the total number of packets received with error. This includes
    /// too-long-frames errors, ring-buffer overflow errors, CRC errors,
    /// frame alignment errors, fifo overruns, and missed packets.
    ifi_ierrors: u64,
    old_ifi_ierrors: u64,
    /// similar to `ifi_ierrors`
    ifi_oerrors: u64,
    old_ifi_oerrors: u64,
    /// Whether or not the above data has been updated during refresh
    updated: bool,
    /// MAC address
    pub(crate) mac_addr: MacAddr,
    /// IP networks
    pub(crate) ip_networks: Vec<IpNetwork>,
    /// Interface Maximum Transfer Unit (MTU)
    mtu: u64,
}

impl NetworkDataInner {
    pub(crate) fn received(&self) -> u64 {
        self.ifi_ibytes.saturating_sub(self.old_ifi_ibytes)
    }

    pub(crate) fn total_received(&self) -> u64 {
        self.ifi_ibytes
    }

    pub(crate) fn transmitted(&self) -> u64 {
        self.ifi_obytes.saturating_sub(self.old_ifi_obytes)
    }

    pub(crate) fn total_transmitted(&self) -> u64 {
        self.ifi_obytes
    }

    pub(crate) fn packets_received(&self) -> u64 {
        self.ifi_ipackets.saturating_sub(self.old_ifi_ipackets)
    }

    pub(crate) fn total_packets_received(&self) -> u64 {
        self.ifi_ipackets
    }

    pub(crate) fn packets_transmitted(&self) -> u64 {
        self.ifi_opackets.saturating_sub(self.old_ifi_opackets)
    }

    pub(crate) fn total_packets_transmitted(&self) -> u64 {
        self.ifi_opackets
    }

    pub(crate) fn errors_on_received(&self) -> u64 {
        self.ifi_ierrors.saturating_sub(self.old_ifi_ierrors)
    }

    pub(crate) fn total_errors_on_received(&self) -> u64 {
        self.ifi_ierrors
    }

    pub(crate) fn errors_on_transmitted(&self) -> u64 {
        self.ifi_oerrors.saturating_sub(self.old_ifi_oerrors)
    }

    pub(crate) fn total_errors_on_transmitted(&self) -> u64 {
        self.ifi_oerrors
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
}
