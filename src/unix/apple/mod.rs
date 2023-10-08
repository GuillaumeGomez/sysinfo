// Take a look at the license at the top of the repository in the LICENSE file.

#[cfg(target_os = "macos")]
pub(crate) mod macos;

#[cfg(target_os = "macos")]
pub(crate) use self::macos as inner;

#[cfg(target_os = "ios")]
pub(crate) mod ios;
#[cfg(target_os = "ios")]
pub(crate) use self::ios as inner;

#[cfg(any(target_os = "ios", feature = "apple-sandbox"))]
pub(crate) mod app_store;

pub mod component;
pub mod cpu;
pub mod disk;
mod ffi;
pub mod network;
pub mod process;
pub mod system;
pub mod users;
mod utils;

pub use self::component::{Component, Components};
pub use self::cpu::Cpu;
pub(crate) use self::disk::DiskInner;
pub use self::network::NetworkData;
pub use self::process::Process;
pub use self::system::System;
pub(crate) use crate::unix::users::get_users;
pub use crate::unix::users::User;
pub(crate) use crate::unix::DisksInner;
