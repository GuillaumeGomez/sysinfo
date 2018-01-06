//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

/// Trait to have a common fallback for the `Pid` type.
pub trait AsU32 {
    /// Allows to convert `Pid` into `u32`.
    fn as_u32(&self) -> u32;
}

cfg_if!{
    if #[cfg(windows)] {
        /// Process id.
        pub type Pid = usize;

        impl AsU32 for Pid {
            fn as_u32(&self) -> u32 {
                *self as u32
            }
        }
    } else {
        use libc::pid_t;

        /// Process id.
        pub type Pid = pid_t;

        impl AsU32 for Pid {
            fn as_u32(&self) -> u32 {
                *self as u32
            }
        }
    }
}
