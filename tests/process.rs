//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

extern crate sysinfo;

#[test]
fn test_process() {
    use sysinfo::{ProcessExt, SystemExt};

    let mut s = sysinfo::System::new();
    s.refresh_processes();
    assert!(s.get_process_list().len() != 0);
    #[cfg(not(windows))]
    assert!(s
        .get_process_list()
        .values()
        .any(|p| p.exe().to_str().unwrap_or_else(|| "").len() != 0));
}

#[test]
fn test_process_disk_usage(){
    use sysinfo::{ProcessExt, SystemExt, get_current_pid};
    use std::fs::File;
    use std::fs;
    use std::io::prelude::*;
    {
        let mut file = File::create("test.txt").unwrap();
        file.write_all(b"This is a test file\nwith test data.\n").unwrap();
    }
    fs::remove_file("test.txt").ok();
    let system = sysinfo::System::new();
    let p = system.get_process(get_current_pid().expect("Failed retrieving current pid.")).expect("failed to get process");

    assert!(p.written_bytes() > 0);
}