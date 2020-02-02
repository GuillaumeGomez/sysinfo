//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

extern crate sysinfo;

#[test]
fn test_process() {
    use sysinfo::SystemExt;
    #[cfg(not(windows))]
    use sysinfo::ProcessExt;

    let mut s = sysinfo::System::new();
    s.refresh_processes();
    assert!(s.get_process_list().len() != 0);
    #[cfg(not(windows))]
    assert!(s
        .get_process_list()
        .values()
        .any(|p| p.exe().to_str().unwrap_or_else(|| "").len() != 0));
}
