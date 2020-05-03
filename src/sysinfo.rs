//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

//! `sysinfo` is a crate used to get a system's information.
//!
//! Before any attempt to read the different structs' information, you need to update them to
//! get up-to-date information.
//!
//! # Examples
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
//! println!("total memory: {} KiB", system.get_total_memory());
//! println!("used memory : {} KiB", system.get_used_memory());
//! println!("total swap  : {} KiB", system.get_total_swap());
//! println!("used swap   : {} KiB", system.get_used_swap());
//! ```

#![crate_name = "sysinfo"]
#![crate_type = "lib"]
#![crate_type = "rlib"]
#![deny(missing_docs)]
#![deny(intra_doc_link_resolution_failure)]
//#![deny(warnings)]
#![allow(unknown_lints)]

#[macro_use]
extern crate cfg_if;
#[cfg(not(any(target_os = "unknown", target_arch = "wasm32")))]
extern crate libc;
extern crate rayon;

#[macro_use]
extern crate doc_comment;

extern crate once_cell;

#[cfg(doctest)]
doctest!("../README.md");

#[cfg(feature = "debug")]
#[doc(hidden)]
macro_rules! sysinfo_debug {
    ($($x:tt)*) => {{
        eprintln!($($x)*);
    }}
}

#[cfg(not(feature = "debug"))]
#[doc(hidden)]
macro_rules! sysinfo_debug {
    ($($x:tt)*) => {{}};
}

cfg_if! {
    if #[cfg(target_os = "macos")] {
        mod mac;
        use mac as sys;

        #[cfg(test)]
        const MIN_USERS: usize = 1;
    } else if #[cfg(windows)] {
        mod windows;
        use windows as sys;
        extern crate winapi;
        extern crate ntapi;

        #[cfg(test)]
        const MIN_USERS: usize = 1;
    } else if #[cfg(unix)] {
        mod linux;
        use linux as sys;

        #[cfg(test)]
        const MIN_USERS: usize = 1;
    } else {
        mod unknown;
        use unknown as sys;

        #[cfg(test)]
        const MIN_USERS: usize = 0;
    }
}

pub use common::{
    AsU32, DiskType, DiskUsage, LoadAvg, NetworksIter, Pid, RefreshKind, Signal, User,
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
    #[cfg(all(not(target_os = "macos"), unix))]
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
    #[cfg(any(not(unix), target_os = "macos"))]
    {
        false
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_memory_usage() {
        let mut s = ::System::new();

        s.refresh_all();
        assert_eq!(
            s.get_processes()
                .iter()
                .all(|(_, proc_)| proc_.memory() == 0),
            false
        );
    }

    #[test]
    fn check_users() {
        let mut s = ::System::new();
        assert!(s.get_users().is_empty());
        s.refresh_users_list();
        assert!(s.get_users().len() >= MIN_USERS);

        let mut s = ::System::new();
        assert!(s.get_users().is_empty());
        s.refresh_all();
        assert!(s.get_users().is_empty());

        let s = ::System::new_all();
        assert!(s.get_users().len() >= MIN_USERS);
    }
}

// Used to check that System is Send and Sync.
#[cfg(doctest)]
doc_comment!(
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
