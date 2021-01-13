//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(target_os = "macos")]
pub(crate) use self::macos as inner;

#[cfg(target_os = "ios")]
pub(crate) mod ios;
#[cfg(target_os = "ios")]
pub(crate) use self::ios as inner;

pub mod component;
pub mod disk;
mod ffi;
pub mod network;
pub mod process;
pub mod processor;
pub mod system;
pub mod users;
mod utils;

pub use self::component::Component;
pub use self::disk::Disk;
pub use self::network::{NetworkData, Networks};
pub use self::process::{Process, ProcessStatus};
pub use self::processor::Processor;
pub use self::system::System;
