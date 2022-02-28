// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::processor::{self, Processor, Query};

use std::mem::zeroed;

use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};

pub(crate) struct KeyHandler {
    pub unique_id: String,
}

impl KeyHandler {
    pub fn new(unique_id: String) -> KeyHandler {
        KeyHandler { unique_id }
    }
}

pub(crate) fn init_processors() -> (Vec<Processor>, String, String) {
    unsafe {
        let mut sys_info: SYSTEM_INFO = zeroed();
        GetSystemInfo(&mut sys_info);
        let (vendor_id, brand) = processor::get_vendor_id_and_brand(&sys_info);
        let frequencies = processor::get_frequencies(sys_info.dwNumberOfProcessors as usize);
        let mut ret = Vec::with_capacity(sys_info.dwNumberOfProcessors as usize + 1);
        for nb in 0..sys_info.dwNumberOfProcessors {
            ret.push(Processor::new_with_values(
                &format!("CPU {}", nb + 1),
                vendor_id.clone(),
                brand.clone(),
                frequencies[nb as usize],
            ));
        }
        (ret, vendor_id, brand)
    }
}

pub(crate) fn add_english_counter(
    s: String,
    query: &mut Query,
    keys: &mut Option<KeyHandler>,
    counter_name: String,
) {
    let mut full = s.encode_utf16().collect::<Vec<_>>();
    full.push(0);
    if query.add_english_counter(&counter_name, full) {
        *keys = Some(KeyHandler::new(counter_name));
    }
}
