// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{Component, Error};

pub(crate) struct ComponentInner {
    pub(crate) updated: bool,
}

impl ComponentInner {
    pub(crate) fn temperature(&self) -> Option<f32> {
        None
    }

    pub(crate) fn max(&self) -> Option<f32> {
        None
    }

    pub(crate) fn critical(&self) -> Option<f32> {
        None
    }

    pub(crate) fn label(&self) -> &str {
        ""
    }

    pub(crate) fn id(&self) -> Option<&str> {
        None
    }

    pub(crate) fn refresh(&mut self) {}
}

pub(crate) struct ComponentsInner {
    pub(crate) components: Vec<Component>,
}

impl ComponentsInner {
    pub(crate) fn new() -> Result<Self, Error> {
        Err(Error::Unsupported)
    }

    pub(crate) fn list(&self) -> &[Component] {
        unreachable!()
    }

    pub(crate) fn list_mut(&mut self) -> &mut [Component] {
        unreachable!()
    }

    pub(crate) fn refresh(&mut self) {
        // Doesn't do anything.
    }
}
