//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

extern crate sysinfo;

#[test]
fn test_process() {
    #[cfg(not(windows))]
    use sysinfo::ProcessExt;
    use sysinfo::SystemExt;

    let mut s = sysinfo::System::new();
    assert_eq!(s.get_processes().len(), 0);
    s.refresh_processes();
    assert!(s.get_processes().len() != 0);
    #[cfg(not(windows))]
    assert!(s
        .get_processes()
        .values()
        .any(|p| p.exe().to_str().unwrap_or_else(|| "").len() != 0));
}
