// Take a look at the license at the top of the repository in the LICENSE file.

#![doc = include_str!("../README.md")]
#![allow(unknown_lints)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![allow(clippy::upper_case_acronyms)]
#![allow(renamed_and_removed_lints)]
#![allow(unknown_lints)]

#[cfg(feature = "debug")]
#[doc(hidden)]
#[allow(unused)]
macro_rules! sysinfo_debug {
    ($($x:tt)*) => {{
        eprintln!($($x)*);
    }}
}

#[cfg(not(feature = "debug"))]
#[doc(hidden)]
#[allow(unused)]
macro_rules! sysinfo_debug {
    ($($x:tt)*) => {{}};
}

cfg_if::cfg_if! {
    if #[cfg(feature = "unknown-ci")] {
        // This is used in CI to check that the build for unknown targets is compiling fine.
        mod unknown;
        use unknown as sys;

        #[cfg(test)]
        pub(crate) const MIN_USERS: usize = 0;
    } else if #[cfg(any(target_os = "macos", target_os = "ios"))] {
        mod apple;
        use apple as sys;
        extern crate core_foundation_sys;

        #[cfg(test)]
        pub(crate) const MIN_USERS: usize = 1;
    } else if #[cfg(windows)] {
        mod windows;
        use windows as sys;
        extern crate winapi;
        extern crate ntapi;

        #[cfg(test)]
        pub(crate) const MIN_USERS: usize = 1;
    } else if #[cfg(any(target_os = "linux", target_os = "android"))] {
        mod linux;
        use linux as sys;

        #[cfg(test)]
        pub(crate) const MIN_USERS: usize = 1;
    } else {
        mod unknown;
        use unknown as sys;

        #[cfg(test)]
        pub(crate) const MIN_USERS: usize = 0;
    }
}

pub use common::{
    get_current_pid, AsU32, DiskType, DiskUsage, Gid, LoadAvg, NetworksIter, Pid,
    ProcessRefreshKind, ProcessStatus, RefreshKind, Signal, Uid, User,
};
pub use sys::{Component, Disk, NetworkData, Networks, Process, Processor, System};
pub use traits::{
    ComponentExt, DiskExt, NetworkExt, NetworksExt, ProcessExt, ProcessorExt, SystemExt, UserExt,
};

#[cfg(feature = "c-interface")]
pub use c_interface::*;

#[cfg(feature = "c-interface")]
mod c_interface;
mod common;
mod debug;
mod system;
mod traits;
mod utils;

/// This function is only used on linux targets, on the other platforms it does nothing and returns
/// `false`.
///
/// On linux, to improve performance, we keep a `/proc` file open for each process we index with
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
/// use sysinfo::{System, SystemExt, set_open_files_limit};
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
            if _new_limit < 0 {
                _new_limit = 0;
            }
            let max = sys::system::get_max_nb_fds();
            if _new_limit > max {
                _new_limit = max;
            }
            if let Ok(ref mut x) = unsafe { sys::system::REMAINING_FILES.lock() } {
                // If files are already open, to be sure that the number won't be bigger when those
                // files are closed, we subtract the current number of opened files to the new
                // limit.
                let diff = max - **x;
                **x = _new_limit - diff;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod test {
    use crate::*;

    #[cfg(feature = "unknown-ci")]
    #[test]
    fn check_unknown_ci_feature() {
        assert!(!System::IS_SUPPORTED);
    }

    #[test]
    fn check_process_memory_usage() {
        let mut s = System::new();
        s.refresh_all();

        if System::IS_SUPPORTED {
            // No process should have 0 as memory usage.
            #[cfg(not(feature = "apple-sandbox"))]
            assert!(!s.processes().iter().all(|(_, proc_)| proc_.memory() == 0));
        } else {
            // There should be no process, but if there is one, its memory usage should be 0.
            assert!(s.processes().iter().all(|(_, proc_)| proc_.memory() == 0));
        }
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
        if System::IS_SUPPORTED {
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
        if !System::IS_SUPPORTED {
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
        std::thread::sleep(std::time::Duration::from_millis(100));
        s.refresh_processes();
        assert!(s
            .processes()
            .iter()
            .all(|(_, proc_)| proc_.cpu_usage() >= 0.0
                && proc_.cpu_usage() <= (s.processors().len() as f32) * 100.0));
        assert!(s
            .processes()
            .iter()
            .any(|(_, proc_)| proc_.cpu_usage() > 0.0));
    }

    #[test]
    fn check_users() {
        let mut s = System::new();
        assert!(s.users().is_empty());
        s.refresh_users_list();
        assert!(s.users().len() >= MIN_USERS);

        let mut s = System::new();
        assert!(s.users().is_empty());
        s.refresh_all();
        assert!(s.users().is_empty());

        let s = System::new_all();
        assert!(s.users().len() >= MIN_USERS);
    }

    #[test]
    fn check_uid_gid() {
        let mut s = System::new();
        assert!(s.users().is_empty());
        s.refresh_users_list();
        let users = s.users();
        assert!(users.len() >= MIN_USERS);

        if System::IS_SUPPORTED {
            #[cfg(not(target_os = "windows"))]
            {
                let user = users
                    .iter()
                    .find(|u| u.name() == "root")
                    .expect("no root user");
                assert_eq!(*user.uid(), 0);
                assert_eq!(*user.gid(), 0);
                if let Some(user) = users.iter().find(|u| *u.gid() > 0) {
                    assert!(*user.uid() > 0);
                    assert!(*user.gid() > 0);
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                assert!(users.iter().filter(|u| *u.uid() > 0).count() > 0);
            }
        }
    }

    #[test]
    fn check_system_info() {
        // We don't want to test on unsupported systems.
        if System::IS_SUPPORTED {
            let s = System::new();
            assert!(!s.name().expect("Failed to get system name").is_empty());

            assert!(!s
                .kernel_version()
                .expect("Failed to get kernel version")
                .is_empty());

            assert!(!s.os_version().expect("Failed to get os version").is_empty());

            assert!(!s
                .long_os_version()
                .expect("Failed to get long OS version")
                .is_empty());
        }
    }

    #[test]
    fn check_host_name() {
        // We don't want to test on unsupported systems.
        if System::IS_SUPPORTED {
            let s = System::new();
            assert!(!s.host_name().expect("Failed to get host name").is_empty());
        }
    }

    #[test]
    fn check_refresh_process_return_value() {
        // We don't want to test on unsupported systems.
        if System::IS_SUPPORTED {
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
            assert!(System::IS_SUPPORTED);
        } else {
            assert!(!System::IS_SUPPORTED);
        }
    }

    #[test]
    fn check_processors_number() {
        let s = System::new();

        if System::IS_SUPPORTED {
            assert!(!s.processors().is_empty());
            // In case we are running inside a VM, it's possible to not have a physical core, only
            // logical ones, which is why we don't test `physical_cores_count > 0`.
            let physical_cores_count = s
                .physical_core_count()
                .expect("failed to get number of physical cores");
            assert!(physical_cores_count <= s.processors().len());
        } else {
            assert!(s.processors().is_empty());
            assert_eq!(s.physical_core_count(), None);
        }
        assert!(s.physical_core_count().unwrap_or(0) <= s.processors().len());
    }
}

// Used to check that System is Send and Sync.
#[cfg(doctest)]
/// ```
/// fn is_send<T: Send>() {}
/// is_send::<sysinfo::System>();
/// ```
///
/// ```
/// fn is_sync<T: Sync>() {}
/// is_sync::<sysinfo::System>();
/// ```
pub mod check_if_system_is_send_and_sync {}
