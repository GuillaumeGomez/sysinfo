// Take a look at the license at the top of the repository in the LICENSE file.

#![doc = include_str!("../README.md")]
#![cfg_attr(feature = "serde", doc = include_str!("../md_doc/serde.md"))]
#![allow(unknown_lints)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::non_send_fields_in_send_ty)]
#![allow(renamed_and_removed_lints)]
#![allow(clippy::assertions_on_constants)]

#[macro_use]
mod macros;

cfg_if::cfg_if! {
    if #[cfg(feature = "unknown-ci")] {
        // This is used in CI to check that the build for unknown targets is compiling fine.
        mod unknown;
        use crate::unknown as sys;

        #[cfg(test)]
        pub(crate) const MIN_USERS: usize = 0;
    } else if #[cfg(any(
        target_os = "macos", target_os = "ios",
        target_os = "linux", target_os = "android",
        target_os = "freebsd"))]
    {
        mod unix;
        mod network;
        use crate::unix::sys as sys;
        use crate::unix::network_helper;

        #[cfg(test)]
        pub(crate) const MIN_USERS: usize = 1;
    } else if #[cfg(windows)] {
        mod windows;
        use crate::windows as sys;
        use crate::windows::network_helper;
        mod network;

        #[cfg(test)]
        pub(crate) const MIN_USERS: usize = 1;
    } else {
        mod unknown;
        use crate::unknown as sys;

        #[cfg(test)]
        pub(crate) const MIN_USERS: usize = 0;
    }
}

pub use crate::common::{
    get_current_pid, CGroupLimits, Component, Components, Cpu, CpuRefreshKind, Disk, DiskKind,
    DiskUsage, Disks, Gid, Group, Groups, LoadAvg, MacAddr, MemoryRefreshKind, NetworkData,
    Networks, Pid, Process, ProcessRefreshKind, ProcessStatus, RefreshKind, Signal, System,
    ThreadKind, Uid, UpdateKind, User, Users,
};

pub(crate) use crate::common::GroupInner;
pub(crate) use crate::sys::{
    ComponentInner, ComponentsInner, CpuInner, DiskInner, DisksInner, NetworkDataInner,
    NetworksInner, ProcessInner, SystemInner, UserInner,
};
pub use crate::sys::{IS_SUPPORTED_SYSTEM, MINIMUM_CPU_UPDATE_INTERVAL, SUPPORTED_SIGNALS};

#[cfg(feature = "c-interface")]
pub use crate::c_interface::*;

#[cfg(feature = "c-interface")]
mod c_interface;
mod common;
mod debug;
#[cfg(feature = "serde")]
mod serde;
mod utils;

/// This function is only used on Linux targets, on the other platforms it does nothing and returns
/// `false`.
///
/// On Linux, to improve performance, we keep a `/proc` file open for each process we index with
/// a maximum number of files open equivalent to half of the system limit.
///
/// The problem is that some users might need all the available file descriptors so we need to
/// allow them to change this limit.
///
/// Note that if you set a limit bigger than the system limit, the system limit will be set.
///
/// Returns `true` if the new value has been set.
///
/// ```no_run
/// use sysinfo::{System, set_open_files_limit};
///
/// // We call the function before any call to the processes update.
/// if !set_open_files_limit(10) {
///     // It'll always return false on non-linux targets.
///     eprintln!("failed to update the open files limit...");
/// }
/// let s = System::new_all();
/// ```
pub fn set_open_files_limit(mut _new_limit: isize) -> bool {
    cfg_if::cfg_if! {
        if #[cfg(all(not(feature = "unknown-ci"), any(target_os = "linux", target_os = "android")))]
        {
            use crate::sys::system::REMAINING_FILES;
            use std::sync::atomic::Ordering;

            if _new_limit < 0 {
                _new_limit = 0;
            }
            let max = sys::system::get_max_nb_fds();
            if _new_limit > max {
                _new_limit = max;
            }

            // If files are already open, to be sure that the number won't be bigger when those
            // files are closed, we subtract the current number of opened files to the new
            // limit.
            REMAINING_FILES.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
                let diff = max.saturating_sub(remaining);
                Some(_new_limit.saturating_sub(diff))
            }).unwrap();

            true

        } else {
            false
        }
    }
}

