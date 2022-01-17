// Take a look at the license at the top of the repository in the LICENSE file.

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

#[test]
fn test_system_disk_usage() {
    use std::fs;
    use std::fs::File;
    use std::io::prelude::*;
    use sysinfo::{DiskExt, SystemExt};

    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }
    if std::env::var("FREEBSD_CI").is_ok() {
        // For an unknown reason, when running this test on Cirrus CI, it fails. It works perfectly
        // locally though... Dark magic...
        return;
    }

    let mut system = sysinfo::System::new();
    system.refresh_disks_list();
    system.refresh_disks_usage();

    {
        let mut file = File::create("test.txt").expect("failed to create file");
        file.write_all(b"This is a test file\nwith test data.\n")
            .expect("failed to write to file");
    }
    fs::remove_file("test.txt").expect("failed to remove file");
    // Waiting a bit just in case...
    std::thread::sleep(std::time::Duration::from_millis(5000));

    system.refresh_disks_usage();

    let mut total_written_since = 0;
    let mut total_written = 0;
    for disk in system.disks() {
        total_written_since += disk.usage().written_bytes;
        total_written += disk.usage().total_written_bytes;
    }
    assert!(
        total_written > 0,
        "found {} total written bytes...",
        total_written
    );
    assert!(
        total_written_since > 0,
        "found {} written bytes...",
        total_written_since
    );
}
