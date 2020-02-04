//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use std::collections::{HashMap, HashSet};

use windows::ffi::{self, MIB_IF_ROW2, PMIB_IF_TABLE2};
use NetworkExt;
use NetworksExt;
use NetworksIter;

use winapi::shared::ifdef::NET_LUID;
use winapi::shared::winerror::NO_ERROR;

macro_rules! old_and_new {
    ($ty_:expr, $name:ident, $old:ident, $new_val:expr) => {{
        $ty_.$old = $ty_.$name;
        $ty_.$name = $new_val;
    }};
}

/// Network interfaces.
///
/// ```no_run
/// use sysinfo::{NetworksExt, System, SystemExt};
///
/// let s = System::new();
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

    fn refresh_interfaces_list(&mut self) {
        let mut table: PMIB_IF_TABLE2 = ::std::ptr::null_mut();
        if unsafe { ffi::GetIfTable2(&mut table) } != NO_ERROR {
            return;
        }
        let mut to_be_removed = HashSet::with_capacity(self.interfaces.len());

        for key in self.interfaces.keys() {
            to_be_removed.insert(key.clone());
        }
        // In here, this is tricky: we have to filter out the software interfaces to only keep
        // the hardware ones. To do so, we first check the connection potential speed (if 0, not
        // interesting), then we check its state: if not open, not interesting either. And finally,
        // we count the members of a same group: if there is more than 1, then it's software level.
        let mut groups = HashMap::new();
        let mut indexes = Vec::new();
        let ptr = unsafe { (*table).Table.as_ptr() };
        for i in 0..unsafe { *table }.NumEntries {
            let ptr = unsafe { &*ptr.offset(i as _) };
            if ptr.TransmitLinkSpeed == 0 && ptr.ReceiveLinkSpeed == 0 {
                continue;
            } else if ptr.MediaConnectState == ffi::MediaConnectStateDisconnected
                || ptr.PhysicalAddressLength == 0
            {
                continue;
            }
            let id = vec![
                ptr.InterfaceGuid.Data2,
                ptr.InterfaceGuid.Data3,
                ptr.InterfaceGuid.Data4[0] as _,
                ptr.InterfaceGuid.Data4[1] as _,
                ptr.InterfaceGuid.Data4[2] as _,
                ptr.InterfaceGuid.Data4[3] as _,
                ptr.InterfaceGuid.Data4[4] as _,
                ptr.InterfaceGuid.Data4[5] as _,
                ptr.InterfaceGuid.Data4[6] as _,
                ptr.InterfaceGuid.Data4[7] as _,
            ];
            let entry = groups.entry(id.clone()).or_insert(0);
            *entry += 1;
            if *entry > 1 {
                continue;
            }
            indexes.push((i, id));
        }
        for (i, id) in indexes {
            let ptr = unsafe { &*ptr.offset(i as _) };
            if *groups.get(&id).unwrap_or(&0) > 1 {
                continue;
            }
            let mut pos = 0;
            for x in ptr.Alias.iter() {
                if *x == 0 {
                    break;
                }
                pos += 1;
            }
            let interface_name = match String::from_utf16(&ptr.Alias[..pos]) {
                Ok(s) => s,
                _ => continue,
            };
            to_be_removed.remove(&interface_name);
            let mut interface =
                self.interfaces
                    .entry(interface_name)
                    .or_insert_with(|| NetworkData {
                        id: ptr.InterfaceLuid,
                        current_out: ptr.OutOctets,
                        old_out: ptr.OutOctets,
                        current_in: ptr.InOctets,
                        old_in: ptr.InOctets,
                    });
            old_and_new!(interface, current_out, old_out, ptr.OutOctets);
            old_and_new!(interface, current_in, old_in, ptr.InOctets);
        }
        unsafe {
            ffi::FreeMibTable(table as _);
        }
        for key in to_be_removed {
            self.interfaces.remove(&key);
        }
    }

    fn refresh(&mut self) {
        let mut entry: MIB_IF_ROW2 = unsafe { ::std::mem::MaybeUninit::uninit().assume_init() };
        for (_, interface) in self.interfaces.iter_mut() {
            entry.InterfaceLuid = interface.id;
            entry.InterfaceIndex = 0; // to prevent the function to pick this one as index
            if unsafe { ffi::GetIfEntry2(&mut entry) } != NO_ERROR {
                continue;
            }
            old_and_new!(interface, current_out, old_out, entry.OutOctets);
            old_and_new!(interface, current_in, old_in, entry.InOctets);
        }
    }
}

/// Contains network information.
pub struct NetworkData {
    id: NET_LUID,
    current_out: u64,
    old_out: u64,
    current_in: u64,
    old_in: u64,
}

impl NetworkExt for NetworkData {
    fn get_income(&self) -> u64 {
        self.current_in - self.old_in
    }

    fn get_outcome(&self) -> u64 {
        self.current_out - self.old_out
    }

    fn get_total_income(&self) -> u64 {
        self.current_in
    }

    fn get_total_outcome(&self) -> u64 {
        self.current_out
    }
}
