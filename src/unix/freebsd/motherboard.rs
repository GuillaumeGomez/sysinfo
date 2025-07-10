// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::utils::get_kenv_var;

pub(crate) struct MotherboardInner;

impl MotherboardInner {
    pub(crate) fn new() -> Option<Self> {
        Some(Self)
    }

    pub(crate) fn asset_tag(&self) -> Option<String> {
        get_kenv_var(b"smbios.planar.tag\0")
    }

    pub(crate) fn name(&self) -> Option<String> {
        get_kenv_var(b"smbios.planar.product\0")
    }

    pub(crate) fn vendor(&self) -> Option<String> {
        get_kenv_var(b"smbios.planar.maker\0")
    }

    pub(crate) fn version(&self) -> Option<String> {
        get_kenv_var(b"smbios.planar.version\0")
    }

    pub(crate) fn serial(&self) -> Option<String> {
        get_kenv_var(b"smbios.planar.serial\0")
    }
}
