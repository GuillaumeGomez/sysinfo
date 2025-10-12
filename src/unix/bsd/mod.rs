use libc::c_int;
use std::time::Duration;

cfg_if! {
    if #[cfg(target_os = "freebsd")] {
        pub(crate) mod freebsd;
        pub(crate) use freebsd::*;

        #[allow(unused_imports)]
        pub(crate) use libc::__error as libc_errno;
    } else if #[cfg(target_os = "netbsd")] {
        pub(crate) mod netbsd;
        pub(crate) use netbsd::*;

        // #[allow(unused_imports)]
        // pub(crate) use libc::__error as libc_errno;
    } else {
        compile_error!("Invalid cfg!");
    }
}

#[cfg(any())]
mod freebsd;
#[cfg(any())]
mod netbsd;

declare_signals! {
    c_int,
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
    Signal::Sys => libc::SIGSYS,
    _ => None,
}

#[doc = include_str!("../../../md_doc/supported_signals.md")]
pub const SUPPORTED_SIGNALS: &[crate::Signal] = supported_signals();
#[doc = include_str!("../../../md_doc/minimum_cpu_update_interval.md")]
pub const MINIMUM_CPU_UPDATE_INTERVAL: Duration = Duration::from_millis(100);
#[doc = include_str!("../../../md_doc/is_supported.md")]
pub const IS_SUPPORTED_SYSTEM: bool = true;
