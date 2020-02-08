//
// Sysinfo
//
// Copyright (c) 2020 Guillaume Gomez
//

// This test is used to ensure that the processors are loaded whatever the method
// used to initialize `System`.

extern crate sysinfo;

#[test]
fn test_processor() {
    use sysinfo::{NetworksExt, SystemExt};

    let s = sysinfo::System::new();
    assert_eq!(s.get_networks().iter().count(), 0);
    let s = sysinfo::System::new_all();
    assert!(s.get_networks().iter().count() > 0);
}
