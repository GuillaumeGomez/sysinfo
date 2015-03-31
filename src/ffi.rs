// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use libc::{c_int};

extern "C" {
	pub fn kill(pid: c_int, signal: c_int) -> c_int;
}