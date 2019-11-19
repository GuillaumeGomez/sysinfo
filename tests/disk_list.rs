//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

extern crate sysinfo;

#[test]
fn test_disks() {
    use sysinfo::SystemExt;

    let s = sysinfo::System::new();
    println!("total memory: {}", s.get_total_memory());
    println!("total cpu cores: {}", s.get_processor_list().len());
}
