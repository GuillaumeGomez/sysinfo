// license info

/*!

*/

#![crate_name = "sysinfo"]
#![crate_type = "lib"]
#![crate_type = "rlib"]

#![feature(std_misc, old_io, collections, old_path, io, path, core, os)]

extern crate libc;

pub use self::processus::*;
pub use self::system::*;

pub mod processus;
pub mod system;
mod ffi;

#[repr(C)]
#[derive(Clone, PartialEq, PartialOrd, Debug, Copy)]
pub enum Signal {
	/// Hangup detected on controlling terminal or death of controlling processus
	Hangup = 1,
	/// Interrupt from keyboard
	Interrupt = 2,
	/// Quit from keyboard
	Quit = 3,
	/// Illegal instruction
	Illegal = 4,
	/// Trace/breakpoint trap
	Trap = 5,
	/// Abort signal from C abort function
	Abort = 6,
	// IOT trap. A synonym for SIGABRT
	//IOT = 6,
	/// Bus error (bad memory access)
	Bus = 7,
	/// Floating point exception
	FloatingPointException = 8,
	/// Kill signal
	Kill = 9,
	/// User-defined signal 1
	User1 = 10,
	/// Invalid memory reference
	Segv = 11,
	/// User-defined signal 2
	User2 = 12,
	/// Broken pipe: write to pipe with no readers
	Pipe = 13,
	/// Timer signal from C alarm function
	Alarm = 14,
	/// Termination signal
	Term = 15,
	/// Stack fault on coprocessor (unused)
	Stklft = 16,
	/// Child stopped or terminated
	Child = 17,
	/// Continue if stopped
	Continue = 18,
	/// Stop process
	Stop = 19,
	/// Stop typed at terminal
	TSTP = 20,
	/// Terminal input for background process
	TTIN = 21,
	/// Terminal output for background process
	TTOU = 22,
	/// Urgent condition on socket
	Urgent = 23,
	/// CPU time limit exceeded
	XCPU = 24,
	/// File size limit exceeded
	XFSZ = 25,
	/// Virtual alarm clock
	VirtualAlarm = 26,
	/// Profiling time expired
	Profiling = 27,
	/// Windows resize signal
	Winch = 28,
	/// I/O now possible
	IO = 29,
	// Pollable event (Sys V). Synonym for IO
	//Poll = 29,
	/// Power failure (System V)
	Power = 30,
	/// Bad argument to routine (SVr4)
	Sys = 31
}