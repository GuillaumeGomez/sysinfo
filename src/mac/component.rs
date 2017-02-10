// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

/// Struct containing a component information (temperature and name for the moment).
pub struct Component {
    /// Temperature is in celsius.
    pub temperature: f32,
    /// Temperature max value.
    pub max: f32,
    /// The highest temperature before the computer halts.
    pub critical: Option<f32>,
    /// Component's label.
    pub label: String,
}

impl Component {
    /// Creates a new `Component` with the given information.
    pub fn new(label: String, max: Option<f32>, critical: Option<f32>) -> Component {
        Component {
            temperature: 0f32,
            label: label,
            max: max.unwrap_or(0.0),
            critical: critical,
        }
    }
}

pub fn update_component(comp: &mut Component, temperature: f32) {
    comp.temperature = temperature;
    if comp.temperature > comp.max {
        comp.max = comp.temperature;
    }
}
