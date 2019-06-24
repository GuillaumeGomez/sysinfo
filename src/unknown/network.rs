// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

use NetworkExt;

/// Contains network information.
#[derive(Debug)]
pub struct NetworkData;

impl NetworkExt for NetworkData {
    fn get_income(&self) -> u64 {
        0
    }

    fn get_outcome(&self) -> u64 {
        0
    }
}
