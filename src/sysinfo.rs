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
//! let mut system = sysinfo::System::new();
//!
//! // First we update all information of our system struct.
//! system.refresh_all();
//!
//! // Now let's print every process' id and name:
//! for (pid, proc_) in system.get_process_list() {
//!     println!("{}:{} => status: {:?}", pid, proc_.name(), proc_.status());
//! }
//!
//! // Then let's print the temperature of the different components:
//! for component in system.get_components_list() {
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
//#![deny(warnings)]
#![allow(unknown_lints)]

extern crate num_cpus;

#[macro_use]
extern crate cfg_if;
#[cfg(not(any(target_os = "unknown", target_arch = "wasm32")))]
extern crate libc;
extern crate rayon;

#[macro_use]
extern crate doc_comment;

#[cfg(test)]
doctest!("../README.md");

cfg_if! {
    if #[cfg(target_os = "macos")] {
        mod mac;
        use mac as sys;
    } else if #[cfg(windows)] {
        mod windows;
        use windows as sys;
        extern crate winapi;
        extern crate ntapi;
    } else if #[cfg(unix)] {
        mod linux;
        use linux as sys;
    } else {
        mod unknown;
        use unknown as sys;
    }
}

pub extern crate cache_size;
pub extern crate pnet_datalink as datalink;

pub use common::{AsU32, Pid, RefreshKind};
pub use io::IOLoad;
pub use net::NICLoad;
pub use num_cpus::{get as get_logical_cores, get_physical as get_physical_cores};

pub use sys::{
    get_avg_load, get_cpu_frequency, Component, Disk, DiskType, NetworkData, Process,
    ProcessStatus, Processor, System,
};
pub use traits::{ComponentExt, DiskExt, NetworkExt, ProcessExt, ProcessorExt, SystemExt};

#[cfg(feature = "c-interface")]
pub use c_interface::*;
pub use utils::get_current_pid;

use std::collections::HashMap;

#[cfg(feature = "c-interface")]
mod c_interface;
mod common;
mod component;
mod io;
mod net;
mod process;
mod processor;
mod system;
mod traits;
mod utils;

/// This function is only used on linux targets, on the other platforms it does nothing.
///
/// On linux, to improve performance, we keep a `/proc` file open for each process we index with
/// a maximum number of files open equivalent to half of the system limit.
///
/// The problem is that some users might need all the available file descriptors so we need to
/// allow them to change this limit. Reducing 
///
/// Note that if you set a limit bigger than the system limit, the system limit will be set.
///
/// Returns `true` if the new value has been set.
pub fn set_open_files_limit(mut new_limit: isize) -> bool {
    #[cfg(all(not(target_os = "macos"), unix))]
    {
        if new_limit < 0 {
            new_limit = 0;
        }
        let max = sys::system::get_max_nb_fds();
        if new_limit > max {
            new_limit = max;
        }
        return if let Ok(ref mut x) = unsafe { sys::system::REMAINING_FILES.lock() } {
            // If files are already open, to be sure that the number won't be bigger when those
            // files are closed, we subtract the current number of opened files to the new limit.
            let diff = max - **x;
            **x = new_limit - diff;
            true
        } else {
            false
        };
    }
    #[cfg(any(not(unix), target_os = "macos"))]
    {
        return false;
    }
}

