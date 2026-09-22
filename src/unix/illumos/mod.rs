// Take a look at the license at the top of the repository in the LICENSE file.

#[cfg(any(
    feature = "disk",
    feature = "gpu",
    feature = "network",
    feature = "system"
))]
mod ffi;
#[cfg(any(feature = "disk", feature = "network", feature = "system"))]
mod kstat;

cfg_select! {
    feature = "system" => {
        pub mod cpu;
        pub mod motherboard;
        pub mod process;
        pub mod product;
        pub mod system;
        mod smbios;

        pub(crate) use self::cpu::CpuInner;
        pub(crate) use self::motherboard::MotherboardInner;
        pub(crate) use self::process::ProcessInner;
        pub(crate) use self::product::ProductInner;
        pub(crate) use self::system::SystemInner;

        #[path = "../bsd/system_common.rs"]
        mod system_common;
        pub use self::system_common::*;
    }
    _ => {}
}
cfg_select! {
    feature = "disk" => {
        pub mod disk;

        pub(crate) use self::disk::DiskInner;
        pub(crate) use crate::unix::DisksInner;
    }
    _ => {}
}
cfg_select! {
    feature = "component" => {
        pub mod component;

        pub(crate) use self::component::{ComponentInner, ComponentsInner};
    }
    _ => {}
}
cfg_select! {
    feature = "gpu" => {
        pub mod gpu;

        pub(crate) use self::gpu::{GpuInner, GpusInner};
    }
    _ => {}
}
cfg_select! {
    feature = "network" => {
        pub mod network;
        mod network_common;

        pub(crate) use self::network::NetworksInner;
        pub(crate) use self::network_common::NetworkDataInner;
    }
    _ => {}
}
cfg_select! {
    feature = "user" => {
        pub(crate) use crate::unix::groups::{get_groups, new_groups};
        pub(crate) use crate::unix::users::{UserInner, get_users, new_users};
    }
    _ => {}
}

#[doc = include_str!("../../../md_doc/is_supported.md")]
pub const IS_SUPPORTED_SYSTEM: bool = true;

// Little trick to ensure `rustfmt` works on all files.
#[cfg(any())]
mod component;
#[cfg(any())]
mod cpu;
#[cfg(any())]
mod disk;
#[cfg(any())]
mod gpu;
#[cfg(any())]
mod motherboard;
#[cfg(any())]
mod network;
#[cfg(any())]
mod network_common;
#[cfg(any())]
mod process;
#[cfg(any())]
mod product;
#[cfg(any())]
mod smbios;
#[cfg(any())]
mod system;
