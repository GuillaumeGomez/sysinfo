//
// Sysinfo
//
// Copyright (c) 2021 Guillaume Gomez
//

use crate::ComponentExt;

/// Dummy struct representing a component.
pub struct Component;

impl ComponentExt for Component {
    fn get_temperature(&self) -> f32 {
        0.0
    }

    fn get_max(&self) -> f32 {
        0.0
    }

    fn get_critical(&self) -> Option<f32> {
        None
    }

    fn get_label(&self) -> &str {
        ""
    }

    fn refresh(&mut self) {}
}
