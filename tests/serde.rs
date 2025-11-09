// Take a look at the license at the top of the repository in the LICENSE file.

#![cfg(feature = "serde")]
#![allow(clippy::assertions_on_constants)]

use sysinfo::{MacAddr, ProcessRefreshKind, ProcessesToUpdate, System};

#[test]
fn test_serde_process_name() {
    if !sysinfo::IS_SUPPORTED_SYSTEM {
        return;
    }
    let mut s = System::new();
    s.refresh_processes_specifics(ProcessesToUpdate::All, false, ProcessRefreshKind::nothing());

    if s.processes().is_empty() {
        panic!("no processes?");
    }

    for p in s.processes().values() {
        let values = match serde_json::to_value(p) {
            Ok(serde_json::Value::Object(values)) => values,
            other => panic!("expected object, found `{other:?}`"),
        };
        match values.get("name") {
            Some(serde_json::Value::String(_)) => {}
            value => panic!("expected a string, found `{value:?}`"),
        }
    }
}

#[test]
fn test_serde_mac_address() {
    let m = MacAddr([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC]);

    let value = match serde_json::to_value(m) {
        Ok(serde_json::Value::String(value)) => value,
        other => panic!("expected string, found `{other:?}`"),
    };
    assert_eq!(value, "12:34:56:78:9a:bc");
}
