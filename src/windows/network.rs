//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

use std::collections::HashMap;

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
        let ptr = unsafe { (*table).Table.as_ptr() };
        for i in 0..unsafe { *table }.NumEntries {
            let ptr = unsafe { &*ptr.offset(i as _) };
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
            let mut interface = self.interfaces.entry(interface_name).or_insert_with(|| {
                NetworkData {
                    id: ptr.InterfaceLuid,
                    current_out: ptr.OutOctets,
                    old_out: ptr.OutOctets,
                    current_in: ptr.InOctets,
                    old_in: ptr.InOctets,
                }
            });
            old_and_new!(interface, current_out, old_out, ptr.OutOctets);
            old_and_new!(interface, current_in, old_in, ptr.InOctets);
        }
        unsafe { ffi::FreeMibTable(table as _); }
    }

    fn refresh(&mut self) {
        let mut entry: MIB_IF_ROW2 = unsafe { ::std::mem::MaybeUninit::uninit().assume_init() };
        for (_, interface) in self.interfaces.iter_mut() {
            entry.InterfaceLuid = interface.id;
            if unsafe { ffi::GetIfEntry2(&mut entry) } != NO_ERROR {
                old_and_new!(interface, current_out, old_out, entry.OutOctets);
                old_and_new!(interface, current_in, old_in, entry.InOctets);
            }
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
        self.current_in
    }
}
