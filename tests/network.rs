// Take a look at the license at the top of the repository in the LICENSE file.

// This test is used to ensure that the processors are loaded whatever the method
// used to initialize `System`.

#[test]
fn test_processor() {
    use sysinfo::{NetworksExt, SystemExt};

    if sysinfo::System::IS_SUPPORTED {
        let s = sysinfo::System::new();
        assert_eq!(s.networks().iter().count(), 0);
        let s = sysinfo::System::new_all();
        assert!(s.networks().iter().count() > 0);
    }
}
