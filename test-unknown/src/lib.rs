use sysinfo::System;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn tester() {
    if let Ok(mut s) = System::new() {
        s.refresh_all();
    }
}