// FIXME: Can be removed once negative trait bounds are supported.
#[cfg(doctest)]
mod doctest {
    /// Check that `Process` doesn't implement `Clone`.
    ///
    /// First we check that the "basic" code works:
    ///
    /// ```no_run
    /// use sysinfo::{Process, System};
    ///
    /// let mut s = System::new_all();
    /// let p: &Process = s.processes().values().next().unwrap();
    /// ```
    ///
    /// And now we check if it fails when we try to clone it:
    ///
    /// ```compile_fail
    /// use sysinfo::{Process, System};
    ///
    /// let mut s = System::new_all();
    /// let p: &Process = s.processes().values().next().unwrap();
    /// let p = (*p).clone();
    /// ```
    mod process_clone {}

    /// Check that `System` doesn't implement `Clone`.
    ///
    /// First we check that the "basic" code works:
    ///
    /// ```no_run
    /// use sysinfo::{Process, System};
    ///
    /// let s = System::new();
    /// ```
    ///
    /// And now we check if it fails when we try to clone it:
    ///
    /// ```compile_fail
    /// use sysinfo::{Process, System};
    ///
    /// let s = System::new();
    /// let s = s.clone();
    /// ```
    mod system_clone {}
}

#[cfg(test)]
mod test {
    use crate::*;

    #[cfg(feature = "unknown-ci")]
    #[test]
    fn check_unknown_ci_feature() {
        assert!(!IS_SUPPORTED_SYSTEM);
    }

