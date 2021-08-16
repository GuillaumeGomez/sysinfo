//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

use sysinfo::ProcessExt;
use sysinfo::SystemExt;

#[test]
fn test_process() {
    let mut s = sysinfo::System::new();
    assert_eq!(s.processes().len(), 0);
    s.refresh_processes();
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }
    assert!(!s.processes().is_empty());
    #[cfg(not(windows))]
    assert!(s
        .processes()
        .values()
        .any(|p| !p.exe().to_str().unwrap_or("").is_empty()));
}

#[test]
fn test_cwd() {
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }
    let mut p = if cfg!(target_os = "windows") {
        std::process::Command::new("waitfor")
            .arg("/t")
            .arg("3")
            .arg("CwdSignal")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    } else {
        std::process::Command::new("sleep")
            .arg("3")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };

    let pid = p.id() as sysinfo::Pid;
    std::thread::sleep(std::time::Duration::from_millis(250));
    let mut s = sysinfo::System::new();
    s.refresh_processes();
    p.kill().expect("Unable to kill process.");

    let processes = s.processes();
    let p = processes.get(&pid);

    if let Some(p) = p {
        assert_eq!(p.pid(), pid);
        assert_eq!(p.cwd(), std::env::current_dir().unwrap());
    } else {
        panic!("Process not found!");
    }
}

#[test]
fn test_cmd() {
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }
    let mut p = if cfg!(target_os = "windows") {
        std::process::Command::new("waitfor")
            .arg("/t")
            .arg("3")
            .arg("CmdSignal")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    } else {
        std::process::Command::new("sleep")
            .arg("3")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };
    std::thread::sleep(std::time::Duration::from_millis(250));
    let mut s = sysinfo::System::new();
    assert!(s.processes().is_empty());
    s.refresh_processes();
    p.kill().expect("Unable to kill process.");
    assert!(!s.processes().is_empty());
    if let Some(process) = s.process(p.id() as sysinfo::Pid) {
        if cfg!(target_os = "windows") {
            assert_eq!(process.cmd(), &["waitfor", "/t", "3", "CmdSignal"]);
        } else {
            assert_eq!(process.cmd(), &["sleep", "3"]);
        }
    } else {
        panic!("Process not found!");
    }
}

#[test]
fn test_environ() {
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }
    let mut p = if cfg!(target_os = "windows") {
        std::process::Command::new("waitfor")
            .arg("/t")
            .arg("3")
            .arg("EnvironSignal")
            .stdout(std::process::Stdio::null())
            .env("FOO", "BAR")
            .env("OTHER", "VALUE")
            .spawn()
            .unwrap()
    } else {
        std::process::Command::new("sleep")
            .arg("3")
            .stdout(std::process::Stdio::null())
            .env("FOO", "BAR")
            .env("OTHER", "VALUE")
            .spawn()
            .unwrap()
    };

    let pid = p.id() as sysinfo::Pid;
    std::thread::sleep(std::time::Duration::from_millis(250));
    let mut s = sysinfo::System::new();
    s.refresh_processes();
    p.kill().expect("Unable to kill process.");

    let processes = s.processes();
    let p = processes.get(&pid);

    if let Some(p) = p {
        assert_eq!(p.pid(), pid);
        assert!(p.environ().iter().any(|e| e == "FOO=BAR"));
        assert!(p.environ().iter().any(|e| e == "OTHER=VALUE"));
    } else {
        panic!("Process not found!");
    }
}

#[test]
fn test_process_refresh() {
    let mut s = sysinfo::System::new();
    assert_eq!(s.processes().len(), 0);

    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }
    s.refresh_process(sysinfo::get_current_pid().expect("failed to get current pid"));
    assert!(s
        .process(sysinfo::get_current_pid().expect("failed to get current pid"))
        .is_some(),);
}

#[test]
fn test_process_disk_usage() {
    use std::fs;
    use std::fs::File;
    use std::io::prelude::*;
    use sysinfo::{get_current_pid, ProcessExt, SystemExt};

    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }

    {
        let mut file = File::create("test.txt").unwrap();
        file.write_all(b"This is a test file\nwith test data.\n")
            .unwrap();
    }
    fs::remove_file("test.txt").ok();
    let mut system = sysinfo::System::new();
    assert!(system.processes().is_empty());
    system.refresh_processes();
    assert!(!system.processes().is_empty());
    let p = system
        .process(get_current_pid().expect("Failed retrieving current pid."))
        .expect("failed to get process");

    assert!(
        p.disk_usage().total_written_bytes > 0,
        "found {} total written bytes...",
        p.disk_usage().total_written_bytes
    );
    assert!(
        p.disk_usage().written_bytes > 0,
        "found {} written bytes...",
        p.disk_usage().written_bytes
    );
}

#[test]
fn cpu_usage_is_not_nan() {
    let mut system = sysinfo::System::new();
    system.refresh_processes();

    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }

    let first_pids = system
        .processes()
        .iter()
        .take(10)
        .map(|(&pid, _)| pid)
        .collect::<Vec<_>>();
    let mut checked = 0;

    first_pids.into_iter().for_each(|pid| {
        system.refresh_process(pid);
        if let Some(p) = system.process(pid) {
            assert!(!p.cpu_usage().is_nan());
            checked += 1;
        }
    });
    assert!(checked > 0);
}
