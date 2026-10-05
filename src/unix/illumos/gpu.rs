// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::CStr;
use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::{Gpu, PCI};

use super::ffi;

const CLASS_CODE_PROPERTY: &CStr = c"class-code";
const DEVICE_ID_PROPERTY: &CStr = c"device-id";
const MODEL_PROPERTY: &CStr = c"model";
const REG_PROPERTY: &CStr = c"reg";
const VENDOR_ID_PROPERTY: &CStr = c"vendor-id";

const PCI_ID_PATHS: &[&str] = &[
    "/usr/share/hwdata/pci.ids",
    "/usr/share/pci.ids",
    "/usr/share/lib/pci/pci.ids",
];

pub(crate) struct GpusInner {
    pub(crate) gpus: Vec<Gpu>,
}

pub(crate) struct GpuInner {
    pci: PCI,
    vendor: Option<String>,
    model: Option<String>,
    pub(crate) updated: bool,
}

impl GpuInner {
    pub(crate) fn pci(&self) -> &PCI {
        &self.pci
    }

    pub(crate) fn vendor(&self) -> Option<&str> {
        self.vendor.as_deref()
    }

    pub(crate) fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    pub(crate) fn usage(&self) -> Option<f32> {
        None
    }

    pub(crate) fn total_memory(&self) -> Option<u64> {
        None
    }

    pub(crate) fn used_memory(&self) -> Option<u64> {
        None
    }
}

impl GpusInner {
    pub(crate) fn new() -> Result<Self, crate::Error> {
        Ok(Self { gpus: Vec::new() })
    }

    pub(crate) fn refresh(&mut self) {
        let Some(snapshot) = DevInfoSnapshot::new() else {
            sysinfo_debug!("failed to create a libdevinfo snapshot");
            return;
        };

        snapshot.for_each_node(|node| {
            let node = DevInfoNode { node };
            let Some(class_code) = node.integer_property(CLASS_CODE_PROPERTY) else {
                return;
            };
            if !is_display_controller(class_code as u32) {
                return;
            }

            let Some(reg) = node.integer_property(REG_PROPERTY) else {
                return;
            };
            let pci = pci_from_reg(reg as u32);

            if let Some(gpu) = self.gpus.iter_mut().find(|gpu| gpu.inner.pci == pci) {
                gpu.inner.updated = true;
                return;
            }

            let vendor_id = node
                .integer_property(VENDOR_ID_PROPERTY)
                .map(|value| value as u32 & 0xffff);
            let device_id = node
                .integer_property(DEVICE_ID_PROPERTY)
                .map(|value| value as u32 & 0xffff);
            let (database_vendor, database_model) = vendor_id
                .zip(device_id)
                .map_or((None, None), |(vendor, device)| pci_names(vendor, device));
            let vendor = database_vendor.or_else(|| {
                vendor_id
                    .and_then(crate::utils::gpu_vendor_name)
                    .map(str::to_owned)
            });
            let model = database_model.or_else(|| node.string_property(MODEL_PROPERTY));

            self.gpus.push(Gpu {
                inner: GpuInner {
                    pci,
                    vendor,
                    model,
                    updated: true,
                },
            });
        });
    }
}

struct DevInfoSnapshot {
    root: *mut ffi::DevInfoNode,
}

impl DevInfoSnapshot {
    fn new() -> Option<Self> {
        // SAFETY: The path is NUL-terminated and the returned snapshot handle is checked before
        // use.
        let root = unsafe { ffi::di_init(c"/".as_ptr(), ffi::DINFO_SUBTREE | ffi::DINFO_PROP) };
        (!root.is_null()).then_some(Self { root })
    }

    fn for_each_node(&self, mut callback: impl FnMut(*mut ffi::DevInfoNode)) {
        let mut nodes = vec![self.root];
        while let Some(node) = nodes.pop() {
            callback(node);

            // SAFETY: `node` belongs to this live snapshot. Child and sibling pointers, when
            // present, are owned by the same snapshot.
            let sibling = unsafe { ffi::di_sibling_node(node) };
            if !sibling.is_null() {
                nodes.push(sibling);
            }
            // SAFETY: Same as above.
            let child = unsafe { ffi::di_child_node(node) };
            if !child.is_null() {
                nodes.push(child);
            }
        }
    }
}

impl Drop for DevInfoSnapshot {
    fn drop(&mut self) {
        // SAFETY: `root` was returned by `di_init` and is finalized exactly once here.
        unsafe { ffi::di_fini(self.root) };
    }
}

struct DevInfoNode {
    node: *mut ffi::DevInfoNode,
}

