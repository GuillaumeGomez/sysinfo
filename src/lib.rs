//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

//! `sysinfo` is a crate used to get a system's information.
//!
//! ## Supported Oses
//!
//! It currently supports the following OSes (alphabetically sorted):
//!  * Android
//!  * iOS
//!  * Linux
//!  * macOS
//!  * Windows
//!
//! You can still use `sysinfo` on non-supported OSes, it'll simply do nothing and always return
//! empty values. You can check in your program directly if an OS is supported by checking the
//! [`SystemExt::IS_SUPPORTED`] constant.
//!
//! ## Usage
//!
//! /!\ Before any attempt to read the different structs' information, you need to update them to
//! get up-to-date information because for most of them, it works on diff between the current value
//! and the old one.
//!
//! Which is why, it's much better to keep the same instance of [`System`] around instead of
//! recreating it multiple times.
//!
//! ## Examples
//!
//! ```
//! use sysinfo::{ProcessExt, SystemExt};
//!
//! let mut system = sysinfo::System::new_all();
//!
//! // First we update all information of our system struct.
//! system.refresh_all();
//!
//! // Now let's print every process' id and name:
//! for (pid, proc_) in system.get_processes() {
//!     println!("{}:{} => status: {:?}", pid, proc_.name(), proc_.status());
//! }
//!
//! // Then let's print the temperature of the different components:
//! for component in system.get_components() {
//!     println!("{:?}", component);
//! }
//!
//! // And then all disks' information:
//! for disk in system.get_disks() {
//!     println!("{:?}", disk);
//! }
//!
//! // And finally the RAM and SWAP information:
//! println!("total memory: {} KB", system.get_total_memory());
//! println!("used memory : {} KB", system.get_used_memory());
//! println!("total swap  : {} KB", system.get_total_swap());
//! println!("used swap   : {} KB", system.get_used_swap());
//!
//! // Display system information:
//! println!("System name:             {:?}", system.get_name());
//! println!("System kernel version:   {:?}", system.get_kernel_version());
//! println!("System OS version:       {:?}", system.get_os_version());
//! println!("System host name:        {:?}", system.get_host_name());
//! ```

#![crate_name = "sysinfo"]
#![crate_type = "lib"]
#![crate_type = "rlib"]
#![allow(unknown_lints)]
#![deny(missing_docs)]
#![deny(broken_intra_doc_links)]
#![allow(clippy::upper_case_acronyms)]
#![allow(renamed_and_removed_lints)]

#[cfg(doctest)]
doc_comment::doctest!("../README.md");

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
    if #[cfg(any(target_os = "macos", target_os = "ios"))] {
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
    AsU32, DiskType, DiskUsage, Gid, LoadAvg, NetworksIter, Pid, RefreshKind, Signal, Uid, User,
};
pub use sys::{Component, Disk, NetworkData, Networks, Process, ProcessStatus, Processor, System};
pub use traits::{
    ComponentExt, DiskExt, NetworkExt, NetworksExt, ProcessExt, ProcessorExt, SystemExt, UserExt,
};

#[cfg(feature = "c-interface")]
pub use c_interface::*;
pub use utils::get_current_pid;

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
    #[cfg(any(target_os = "linux", target_os = "android"))]
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
            // files are closed, we subtract the current number of opened files to the new limit.
            let diff = max - **x;
            **x = _new_limit - diff;
            true
        } else {
            false
        }
    }
    #[cfg(all(not(target_os = "linux"), not(target_os = "android")))]
    {
        false
    }
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn check_memory_usage() {
        // We don't want to test on unsupported systems.
        if System::IS_SUPPORTED {
            let mut s = System::new();
            s.refresh_all();
            assert_eq!(
                s.get_processes()
                    .iter()
                    .all(|(_, proc_)| proc_.memory() == 0),
                false
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn check_cpu_usage() {
        let mut s = System::new();

        s.refresh_all();
        // All CPU usage will start at zero until the second refresh
        assert_eq!(
            s.get_processes()
                .iter()
                .all(|(_, proc_)| proc_.cpu_usage() == 0.0),
            true
        );

        // Wait a bit to update CPU usage values
        std::thread::sleep(std::time::Duration::from_millis(100));
        s.refresh_all();
        assert_eq!(
            s.get_processes()
                .iter()
                .all(|(_, proc_)| proc_.cpu_usage() >= 0.0
                    && proc_.cpu_usage() <= (s.get_processors().len() as f32) * 100.0),
            true
        );
    }

    #[test]
    fn check_users() {
        let mut s = System::new();
        assert!(s.get_users().is_empty());
        s.refresh_users_list();
        assert!(s.get_users().len() >= MIN_USERS);

        let mut s = System::new();
        assert!(s.get_users().is_empty());
        s.refresh_all();
        assert!(s.get_users().is_empty());

        let s = System::new_all();
        assert!(s.get_users().len() >= MIN_USERS);
    }

    #[test]
    fn check_uid_gid() {
        let mut s = System::new();
        assert!(s.get_users().is_empty());
        s.refresh_users_list();
        let users = s.get_users();
        assert!(users.len() >= MIN_USERS);

        for user in users {
            match user.get_name() {
                "root" => {
                    assert_eq!(*user.get_uid(), 0);
                    assert_eq!(*user.get_gid(), 0);
                }
                _ => {
                    assert!(*user.get_uid() > 0);
                    #[cfg(not(target_os = "windows"))]
                    assert!(*user.get_gid() > 0);
                }
            }
        }
    }

    #[test]
    fn check_system_info() {
        // We don't want to test on unsupported systems.
        if System::IS_SUPPORTED {
            let s = System::new();
            assert!(!s.get_name().expect("Failed to get system name").is_empty());

            assert!(!s
                .get_kernel_version()
                .expect("Failed to get kernel version")
                .is_empty());

            assert!(!s
                .get_os_version()
                .expect("Failed to get os version")
                .is_empty());

            assert!(!s
                .get_long_os_version()
                .expect("Failed to get long OS version")
                .is_empty());
        }
    }

    #[test]
    fn check_host_name() {
        // We don't want to test on unsupported systems.
        if System::IS_SUPPORTED {
            let s = System::new();
            assert!(!s
                .get_host_name()
                .expect("Failed to get host name")
                .is_empty());
        }
    }

    #[test]
    fn check_refresh_process_return_value() {
        // We don't want to test on unsupported systems.
        if System::IS_SUPPORTED {
            let pid = get_current_pid().expect("Failed to get current PID");
            let mut s = System::new();

            // First check what happens in case the process isn't already in our process list.
            assert!(s.refresh_process(pid));
            // Then check that it still returns true if the process is already in our process list.
            assert!(s.refresh_process(pid));
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
            assert!(!s.get_processors().is_empty());
            let physical_cores_count = s
                .get_physical_core_count()
                .expect("failed to get number of physical cores");
            assert!(physical_cores_count > 0);
            assert!(physical_cores_count <= s.get_processors().len());
        } else {
            assert!(s.get_processors().is_empty());
            assert_eq!(s.get_physical_core_count(), None);
        }
    }
}

// Used to check that System is Send and Sync.
#[cfg(doctest)]
doc_comment::doc_comment!(
    "
```
fn is_send<T: Send>() {}
is_send::<sysinfo::System>();
```

```
fn is_sync<T: Sync>() {}
is_sync::<sysinfo::System>();
```"
);
