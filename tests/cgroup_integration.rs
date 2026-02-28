// Take a look at the license at the top of the repository in the LICENSE file.

#![cfg(feature = "system")]
use sysinfo::System;

#[test]
#[ignore]
fn test_cgroup_limits_with_memory_constraint() {
    // This test should be run via:
    // systemd-run --user --scope -p MemoryMax=512M cargo test --test cgroup_integration -- --ignored --nocapture
    let mut sys = System::new();
    sys.refresh_memory();

    if let Some(limits) = sys.cgroup_limits() {
        // Check if we're in a constrained cgroup (512MB)
        let expected = 512 * 1024 * 1024;

        println!("CGroup limits detected: {:?}", limits);
        assert!(
            limits.total_memory <= expected,
            "Expected 512MB limit, got {} bytes",
            limits.total_memory
        );
    } else {
        panic!("cgroup_limits() returned None - cgroup detection failed");
    }
}
