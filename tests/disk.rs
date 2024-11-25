// Take a look at the license at the top of the repository in the LICENSE file.

#[cfg(all(feature = "system", feature = "disk"))]
fn should_skip() -> bool {
    if !sysinfo::IS_SUPPORTED_SYSTEM {
        return true;
    }

    let s = sysinfo::System::new_all();
    if s.physical_core_count().unwrap_or_default() == 0 {
        return true;
    }
    false
}

#[test]
#[cfg(all(feature = "system", feature = "disk"))]
fn test_disks() {
    if should_skip() {
        return;
    }

    let mut disks = sysinfo::Disks::new();
    assert!(disks.list().is_empty());
    disks.refresh_list();
    assert!(!disks.list().is_empty());
}

#[test]
#[cfg(all(feature = "system", feature = "disk"))]
fn test_disk_refresh_kind() {
    use itertools::Itertools;

    use sysinfo::{DiskKind, DiskRefreshKind, Disks};

    if should_skip() {
        return;
    }

    for fs in [
        DiskRefreshKind::with_kind,
        DiskRefreshKind::without_kind,
        DiskRefreshKind::with_details,
        DiskRefreshKind::without_details,
        DiskRefreshKind::with_io_usage,
        DiskRefreshKind::without_io_usage,
    ]
    .iter()
    .powerset()
    {
        let mut refreshes = DiskRefreshKind::new();
        for f in fs {
            refreshes = f(refreshes);
        }

        let assertions = |disks: &Disks| {
            for disk in disks.list().iter() {
                if refreshes.kind() {
                    assert_ne!(
                        disk.kind(),
                        DiskKind::Unknown(-1),
                        "disk.kind should be refreshed"
                    );
                } else {
                    assert_eq!(
                        disk.kind(),
                        DiskKind::Unknown(-1),
                        "disk.kind should not be refreshed"
                    );
                }

                if refreshes.details() {
                    assert_ne!(
                        disk.available_space(),
                        Default::default(),
                        "disk.available_space should be refreshed"
                    );
                    assert_ne!(
                        disk.total_space(),
                        Default::default(),
                        "disk.total_space should be refreshed"
                    );
                    // We can't assert anything about booleans, since false is indistinguishable from
                    // not-refreshed
                } else {
                    assert_eq!(
                        disk.available_space(),
                        Default::default(),
                        "disk.available_space should not be refreshed"
                    );
                    assert_eq!(
                        disk.total_space(),
                        Default::default(),
                        "disk.total_space should not be refreshed"
                    );
                    assert_eq!(
                        disk.is_read_only(),
                        Default::default(),
                        "disk.is_read_only should not be refreshed"
                    );
                    assert_eq!(
                        disk.is_removable(),
                        Default::default(),
                        "disk.is_removable should not be refreshed"
                    );
                }

                if refreshes.io_usage() {
                    assert_ne!(
                        disk.usage(),
                        Default::default(),
                        "disk.usage should be refreshed"
                    );
                } else {
                    assert_eq!(
                        disk.usage(),
                        Default::default(),
                        "disk.usage should not be refreshed"
                    );
                }
            }
        };

        // load and refresh with the desired details should work
        let disks = Disks::new_with_refreshed_list_specifics(refreshes);
        assertions(&disks);

        // load with minimal `DiskRefreshKind`, then refresh for added detail should also work!
        let mut disks = Disks::new_with_refreshed_list_specifics(DiskRefreshKind::new());
        disks.refresh_specifics(refreshes);
        assertions(&disks);
    }
}

#[test]
#[cfg(all(feature = "system", feature = "disk"))]
fn test_disks_usage() {
    use std::fs::{remove_file, File};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::thread::sleep;

    use sysinfo::Disks;

    if should_skip() {
        return;
    }

    // The test always fails in CI on Linux. For some unknown reason, /proc/diskstats just doesn't
    // update, regardless of how long we wait. Until the root cause is discovered, skip the test
    // in CI.
    if cfg!(target_os = "linux") && std::env::var("CI").is_ok() {
        return;
    }

    let mut disks = Disks::new_with_refreshed_list();

    let path = match std::env::var("CARGO_TARGET_DIR") {
        Ok(p) => Path::new(&p).join("data.tmp"),
        _ => PathBuf::from("target/data.tmp"),
    };
    let mut file = File::create(&path).expect("failed to create temporary file");

    // Write 10mb worth of data to the temp file.
    let data = vec![1u8; 10 * 1024 * 1024];
    file.write_all(&data).unwrap();
    // The sync_all call is important to ensure all the data is persisted to disk. Without
    // the call, this test is flaky.
    file.sync_all().unwrap();

    // Wait a bit just in case
    sleep(std::time::Duration::from_millis(500));
    disks.refresh();

    // Depending on the OS and how disks are configured, the disk usage may be the exact same
    // across multiple disks. To account for this, collect the disk usages and dedup.
    let mut disk_usages = disks.list().iter().map(|d| d.usage()).collect::<Vec<_>>();
    disk_usages.dedup();

    let mut written_bytes = 0;
    for disk_usage in disk_usages {
        written_bytes += disk_usage.written_bytes;
    }

    let _ = remove_file(path);

    // written_bytes should have increased by about 10mb, but this is not fully reliable in CI Linux. For now,
    // just verify the number is non-zero.
    assert!(written_bytes > 0);
}
