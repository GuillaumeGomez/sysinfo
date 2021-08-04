//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

pub mod disk;
pub mod ffi;

#[cfg(not(feature = "apple-sandbox"))]
pub mod system;

#[cfg(not(feature = "apple-sandbox"))]
pub mod component;
#[cfg(not(feature = "apple-sandbox"))]
pub mod process;

#[cfg(feature = "apple-sandbox")]
pub use crate::sys::app_store::component;
#[cfg(feature = "apple-sandbox")]
pub use crate::sys::app_store::process;
