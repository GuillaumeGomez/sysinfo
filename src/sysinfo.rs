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
//! use sysinfo::SystemExt;
//!
//! let mut system = sysinfo::System::new();
//!
//! // First we update all information of our system struct.
//! system.refresh_all();
//!
//! // Now let's print every process' id and name:
//! for (pid, proc_) in system.get_process_list() {
//!     println!("{}:{} => status: {:?}", pid, proc_.name, proc_.status);
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
//! println!("total memory: {} kB", system.get_total_memory());
//! println!("used memory : {} kB", system.get_used_memory());
//! println!("total swap  : {} kB", system.get_total_swap());
//! println!("used swap   : {} kB", system.get_used_swap());
//! ```

#![crate_name = "sysinfo"]
#![crate_type = "lib"]
#![crate_type = "rlib"]

#![deny(missing_docs)]
#![deny(warnings)]
#![allow(unknown_lints)]
#![allow(too_many_arguments)]

extern crate libc;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
use mac as sys;
#[cfg(not(target_os = "macos"))]
mod linux;
#[cfg(not(target_os = "macos"))]
use linux as sys;

pub use sys::{
    Component,
    Process,
    ProcessStatus,
    Processor,
    System,
    Disk,
    DiskType,
};
pub use traits::{
    DiskExt,
    ProcessExt,
    ProcessorExt,
    SystemExt,
};

mod component;
mod process;
mod processor;
mod system;
mod traits;
mod utils;
mod enums;

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
