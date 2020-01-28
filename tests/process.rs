//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

extern crate sysinfo;

use sysinfo::ProcessExt;
use sysinfo::SystemExt;

#[test]
fn test_process() {
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

#[test]
fn test_process_refresh() {
    let mut s = sysinfo::System::new();
    assert_eq!(s.get_processes().len(), 0);
    s.refresh_process(sysinfo::get_current_pid().expect("failed to get current pid"));
    assert_eq!(
        s.get_process(sysinfo::get_current_pid().expect("failed to get current pid"))
            .is_some(),
        true
    );
}

#[test]
#[cfg(windows)]
fn test_get_cmd_line() {
    let p = std::process::Command::new("timeout")
        .arg("/t")
        .arg("3")
        .spawn()
        .unwrap();
    let mut s = sysinfo::System::new();
    s.refresh_processes();
    let process = s.get_process(p.id() as sysinfo::Pid).unwrap();
    assert_eq!(process.cmd(), &["timeout", "/t", "3"]);
}

#[test]
#[cfg(not(windows))]
fn test_get_cmd_line() {
    let p = std::process::Command::new("sleep")
        .arg("3")
        .spawn()
        .unwrap();
    let mut s = sysinfo::System::new();
    s.refresh_processes();
    let process = s.get_process(p.id() as sysinfo::Pid).unwrap();
    assert_eq!(process.cmd(), &["sleep", "3"]);
}

#[test]
fn test_process_disk_usage() {
    use sysinfo::{ProcessExt, SystemExt};
    use std::fs::File;
    use std::fs;
    use std::io::prelude::*;
    {
        let mut file = File::create("test.txt").unwrap();
        file.write_all(b"This is a test file\nwith test data.\n").unwrap();
    }
    fs::remove_file("test.txt").ok();
    let mut system = sysinfo::System::new();
    system.refresh_processes();
    let process_list = system.get_process_list();
    let mut write_bytes: u64 = 0;
    for p in process_list.values(){
        write_bytes += p.written_bytes();
    }

    assert!(write_bytes > 0);
}
