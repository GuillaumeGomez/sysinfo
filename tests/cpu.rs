// Take a look at the license at the top of the repository in the LICENSE file.

// This test is used to ensure that the CPUs are not loaded by default.

#[test]
fn test_cpu() {
    use sysinfo::{CpuExt, SystemExt};

    if sysinfo::System::IS_SUPPORTED {
        let mut s = sysinfo::System::new();
        assert!(s.cpus().is_empty());
        s.refresh_cpu();
        assert!(!s.cpus().is_empty());

        let s = sysinfo::System::new_all();
        assert!(!s.cpus().is_empty());

        assert!(!s.cpus()[0].brand().chars().any(|c| c == '\0'));
    }
}

#[test]
fn test_physical_core_numbers() {
    use sysinfo::SystemExt;

    if sysinfo::System::IS_SUPPORTED {
        let s = sysinfo::System::new();
        let count = s.physical_core_count();
        assert_ne!(count, None);
        assert!(count.unwrap() > 0);
    }
}
