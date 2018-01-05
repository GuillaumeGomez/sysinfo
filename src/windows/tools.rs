// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

pub struct KeyHandler {
    pub unique_id: String,
    pub win_key: Vec<u16>,
}

impl KeyHandler {
    pub fn new(unique_id: String, win_key: Vec<u16>) -> KeyHandler {
        KeyHandler {
            unique_id: unique_id,
            win_key: win_key,
        }
    }
}
