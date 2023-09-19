// Take a look at the license at the top of the repository in the LICENSE file.

// This test is used to ensure that the networks are not loaded by default.

#[test]
fn test_networks() {
    use sysinfo::{Networks, NetworksExt, SystemExt};

    if sysinfo::System::IS_SUPPORTED {
        let mut n = Networks::new();
        assert_eq!(n.iter().count(), 0);
        n.refresh();
        assert_eq!(n.iter().count(), 0);
        n.refresh_list();
        assert!(n.iter().count() > 0);
    }
}
