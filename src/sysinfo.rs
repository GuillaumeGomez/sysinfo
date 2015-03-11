// license info

/*!

*/

#![crate_name = "sysinfo"]
#![crate_type = "lib"]
#![crate_type = "rlib"]

#![feature(std_misc, old_io, collections, old_path, io, path, core)]

pub use self::processus::*;
pub use self::system::*;

pub mod processus;
pub mod system;