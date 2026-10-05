// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{GpuInner, GpusInner, PCIAddress};

use std::cmp::Ordering;

/// Type containing GPU information.
///
/// It is returned by [`Gpus`][crate::Gpus].
///
/// It is currently supported on Linux and macOS.
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

impl std::ops::Deref for Gpus {
    type Target = [Gpu];

    fn deref(&self) -> &Self::Target {
        self.list()
    }
}

impl<'a> IntoIterator for &'a Gpus {
    type Item = &'a Gpu;
    type IntoIter = std::slice::Iter<'a, Gpu>;

    fn into_iter(self) -> Self::IntoIter {
        self.list().iter()
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

impl PartialEq for Gpu {
    fn eq(&self, other: &Self) -> bool {
        self.pci_address() == other.pci_address()
    }
}

impl Eq for Gpu {}

impl PartialOrd for Gpu {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Gpu {
    fn cmp(&self, other: &Self) -> Ordering {
        self.pci_address().cmp(other.pci_address())
    }
}

impl Gpu {
    /// Returns the PCI Address of this GPU. Can be used as ID as it's unique to this GPU.
    pub fn pci_address(&self) -> &PCIAddress {
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
    ///
    /// On Windows, the first time you query this information, it'll return `None` as it is
    /// time-diff based.
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
