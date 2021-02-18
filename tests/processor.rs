//
// Sysinfo
//
// Copyright (c) 2020 Guillaume Gomez
//

// This test is used to ensure that the processors are loaded whatever the method
// used to initialize `System`.

#[test]
fn test_processor() {
    use sysinfo::{ProcessorExt, SystemExt};

    let s = sysinfo::System::new();
    assert!(!s.get_processors().is_empty());
    let s = sysinfo::System::new_all();
    assert!(!s.get_processors().is_empty());

    assert!(s.get_processors()[0]
        .get_brand()
        .chars()
        .find(|c| *c == '\0')
        .is_none())
}

#[test]
fn test_physical_core_numbers() {
    use sysinfo::SystemExt;

    let s = sysinfo::System::new();
    let count = s.get_physical_core_count();
    assert_ne!(count, None);
    assert!(count.unwrap() > 0);
}
