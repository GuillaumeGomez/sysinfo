// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{Component, ComponentsExt};

pub(crate) struct ComponentInner;

impl ComponentInner {
    pub(crate) fn temperature(&self) -> f32 {
        0.0
    }

    pub(crate) fn max(&self) -> f32 {
        0.0
    }

    pub(crate) fn critical(&self) -> Option<f32> {
        None
    }

    pub(crate) fn label(&self) -> &str {
        ""
    }

    pub(crate) fn refresh(&mut self) {}
}

#[doc = include_str!("../../../../md_doc/components.md")]
pub struct Components {
    components: Vec<Component>,
}

impl ComponentsExt for Components {
    fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    fn components(&self) -> &[Component] {
        &self.components
    }

    fn components_mut(&mut self) -> &mut [Component] {
        &mut self.components
    }

    fn refresh_list(&mut self) {
        // Doesn't do anything.
    }
}
