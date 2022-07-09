// Take a look at the license at the top of the repository in the LICENSE file.

#[cfg(target_arch = "x86")]
pub(crate) mod x86;

#[cfg(target_arch = "x86")]
pub use self::x86::*;

#[cfg(target_arch = "aarch64")]
pub(crate) mod arm;

#[cfg(target_arch = "aarch64")]
pub use self::arm::*;