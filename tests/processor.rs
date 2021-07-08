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

    if sysinfo::System::IS_SUPPORTED {
        let s = sysinfo::System::new();
        assert!(!s.processors().is_empty());
        let s = sysinfo::System::new_all();
        assert!(!s.processors().is_empty());

        assert!(!s.processors()[0].brand().chars().any(|c| c == '\0'));
    }
}

#[test]
fn test_physical_core_numbers() {
    use sysinfo::SystemExt;

    if sysinfo::System::IS_SUPPORTED {
        let s = sysinfo::System::new();
        let count = s.physical_core_count();
        assert_ne!(count, None);
        assert!(count.unwrap() > 0);
    }
}
