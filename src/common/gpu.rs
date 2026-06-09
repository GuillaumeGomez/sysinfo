// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{GpuInner, GpusInner};
use std::fmt;

/// Type containing GPU information.
///
/// It is returned by [`Gpus`][crate::Gpus].
///
/// It is currently supported on Linux and macOS.
///
/// ```no_run
/// use sysinfo::Gpus;
///
/// let gpus = Gpus::new_with_refreshed_list();
/// for gpu in gpus.list() {
///     println!("{gpu:?}");
/// }
/// ```
pub struct Gpus {
    pub(crate) inner: GpusInner,
}

impl Gpus {
    /// Creates a new empty [`Gpus`][crate::Gpus] type.
    ///
    /// If you want it to be filled directly, take a look at [`Gpus::new_with_refreshed_list`].
    ///
    /// ```no_run
    /// use sysinfo::Gpus;
    ///
    /// if let Ok(mut gpus) = Gpus::new() {
    ///     gpus.refresh(true);
    ///     for gpu in gpus.list() {
    ///         println!("{gpu:?}");
    ///     }
    /// }
    /// ```
    pub fn new() -> Result<Self, crate::Error> {
        Ok(Self {
            inner: GpusInner::new()?,
        })
    }

    /// Creates a new [`Gpus`][crate::Gpus] type with the GPU list loaded.
    ///
    /// ```no_run
    /// use sysinfo::Gpus;
    ///
    /// if let Ok(gpus) = Gpus::new_with_refreshed_list() {
    ///     for gpu in gpus.list() {
    ///         println!("{gpu:?}");
    ///     }
    /// }
    /// ```
    pub fn new_with_refreshed_list() -> Result<Self, crate::Error> {
        let mut gpus = Self::new()?;
        gpus.refresh(false);
        Ok(gpus)
    }

    /// Returns the list of GPUs.
    ///
    /// ```no_run
    /// use sysinfo::Gpus;
    ///
    /// if let Ok(gpus) = Gpus::new_with_refreshed_list() {
    ///     for gpu in gpus.list() {
    ///         println!("{gpu:?}");
    ///     }
    /// }
    /// ```
    pub fn list(&self) -> &[Gpu] {
        &self.inner.gpus
    }

    /// Refreshes the list of GPUs.
    ///
    /// ```no_run
    /// use sysinfo::Gpus;
    ///
    /// if let Ok(mut gpus) = Gpus::new_with_refreshed_list() {
    ///     // Wait some time...? Then refresh the data of each GPU.
    ///     gpus.refresh(true);
    /// }
    /// ```
    pub fn refresh(&mut self, remove_not_listed_gpus: bool) {
        self.inner.refresh();
        if remove_not_listed_gpus {
            self.inner.gpus.retain_mut(|c| {
                if !c.inner.updated {
                    return false;
                }
                c.inner.updated = false;
                true
            });
        } else {
            for gpu in &mut self.inner.gpus {
                gpu.inner.updated = false;
            }
        }
    }
}

/// A PCI (Peripheral Component Interconnect) is an architecture used to identify and manage
/// hardware devices.
///
/// It is returned by [`Gpu::pci`][crate::Gpu::pci].
///
/// If you want to understand in details what a PCI is, I recommend:
/// <https://en.wikipedia.org/wiki/Peripheral_Component_Interconnect>.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct PCI {
    /// A PCI domain, also called "segment".
    pub domain: u32,
    /// A PCI bus.
    pub bus: u32,
    /// A PCI device.
    pub device: u32,
    /// A PCI function.
    pub function: u32,
}

impl fmt::Display for PCI {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self {
            domain,
            bus,
            device,
            function,
        } = self;
        write!(f, "{domain:04x}:{bus:02x}:{device:02x}.{function}")
    }
}

impl core::str::FromStr for PCI {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn get_next_u32<'a>(
            iter: &mut impl Iterator<Item = &'a str>,
            missing_msg: &'static str,
            invalid_msg: &'static str,
        ) -> Result<u32, &'static str> {
            let Some(value) = iter.next() else {
                return Err(missing_msg);
            };
            value.parse::<u32>().map_err(|_| invalid_msg)
        }

        let mut iter = s.split(':');
        let domain = get_next_u32(&mut iter, "missing domain", "invalid domain")?;
        let bus = get_next_u32(&mut iter, "missing bus", "invalid bus")?;
        let Some(last) = iter.next() else {
            return Err("missing device");
        };
        if iter.next().is_some() {
            return Err("unexpected `:` after bus");
        };
        let mut iter = last.split('.');
        let device = get_next_u32(&mut iter, "missing device", "invalid device")?;
        let function = get_next_u32(&mut iter, "missing function", "invalid function")?;
        if iter.next().is_some() {
            return Err("unexpected `:` after function");
        };

        Ok(Self {
            domain,
            bus,
            device,
            function,
        })
    }
}

/// Type containing GPU information.
///
/// It is returned by [`Gpus`][crate::Gpus].
///
/// ```no_run
/// use sysinfo::Gpus;
///
/// if let Ok(gpus) = Gpus::new_with_refreshed_list() {
///     for gpu in gpus.list() {
///         println!("{gpu:?}");
///     }
/// }
/// ```
pub struct Gpu {
    pub(crate) inner: GpuInner,
}

impl Gpu {
    /// Returns the PCI of this GPU. Can be used as ID as it's unique to this GPU.
    pub fn pci(&self) -> &PCI {
        self.inner.pci()
    }

    /// Returns the name of the vendor of this GPU. Returns `None` if the information cannot be
    /// retrieved.
    pub fn vendor(&self) -> Option<&str> {
        self.inner.vendor()
    }

    /// Returns the model of this GPU. Returns `None` if the information cannot be retrieved.
    pub fn model(&self) -> Option<&str> {
        self.inner.model()
    }

    /// Returns the % usage of this GPU.
    ///
    /// On Linux, it's generally very tricky to get this information, as such, only some GPUs
    /// like NVIDIA's or AMD's provide this information.
    ///
    /// On macOS, the information should always be available.
    pub fn usage(&self) -> Option<f32> {
        self.inner.usage()
    }

    /// Returns the total VRAM of this GPU in bytes.
    ///
    /// ⚠️ It does not take into account the GPU memory that is shared with the system's RAM.
    ///
    /// On macOS, GPUs share memory with the system, it always returns `None`.
    ///
    /// Returns `None` if the information cannot be retrieved.
    pub fn total_memory(&self) -> Option<u64> {
        self.inner.total_memory()
    }

    /// Returns the used VRAM of this GPU in bytes.
    ///
    /// ⚠️ It does not take into account the GPU memory that is shared with the system's RAM.
    ///
    /// On macOS, GPUs share memory with the system, it always returns `None`.
    ///
    /// Returns `None` if the information cannot be retrieved.
    pub fn used_memory(&self) -> Option<u64> {
        self.inner.used_memory()
    }
}
