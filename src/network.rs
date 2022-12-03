// Take a look at the license at the top of the repository in the LICENSE file.

use std::collections::HashMap;

use crate::common::InterfaceAddress;
use crate::network_helper::get_interface_address;
use crate::NetworkData;

/// Interface addresses are OS-independent
pub(crate) fn refresh_networks_addresses(interfaces: &mut HashMap<String, NetworkData>) {
    if let Ok(ifa_iterator) = get_interface_address() {
        for (name, ifa) in ifa_iterator {
            if let Some(interface) = interfaces.get_mut(&name) {
                match ifa {
                    InterfaceAddress::MAC(mac_address) => {
                        interface.mac_addr = mac_address;
                    }
                    InterfaceAddress::IPv4(address, netmask) => {
                        interface.ipv4_addr = address;
                        interface.ipv4_mask = netmask;
                    }
                    _ => {}
                }
            }
        }
    }
}
