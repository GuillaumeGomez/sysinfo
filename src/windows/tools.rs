// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::cpu::Query;

pub(crate) struct KeyHandler {
    pub unique_id: String,
}

impl KeyHandler {
    pub fn new(unique_id: String) -> KeyHandler {
        KeyHandler { unique_id }
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
