// Take a look at the license at the top of the repository in the LICENSE file.

use crate::ComponentExt;

#[doc = include_str!("../../md_doc/component.md")]
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
