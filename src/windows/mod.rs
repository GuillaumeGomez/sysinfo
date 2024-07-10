// Take a look at the license at the top of the repository in the LICENSE file.

mod groups;

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

    if #[cfg(feature = "component")] {
        pub mod component;

        pub(crate) use self::component::{ComponentInner, ComponentsInner};
    }

    if #[cfg(feature = "network")] {
        mod network;
        pub(crate) mod network_helper;

        pub(crate) use self::network::{NetworkDataInner, NetworksInner};
    }
}

mod sid;
mod users;
mod utils;

pub(crate) use self::groups::get_groups;
pub(crate) use self::sid::Sid;
pub(crate) use self::users::get_users;
pub(crate) use self::users::UserInner;

#[doc = include_str!("../../md_doc/is_supported.md")]
pub const IS_SUPPORTED_SYSTEM: bool = true;
