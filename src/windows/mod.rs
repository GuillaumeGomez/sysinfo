// Take a look at the license at the top of the repository in the LICENSE file.

mod component;
mod groups;
mod network;
pub(crate) mod network_helper;

cfg_if! {
    if #[cfg(feature = "system")] {
        mod process;
        mod cpu;
        mod system;

        pub(crate) use self::cpu::CpuInner;
        pub(crate) use self::process::ProcessInner;
        pub(crate) use self::system::SystemInner;
        pub use self::system::{MINIMUM_CPU_UPDATE_INTERVAL, SUPPORTED_SIGNALS};
    }
    if #[cfg(feature = "disk")] {
        mod disk;

        pub(crate) use self::disk::{DiskInner, DisksInner};
    }
}

mod sid;
mod users;
mod utils;

pub(crate) use self::component::{ComponentInner, ComponentsInner};
pub(crate) use self::groups::get_groups;
pub(crate) use self::network::{NetworkDataInner, NetworksInner};
pub(crate) use self::sid::Sid;
pub(crate) use self::users::get_users;
pub(crate) use self::users::UserInner;

#[doc = include_str!("../../md_doc/is_supported.md")]
pub const IS_SUPPORTED_SYSTEM: bool = true;
