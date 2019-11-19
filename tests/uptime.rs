//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

extern crate sysinfo;

#[test]
fn test_uptime() {
    use sysinfo::SystemExt;

    let mut s = sysinfo::System::new();
    s.refresh_all();
    assert!(s.get_uptime() != 0);
}
