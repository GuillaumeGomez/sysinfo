// Take a look at the license at the top of the repository in the LICENSE file.

pub mod component;
pub mod cpu;
pub mod disk;
pub mod groups;
pub mod network;
pub mod process;
pub mod system;
pub mod users;

pub(crate) use self::component::{ComponentInner, ComponentsInner};
pub(crate) use self::cpu::CpuInner;
pub(crate) use self::disk::{DiskInner, DisksInner};
pub(crate) use self::groups::get_groups;
pub(crate) use self::network::{NetworkDataInner, NetworksInner};
pub(crate) use self::process::ProcessInner;
pub(crate) use self::system::SystemInner;
pub(crate) use self::users::{get_users, UserInner};

use std::time::Duration;

declare_signals! {
    (),
    _ => None,
}

#[doc = include_str!("../../md_doc/is_supported.md")]
pub const IS_SUPPORTED_SYSTEM: bool = false;
#[doc = include_str!("../../md_doc/supported_signals.md")]
pub const SUPPORTED_SIGNALS: &[crate::Signal] = supported_signals();
#[doc = include_str!("../../md_doc/minimum_cpu_update_interval.md")]
pub const MINIMUM_CPU_UPDATE_INTERVAL: Duration = Duration::from_millis(0);
