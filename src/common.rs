//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

cfg_if!{
    if #[cfg(windows)] {
        /// Process id.
        pub type Pid = usize;
    } else {
        use libc::pid_t;

        /// Process id.
        pub type Pid = pid_t;
    }
}