/// An enum representing signal on UNIX-like systems.
#[repr(C)]
#[derive(Clone, PartialEq, PartialOrd, Debug, Copy)]
pub enum Signal {
    /// Hangup detected on controlling terminal or death of controlling process.
    Hangup = 1,
    /// Interrupt from keyboard.
    Interrupt = 2,
    /// Quit from keyboard.
    Quit = 3,
    /// Illegal instruction.
    Illegal = 4,
    /// Trace/breakpoint trap.
    Trap = 5,
    /// Abort signal from C abort function.
    Abort = 6,
    // IOT trap. A synonym for SIGABRT.
    // IOT = 6,
    /// Bus error (bad memory access).
    Bus = 7,
    /// Floating point exception.
    FloatingPointException = 8,
    /// Kill signal.
    Kill = 9,
    /// User-defined signal 1.
    User1 = 10,
    /// Invalid memory reference.
    Segv = 11,
    /// User-defined signal 2.
    User2 = 12,
    /// Broken pipe: write to pipe with no readers.
    Pipe = 13,
    /// Timer signal from C alarm function.
    Alarm = 14,
    /// Termination signal.
    Term = 15,
    /// Stack fault on coprocessor (unused).
    Stklft = 16,
    /// Child stopped or terminated.
    Child = 17,
    /// Continue if stopped.
    Continue = 18,
    /// Stop process.
    Stop = 19,
    /// Stop typed at terminal.
    TSTP = 20,
    /// Terminal input for background process.
    TTIN = 21,
    /// Terminal output for background process.
    TTOU = 22,
    /// Urgent condition on socket.
    Urgent = 23,
    /// CPU time limit exceeded.
    XCPU = 24,
    /// File size limit exceeded.
    XFSZ = 25,
    /// Virtual alarm clock.
    VirtualAlarm = 26,
    /// Profiling time expired.
    Profiling = 27,
    /// Windows resize signal.
    Winch = 28,
    /// I/O now possible.
    IO = 29,
    // Pollable event (Sys V). Synonym for IO
    //Poll = 29,
    /// Power failure (System V).
    Power = 30,
    /// Bad argument to routine (SVr4).
    Sys = 31,
}

/// A struct represents system load average value.
#[repr(C)]
#[derive(Default, Debug)]
pub struct LoadAvg {
    /// Average load within one minite.
    pub one: f64,
    /// Average load within five minites.
    pub five: f64,
    /// Average load within fifteen minites.
    pub fifteen: f64,
}

/// Returns system wide configuration
///
/// # Notes
///
/// Current only can be used in operating system mounted `procfs`
pub fn get_sysctl_list() -> HashMap<String, String> {
    const DIR: &str = "/proc/sys/";
    let mut result = HashMap::new();
    for entry in walkdir::WalkDir::new(DIR) {
        let entry = match entry {
            Ok(entry) => entry,
            _ => continue,
        };

        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            _ => continue,
        };

        let path = match entry.path().to_str() {
            Some(p) => p,
            _ => continue,
        };
        let name = path.trim_start_matches(DIR).replace("/", ".");
        result.insert(name, content.trim().to_string());
    }
    result
}

#[cfg(test)]
mod test {
    use traits::{ProcessExt, SystemExt};

    #[test]
    fn check_memory_usage() {
        let mut s = ::System::new();

        s.refresh_all();
        assert_eq!(
            s.get_process_list()
                .iter()
                .all(|(_, proc_)| proc_.memory() == 0),
            false
        );
    }

    #[test]
    fn test_get_cpu_frequency() {
        println!("test get_cpu_frequency: {}", ::get_cpu_frequency());
    }

    #[test]
    fn test_get_avg_load() {
        println!("test get_avg_load: {:?}", ::get_avg_load());
    }

    #[test]
    fn test_nic_load() {
        println!("test test_nic_load: {:?}", ::NICLoad::snapshot());
    }

    #[test]
    fn test_io_load() {
        println!("test test_io_load: {:?}", ::IOLoad::snapshot());
    }

    #[test]
    fn test_get_cores() {
        assert_ne!(::get_logical_cores(), 0, "expect none-zero logical core");
        assert_ne!(::get_physical_cores(), 0, "expect none-zero physical core");
    }

    #[test]
    fn test_cache_size() {
        let caches = vec![
            ("l1-cache-size", ::cache_size::l1_cache_size()),
            ("l1-cache-line-size", ::cache_size::l1_cache_line_size()),
            ("l2-cache-size", ::cache_size::l2_cache_size()),
            ("l2-cache-line-size", ::cache_size::l2_cache_line_size()),
            ("l3-cache-size", ::cache_size::l3_cache_size()),
            ("l3-cache-line-size", ::cache_size::l3_cache_line_size()),
        ];
        for c in caches {
            assert_ne!(c.1.unwrap(), 0, "{} expect non-zero", c.0)
        }
    }
}
