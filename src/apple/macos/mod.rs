//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

mod component;
mod disk;

pub use self::component::Component;
pub(crate) use self::component::{get_temperature, COMPONENTS_TEMPERATURE_IDS};
pub(crate) use self::disk::get_disks;

pub(crate) mod ffi;
