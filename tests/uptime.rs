//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

#[test]
fn test_uptime() {
    use sysinfo::SystemExt;

    if sysinfo::System::IS_SUPPORTED {
        let mut s = sysinfo::System::new();
        s.refresh_all();
        assert!(s.get_uptime() != 0);
    }
}
