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

pub(crate) use self::component::{ComponentInner, ComponentsInner};
pub(crate) use self::cpu::CpuInner;
pub(crate) use self::disk::DiskInner;
pub(crate) use self::network::{NetworkDataInner, NetworksInner};
pub(crate) use self::process::ProcessInner;
pub(crate) use self::system::SystemInner;
pub(crate) use crate::unix::users::{get_users, UserInner};
pub(crate) use crate::unix::DisksInner;

use std::time::Duration;

#[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
declare_signals! {
    libc::c_int,
    Signal::Hangup => libc::SIGHUP,
    Signal::Interrupt => libc::SIGINT,
    Signal::Quit => libc::SIGQUIT,
    Signal::Illegal => libc::SIGILL,
    Signal::Trap => libc::SIGTRAP,
    Signal::Abort => libc::SIGABRT,
    Signal::IOT => libc::SIGIOT,
    Signal::Bus => libc::SIGBUS,
    Signal::FloatingPointException => libc::SIGFPE,
    Signal::Kill => libc::SIGKILL,
    Signal::User1 => libc::SIGUSR1,
    Signal::Segv => libc::SIGSEGV,
    Signal::User2 => libc::SIGUSR2,
    Signal::Pipe => libc::SIGPIPE,
    Signal::Alarm => libc::SIGALRM,
    Signal::Term => libc::SIGTERM,
    Signal::Child => libc::SIGCHLD,
    Signal::Continue => libc::SIGCONT,
    Signal::Stop => libc::SIGSTOP,
    Signal::TSTP => libc::SIGTSTP,
    Signal::TTIN => libc::SIGTTIN,
    Signal::TTOU => libc::SIGTTOU,
    Signal::Urgent => libc::SIGURG,
    Signal::XCPU => libc::SIGXCPU,
    Signal::XFSZ => libc::SIGXFSZ,
    Signal::VirtualAlarm => libc::SIGVTALRM,
    Signal::Profiling => libc::SIGPROF,
    Signal::Winch => libc::SIGWINCH,
    Signal::IO => libc::SIGIO,
    // SIGPOLL doesn't exist on apple targets but since it's an equivalent of SIGIO on unix,
    // we simply use the SIGIO constant.
    Signal::Poll => libc::SIGIO,
    Signal::Sys => libc::SIGSYS,
    _ => None,
}
#[cfg(any(target_os = "ios", feature = "apple-sandbox"))]
declare_signals! {
    libc::c_int,
    _ => None,
}

#[doc = include_str!("../../../md_doc/is_supported.md")]
pub const IS_SUPPORTED_SYSTEM: bool = true;
#[doc = include_str!("../../../md_doc/supported_signals.md")]
pub const SUPPORTED_SIGNALS: &[crate::Signal] = supported_signals();
#[doc = include_str!("../../../md_doc/minimum_cpu_update_interval.md")]
pub const MINIMUM_CPU_UPDATE_INTERVAL: Duration = Duration::from_millis(200);
