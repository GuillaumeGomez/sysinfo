//
// Sysinfo
//
// Copyright (c) 2021 Guillaume Gomez
//

cfg_if! {
    if #[cfg(target_os = "macos")] {
        mod macos;
        pub use self::macos::Component;
        pub(crate) use self::macos::{get_temperature, COMPONENTS_TEMPERATURE_IDS};
    } else {
        mod ios;
        pub use self::ios::Component;
    }
}
