//
// Sysinfo
//
// Copyright (c) 2017 Guillaume Gomez
//

#[test]
fn test_disks() {
    use sysinfo::SystemExt;

    if sysinfo::System::IS_SUPPORTED {
        let s = sysinfo::System::new_all();
        // If we don't have any physical core present, it's very likely that we're inside a VM...
        if s.physical_core_count().unwrap_or_default() > 0 {
            assert!(!s.disks().is_empty());
        }
    }
}
