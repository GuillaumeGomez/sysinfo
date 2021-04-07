//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

pub mod component;
pub mod disk;
pub mod ffi;
pub mod system;

#[cfg(not(feature = "apple-app-store"))]
pub mod process;

#[cfg(feature = "apple-app-store")]
pub use crate::sys::app_store::process;
