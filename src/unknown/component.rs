//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

use crate::ComponentExt;

/// Dummy struct representing a component.
pub struct Component {}

impl ComponentExt for Component {
    fn temperature(&self) -> f32 {
        0.0
    }

    fn max(&self) -> f32 {
        0.0
    }

    fn critical(&self) -> Option<f32> {
        None
    }

    fn label(&self) -> &str {
        ""
    }

    fn refresh(&mut self) {}
}
