// Take a look at the license at the top of the repository in the LICENSE file.

// This test is used to ensure that the networks are not loaded by default.

#[test]
fn test_networks() {
    use sysinfo::{NetworksExt, SystemExt};

    if sysinfo::System::IS_SUPPORTED {
        let s = sysinfo::System::new();
        assert_eq!(s.networks().iter().count(), 0);
        let s = sysinfo::System::new_all();
        assert!(s.networks().iter().count() > 0);
    }
}
