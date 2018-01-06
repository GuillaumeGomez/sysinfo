// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

use NetworkExt;
use windows::tools::KeyHandler;
use windows::processor::Query;

/// Contains network information.
pub struct NetworkData {
    current_out: u64,
    current_in: u64,
    keys_in: Vec<KeyHandler>,
    keys_out: Vec<KeyHandler>,
}

impl NetworkExt for NetworkData {
    fn get_income(&self) -> u64 {
        self.current_in
    }

    fn get_outcome(&self) -> u64 {
        self.current_out
    }
}

pub fn new() -> NetworkData {
    NetworkData {
        current_in: 0,
        current_out: 0,
        keys_in: Vec::new(),
        keys_out: Vec::new(),
    }
}

pub fn refresh(network: &mut NetworkData, query: &Option<Query>) {
    if let &Some(ref query) = query {
        network.current_in = 0;
        for key in &network.keys_in {
            network.current_in += query.get_u64(&key.unique_id).expect("key disappeared");
        }
        network.current_out = 0;
        for key in &network.keys_out {
            network.current_out += query.get_u64(&key.unique_id).expect("key disappeared");
        }
    }
}

pub fn get_keys_in(network: &mut NetworkData) -> &mut Vec<KeyHandler> {
    &mut network.keys_in
}

pub fn get_keys_out(network: &mut NetworkData) -> &mut Vec<KeyHandler> {
    &mut network.keys_out
}