    // If this test doesn't compile, it means the current OS doesn't implement them correctly.
    #[test]
    fn check_macro_types() {
        fn check_is_supported(_: bool) {}
        fn check_supported_signals(_: &'static [Signal]) {}
        fn check_minimum_cpu_update_interval(_: std::time::Duration) {}

        check_is_supported(IS_SUPPORTED_SYSTEM);
        check_supported_signals(SUPPORTED_SIGNALS);
        check_minimum_cpu_update_interval(MINIMUM_CPU_UPDATE_INTERVAL);
    }

    #[test]
    fn check_process_memory_usage() {
        let mut s = System::new();
        s.refresh_specifics(RefreshKind::everything());

        if IS_SUPPORTED_SYSTEM {
            // No process should have 0 as memory usage.
            #[cfg(not(feature = "apple-sandbox"))]
            assert!(!s.processes().iter().all(|(_, proc_)| proc_.memory() == 0));
        } else {
            // There should be no process, but if there is one, its memory usage should be 0.
            assert!(s.processes().iter().all(|(_, proc_)| proc_.memory() == 0));
        }
    }

    #[test]
    fn check_system_implemented_traits() {
        fn check<T: Sized + std::fmt::Debug + Default + Send + Sync>(_: T) {}

        check(System::new());
    }

    #[test]
    fn check_memory_usage() {
        let mut s = System::new();

        assert_eq!(s.total_memory(), 0);
        assert_eq!(s.free_memory(), 0);
        assert_eq!(s.available_memory(), 0);
        assert_eq!(s.used_memory(), 0);
        assert_eq!(s.total_swap(), 0);
        assert_eq!(s.free_swap(), 0);
        assert_eq!(s.used_swap(), 0);

        s.refresh_memory();
        if IS_SUPPORTED_SYSTEM {
            assert!(s.total_memory() > 0);
            assert!(s.used_memory() > 0);
            if s.total_swap() > 0 {
                // I think it's pretty safe to assume that there is still some swap left...
                assert!(s.free_swap() > 0);
            }
        } else {
            assert_eq!(s.total_memory(), 0);
            assert_eq!(s.used_memory(), 0);
            assert_eq!(s.total_swap(), 0);
            assert_eq!(s.free_swap(), 0);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn check_processes_cpu_usage() {
        if !IS_SUPPORTED_SYSTEM {
            return;
        }
        let mut s = System::new();

        s.refresh_processes();
        // All CPU usage will start at zero until the second refresh
        assert!(s
            .processes()
            .iter()
            .all(|(_, proc_)| proc_.cpu_usage() == 0.0));

        // Wait a bit to update CPU usage values
        std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
        s.refresh_processes();
        assert!(s
            .processes()
            .iter()
            .all(|(_, proc_)| proc_.cpu_usage() >= 0.0
                && proc_.cpu_usage() <= (s.cpus().len() as f32) * 100.0));
        assert!(s
            .processes()
            .iter()
            .any(|(_, proc_)| proc_.cpu_usage() > 0.0));
    }

    #[test]
    fn check_cpu_usage() {
        if !IS_SUPPORTED_SYSTEM {
            return;
        }
        let mut s = System::new();
        for _ in 0..10 {
            s.refresh_cpu_usage();
            // Wait a bit to update CPU usage values
            std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
            if s.cpus().iter().any(|c| c.cpu_usage() > 0.0) {
                // All good!
                return;
            }
        }
        panic!("CPU usage is always zero...");
    }

    #[test]
    fn check_list() {
        let mut users = Users::new();
        assert!(users.list().is_empty());
        users.refresh_list();
        assert!(users.list().len() >= MIN_USERS);
    }

    #[test]
    fn check_uid_gid() {
        let mut users = Users::new();
        assert!(users.list().is_empty());
        users.refresh_list();
        let user_list = users.list();
        assert!(user_list.len() >= MIN_USERS);

        if IS_SUPPORTED_SYSTEM {
            #[cfg(not(target_os = "windows"))]
            {
                let user = user_list
                    .iter()
                    .find(|u| u.name() == "root")
                    .expect("no root user");
                assert_eq!(**user.id(), 0);
                assert_eq!(*user.group_id(), 0);
                if let Some(user) = users.iter().find(|u| *u.group_id() > 0) {
                    assert!(**user.id() > 0);
                    assert!(*user.group_id() > 0);
                }
                assert!(user_list.iter().filter(|u| **u.id() > 0).count() > 0);
            }

            // And now check that our `get_user_by_id` method works.
            let s = System::new_with_specifics(
                RefreshKind::new()
                    .with_processes(ProcessRefreshKind::new().with_user(UpdateKind::Always)),
            );
            assert!(s
                .processes()
                .iter()
                .filter_map(|(_, p)| p.user_id())
                .any(|uid| users.get_user_by_id(uid).is_some()));
        }
    }

    #[test]
    fn check_all_process_uids_resolvable() {
        // On linux, some user IDs don't have an associated user (no idea why though).
        // If `getent` doesn't find them, we can assume it's a dark secret from the linux land.
        if IS_SUPPORTED_SYSTEM && cfg!(not(target_os = "linux")) {
            let s = System::new_with_specifics(
                RefreshKind::new()
                    .with_processes(ProcessRefreshKind::new().with_user(UpdateKind::Always)),
            );
            let users = Users::new_with_refreshed_list();

            // For every process where we can get a user ID, we should also be able
            // to find that user ID in the global user list
            for process in s.processes().values() {
                if let Some(uid) = process.user_id() {
                    assert!(
                        users.get_user_by_id(uid).is_some(),
                        "No UID {:?} found",
                        uid
                    );
                }
            }
        }
    }

    #[test]
    fn check_system_info() {
        // We don't want to test on unsupported systems.
        if IS_SUPPORTED_SYSTEM {
            assert!(!System::name()
                .expect("Failed to get system name")
                .is_empty());

            assert!(!System::kernel_version()
                .expect("Failed to get kernel version")
                .is_empty());

            assert!(!System::os_version()
                .expect("Failed to get os version")
                .is_empty());

            assert!(!System::long_os_version()
                .expect("Failed to get long OS version")
                .is_empty());
        }

        assert!(!System::distribution_id().is_empty());
    }

    #[test]
    fn check_host_name() {
        // We don't want to test on unsupported systems.
        if IS_SUPPORTED_SYSTEM {
            assert!(System::host_name().is_some());
        }
    }

    #[test]
    fn check_refresh_process_return_value() {
        // We don't want to test on unsupported systems.
        if IS_SUPPORTED_SYSTEM {
            let _pid = get_current_pid().expect("Failed to get current PID");

            #[cfg(not(feature = "apple-sandbox"))]
            {
                let mut s = System::new();
                // First check what happens in case the process isn't already in our process list.
                assert!(s.refresh_process(_pid));
                // Then check that it still returns true if the process is already in our process list.
                assert!(s.refresh_process(_pid));
            }
        }
    }

    #[test]
    fn ensure_is_supported_is_set_correctly() {
        if MIN_USERS > 0 {
            assert!(IS_SUPPORTED_SYSTEM);
        } else {
            assert!(!IS_SUPPORTED_SYSTEM);
        }
    }

    #[test]
    fn check_cpus_number() {
        let mut s = System::new();

        // This information isn't retrieved by default.
        assert!(s.cpus().is_empty());
        if IS_SUPPORTED_SYSTEM {
            // The physical cores count is recomputed every time the function is called, so the
            // information must be relevant even with nothing initialized.
            let physical_cores_count = s
                .physical_core_count()
                .expect("failed to get number of physical cores");

            s.refresh_cpu_usage();
            // The cpus shouldn't be empty anymore.
            assert!(!s.cpus().is_empty());

            // In case we are running inside a VM, it's possible to not have a physical core, only
            // logical ones, which is why we don't test `physical_cores_count > 0`.
            let physical_cores_count2 = s
                .physical_core_count()
                .expect("failed to get number of physical cores");
            assert!(physical_cores_count2 <= s.cpus().len());
            assert_eq!(physical_cores_count, physical_cores_count2);
        } else {
            assert_eq!(s.physical_core_count(), None);
        }
        assert!(s.physical_core_count().unwrap_or(0) <= s.cpus().len());
    }

    #[test]
    fn check_nb_supported_signals() {
        if IS_SUPPORTED_SYSTEM {
            assert!(
                !SUPPORTED_SIGNALS.is_empty(),
                "SUPPORTED_SIGNALS shouldn't be empty on supported systems!"
            );
        } else {
            assert!(
                SUPPORTED_SIGNALS.is_empty(),
                "SUPPORTED_SIGNALS should be empty on not support systems!"
            );
        }
    }

    // Ensure that the CPUs frequency isn't retrieved until we ask for it.
    #[test]
    fn check_cpu_frequency() {
        if !IS_SUPPORTED_SYSTEM {
            return;
        }
        let mut s = System::new();
        s.refresh_processes();
        for proc_ in s.cpus() {
            assert_eq!(proc_.frequency(), 0);
        }
        s.refresh_cpu_usage();
        for proc_ in s.cpus() {
            assert_eq!(proc_.frequency(), 0);
        }
        // In a VM, it'll fail.
        if std::env::var("APPLE_CI").is_err() && std::env::var("FREEBSD_CI").is_err() {
            s.refresh_cpu_specifics(CpuRefreshKind::everything());
            for proc_ in s.cpus() {
                assert_ne!(proc_.frequency(), 0);
            }
        }
    }

    // In case `Process::updated` is misused, `System::refresh_processes` might remove them
    // so this test ensures that it doesn't happen.
    #[test]
    fn check_refresh_process_update() {
        if !IS_SUPPORTED_SYSTEM {
            return;
        }
        let mut s = System::new_all();
        let total = s.processes().len() as isize;
        s.refresh_processes();
        let new_total = s.processes().len() as isize;
        // There should be almost no difference in the processes count.
        assert!(
            (new_total - total).abs() <= 5,
            "{} <= 5",
            (new_total - total).abs()
        );
    }

    #[test]
    fn check_cpu_arch() {
        assert_eq!(System::cpu_arch().is_some(), IS_SUPPORTED_SYSTEM);
    }

    // This test only exists to ensure that the `Display` and `Debug` traits are implemented on the
    // `ProcessStatus` enum on all targets.
    #[test]
    fn check_display_impl_process_status() {
        println!("{} {:?}", ProcessStatus::Parked, ProcessStatus::Idle);
    }

    // Ensure that the `Display` and `Debug` traits are implemented on the `MacAddr` struct
    #[test]
    fn check_display_impl_mac_address() {
        println!(
            "{} {:?}",
            MacAddr([0x1, 0x2, 0x3, 0x4, 0x5, 0x6]),
            MacAddr([0xa, 0xb, 0xc, 0xd, 0xe, 0xf])
        );
    }

    #[test]
    fn check_mac_address_is_unspecified_true() {
        assert!(MacAddr::UNSPECIFIED.is_unspecified());
        assert!(MacAddr([0; 6]).is_unspecified());
    }

    #[test]
    fn check_mac_address_is_unspecified_false() {
        assert!(!MacAddr([1, 2, 3, 4, 5, 6]).is_unspecified());
    }

    // This test exists to ensure that the `TryFrom<usize>` and `FromStr` traits are implemented
    // on `Uid`, `Gid` and `Pid`.
    #[allow(clippy::unnecessary_fallible_conversions)]
    #[test]
    fn check_uid_gid_from_impls() {
        use std::convert::TryFrom;
        use std::str::FromStr;

        #[cfg(not(windows))]
        {
            assert!(crate::Uid::try_from(0usize).is_ok());
            assert!(crate::Uid::from_str("0").is_ok());
        }
        #[cfg(windows)]
        {
            assert!(crate::Uid::from_str("S-1-5-18").is_ok()); // SECURITY_LOCAL_SYSTEM_RID
            assert!(crate::Uid::from_str("0").is_err());
        }

        assert!(crate::Gid::try_from(0usize).is_ok());
        assert!(crate::Gid::from_str("0").is_ok());

        assert!(crate::Pid::try_from(0usize).is_ok());
        // If it doesn't panic, it's fine.
        let _ = crate::Pid::from(0);
        assert!(crate::Pid::from_str("0").is_ok());
    }

    #[test]
    fn check_groups() {
        if !crate::IS_SUPPORTED_SYSTEM {
            return;
        }
        assert!(!Groups::new_with_refreshed_list().is_empty());
    }
}
