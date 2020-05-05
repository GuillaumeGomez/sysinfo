//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

extern crate sysinfo;

#[test]
fn test_disks() {
    use sysinfo::SystemExt;

    let s = sysinfo::System::new_all();
    assert!(!s.get_disks().is_empty());
}
