// Take a look at the license at the top of the repository in the LICENSE file.

#[doc(hidden)]
#[allow(unused)]
macro_rules! sysinfo_debug {
    ($($x:tt)*) => {{
        #[cfg(feature = "debug")]
        eprintln!($($x)*);
    }}
}

#[cfg(feature = "system")]
macro_rules! declare_signals {
    ($kind:ty, _ => None,) => (
        use crate::Signal;

        pub(crate) const fn supported_signals() -> &'static [Signal] {
            &[]
        }
    );

    ($kind:ty, $(Signal::$signal:ident => $map:expr,)+ _ => None,) => (
        use crate::Signal;

        pub(crate) const fn supported_signals() -> &'static [Signal] {
            &[$(Signal::$signal,)*]
        }

        #[inline]
        pub(crate) fn convert_signal(s: Signal) -> Option<$kind> {
            match s {
                $(Signal::$signal => Some($map),)*
                _ => None,
            }
        }
    );

    ($kind:ty, $(Signal::$signal:ident => $map:expr,)+) => (
        use crate::Signal;

        pub(crate) const fn supported_signals() -> &'static [Signal] {
            &[$(Signal::$signal,)*]
        }

        #[inline]
        pub(crate) fn convert_signal(s: Signal) -> Option<$kind> {
            match s {
                $(Signal::$signal => Some($map),)*
            }
        }
    )
}

#[cfg(all(unix, not(feature = "unknown-ci")))]
#[allow(unused_macros)]
macro_rules! retry_eintr {
    // Resets `errno` before every attempt, not only the first one: otherwise a stale `EINTR` from an
    // interrupted attempt is still there when the retried call succeeds without touching `errno`.
    (set_to_0 => $kind:ident => $($t:tt)+) => {
        retry_eintr!($kind => { retry_eintr!(@reset_errno); $($t)+ })
    };
    (set_to_0 => $($t:tt)+) => {
        retry_eintr!({ retry_eintr!(@reset_errno); $($t)+ })
    };
    (@reset_errno) => {{
        #[allow(unused_unsafe)]
        let errno = unsafe { crate::unix::libc_errno() };
        if !errno.is_null() {
            #[allow(unused_unsafe)]
            unsafe { *errno = 0; }
        }
    }};
    (null => $($t:tt)+) => {{
        loop {
            let ret = $($t)+;
            if ret.is_null() && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            break ret;
        }
    }};
    ($errno_value:ident => $($t:tt)+) => {{
        loop {
            let ret = $($t)+;
            if ret < 0 {
                let tmp = std::io::Error::last_os_error();
                if tmp.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                $errno_value = tmp.raw_os_error().unwrap_or(0);
            }
            break ret;
        }
    }};
    ($($t:tt)+) => {{
        loop {
            let ret = $($t)+;
            if ret < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            break ret;
        }
    }};
}

#[cfg(test)]
#[allow(unused_macros)]
macro_rules! check_unsupported {
    ($e:expr) => {
        match $e {
            Ok(a) => a,
            Err(error) => {
                if !matches!(error, crate::Error::Unsupported) {
                    panic!("Expected `Error::Unsupported`, found {error:?}");
                }
                return;
            }
        }
    };
}
