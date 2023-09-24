// Take a look at the license at the top of the repository in the LICENSE file.

// This test is used to ensure that the CPUs are not loaded by default.

#[test]
fn test_cpu() {
    use sysinfo::{CpuExt, SystemExt};

    let mut s = sysinfo::System::new();
    assert!(s.cpus().is_empty());

    if !sysinfo::System::IS_SUPPORTED {
        return;
    }

    s.refresh_cpu();
    assert!(!s.cpus().is_empty());

    let s = sysinfo::System::new_all();
    assert!(!s.cpus().is_empty());

    assert!(!s.cpus()[0].brand().chars().any(|c| c == '\0'));

    if !cfg!(target_os = "freebsd") {
        // This information is currently not retrieved on freebsd...
        assert!(s.cpus().iter().any(|c| !c.brand().is_empty()));
    }
    assert!(s.cpus().iter().any(|c| !c.vendor_id().is_empty()));
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

#[test]
fn test_global_cpu_info_not_set() {
    use sysinfo::{CpuExt, SystemExt};

    let mut s = sysinfo::System::new();
    assert_eq!(s.global_cpu_info().vendor_id(), "");
    assert_eq!(s.global_cpu_info().brand(), "");
    assert_eq!(s.global_cpu_info().frequency(), 0);
    s.refresh_cpu();
    assert_eq!(s.global_cpu_info().vendor_id(), "");
    assert_eq!(s.global_cpu_info().brand(), "");
    assert_eq!(s.global_cpu_info().frequency(), 0);
}
