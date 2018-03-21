// 
// Sysinfo
// 
// Copyright (c) 2018 Guillaume Gomez
//

use ComponentExt;

/// Struct containing a component information (temperature and name for the moment).
pub struct Component {
    temperature: f32,
    max: f32,
    critical: Option<f32>,
    label: String,
}

impl Component {
    /// Creates a new `Component` with the given information.
    pub fn new(label: String, max: Option<f32>, critical: Option<f32>) -> Component {
        Component {
            temperature: 0f32,
            label,
            max: max.unwrap_or(0.0),
            critical,
        }
    }
}

impl ComponentExt for Component {
    fn get_temperature(&self) -> f32 {
        self.temperature
    }

    fn get_max(&self) -> f32 {
        self.max
    }

    fn get_critical(&self) -> Option<f32> {
        self.critical
    }

    fn get_label(&self) -> &str {
        &self.label
    }
}

pub fn update_component(comp: &mut Component, temperature: f32) {
    comp.temperature = temperature;
    if comp.temperature > comp.max {
        comp.max = comp.temperature;
    }
}
