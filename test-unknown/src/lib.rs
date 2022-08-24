use sysinfo::{System, SystemExt};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn tester() {
    let mut s = System::new();
    s.refresh_all();
}
