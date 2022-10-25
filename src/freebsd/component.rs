// Take a look at the license at the top of the repository in the LICENSE file.

use super::utils::get_sys_value_by_name;
use crate::ComponentExt;

#[doc = include_str!("../../md_doc/component.md")]
pub struct Component {
    id: Vec<u8>,
    label: String,
    temperature: f32,
    max: f32,
}

impl ComponentExt for Component {
    fn temperature(&self) -> f32 {
        self.temperature
    }

    fn max(&self) -> f32 {
        self.max
    }

    fn critical(&self) -> Option<f32> {
        None
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn refresh(&mut self) {
        unsafe {
            if let Some(temperature) = refresh_component(&self.id) {
                self.temperature = temperature;
                if self.temperature > self.max {
                    self.max = self.temperature;
                }
            }
        }
    }
}

unsafe fn refresh_component(id: &[u8]) -> Option<f32> {
    let mut temperature: libc::c_int = 0;
    if !get_sys_value_by_name(id, &mut temperature) {
        None
    } else {
        // convert from Kelvin (x 10 -> 273.2 x 10) to Celsius
        Some((temperature - 2732) as f32 / 10.)
    }
}

pub unsafe fn get_components(nb_cpus: usize) -> Vec<Component> {
    // For now, we only have temperature for CPUs...
    let mut components = Vec::with_capacity(nb_cpus);

    for core in 0..nb_cpus {
        let id = format!("dev.cpu.{core}.temperature\0").as_bytes().to_vec();
        if let Some(temperature) = refresh_component(&id) {
            components.push(Component {
                id,
                label: format!("CPU {}", core + 1),
                temperature,
                max: temperature,
            });
        }
    }
    components
}