impl DevInfoNode {
    fn integer_property(&self, name: &CStr) -> Option<i32> {
        let mut value = std::ptr::null_mut();
        // SAFETY: The node and property name are valid, and `value` points to writable storage.
        let count = unsafe {
            ffi::di_prop_lookup_ints(ffi::DDI_DEV_T_ANY, self.node, name.as_ptr(), &mut value)
        };
        if count < 1 || value.is_null() {
            None
        } else {
            // SAFETY: A positive count means libdevinfo returned at least one integer. The data is
            // owned by the snapshot and remains valid for its lifetime.
            Some(unsafe { *value })
        }
    }

    fn string_property(&self, name: &CStr) -> Option<String> {
        let mut value = std::ptr::null_mut();
        // SAFETY: The node and property name are valid, and `value` points to writable storage.
        let count = unsafe {
            ffi::di_prop_lookup_strings(ffi::DDI_DEV_T_ANY, self.node, name.as_ptr(), &mut value)
        };
        if count < 1 || value.is_null() {
            None
        } else {
            // SAFETY: A positive count means libdevinfo returned a NUL-terminated string. The data
            // is owned by the snapshot and remains valid for its lifetime.
            Some(
                unsafe { CStr::from_ptr(value) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }
}

fn is_display_controller(class_code: u32) -> bool {
    class_code & 0xff0000 == 0x030000
}

fn pci_from_reg(reg: u32) -> PCI {
    PCI {
        domain: 0,
        bus: (reg >> 16) & 0xff,
        device: (reg >> 11) & 0x1f,
        function: (reg >> 8) & 0x7,
    }
}

fn pci_names(vendor_id: u32, device_id: u32) -> (Option<String>, Option<String>) {
    for path in PCI_ID_PATHS {
        let Ok(file) = File::open(path) else {
            continue;
        };
        let names = parse_pci_names(BufReader::new(file), vendor_id, device_id);
        if names.0.is_some() || names.1.is_some() {
            return names;
        }
    }
    (None, None)
}

fn parse_pci_names(
    reader: impl BufRead,
    vendor_id: u32,
    device_id: u32,
) -> (Option<String>, Option<String>) {
    let mut vendor = None;
    let mut in_vendor = false;

    for line in reader.lines().map_while(Result::ok) {
        if line.starts_with("C ") {
            break;
        }
        if !line.starts_with(char::is_whitespace) {
            if in_vendor {
                break;
            }
            let Some((id, name)) = parse_id_and_name(&line) else {
                continue;
            };
            if id == vendor_id {
                vendor = Some(name.to_owned());
                in_vendor = true;
            }
        } else if in_vendor && line.starts_with('\t') && !line.starts_with("\t\t") {
            let Some((id, name)) = parse_id_and_name(line.trim_start()) else {
                continue;
            };
            if id == device_id {
                return (vendor, Some(name.to_owned()));
            }
        }
    }
    (vendor, None)
}

fn parse_id_and_name(line: &str) -> Option<(u32, &str)> {
    let (id, name) = line.split_once(char::is_whitespace)?;
    Some((u32::from_str_radix(id, 16).ok()?, name.trim_start()))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn display_controller_class() {
        assert!(is_display_controller(0x030000));
        assert!(is_display_controller(0x030200));
        assert!(!is_display_controller(0x020000));
    }

    #[test]
    fn pci_location_from_reg() {
        assert_eq!(
            pci_from_reg(0x00001800),
            PCI {
                domain: 0,
                bus: 0,
                device: 3,
                function: 0,
            }
        );
        assert_eq!(
            pci_from_reg(0x00122b00),
            PCI {
                domain: 0,
                bus: 0x12,
                device: 5,
                function: 3,
            }
        );
    }

    #[test]
    fn pci_id_names() {
        let data = b"\
1234  Example Devices, Inc.\n\
\t1111  Example GPU\n\
\t\t1234 5678  Example subsystem\n\
\t2222  Another GPU\n\
abcd  Other Vendor\n\
C 00  Unclassified device\n";

        assert_eq!(
            parse_pci_names(Cursor::new(data), 0x1234, 0x1111),
            (
                Some("Example Devices, Inc.".to_owned()),
                Some("Example GPU".to_owned()),
            )
        );
        assert_eq!(
            parse_pci_names(Cursor::new(data), 0x1234, 0x9999),
            (Some("Example Devices, Inc.".to_owned()), None)
        );
        assert_eq!(
            parse_pci_names(Cursor::new(data), 0xffff, 0x1111),
            (None, None)
        );
    }

    #[test]
    fn refreshes_gpu_list() {
        let mut gpus = crate::Gpus::new_with_refreshed_list().unwrap();
        assert!(
            gpus.iter()
                .enumerate()
                .all(|(index, gpu)| !gpus[..index].contains(gpu))
        );

        gpus.refresh(true);
        assert!(
            gpus.iter()
                .enumerate()
                .all(|(index, gpu)| !gpus[..index].contains(gpu))
        );
    }
}
