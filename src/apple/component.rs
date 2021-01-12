//
// Sysinfo
//
// Copyright (c) 2021 Guillaume Gomez
//

cfg_if! {
    if #[cfg(target_os = "macos")] {
        pub use super::macos::Component;
        pub(crate) use super::macos::{get_temperature, COMPONENTS_TEMPERATURE_IDS};
    } else {
        pub use super::ios::Component;
    }
}
