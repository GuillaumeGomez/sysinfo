// Take a look at the license at the top of the repository in the LICENSE file.

use std::fs::{File, read_dir};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

use crate::{Component, Error};

const SENSOR_ROOT: &str = "/dev/sensors/temperature";

// Definitions from <sys/sensors.h>. They are not currently exposed by libc.
const SENSOR_KIND_TEMPERATURE: u64 = 0x01;

const SENSOR_UNIT_CELSIUS: u32 = 0x01;
const SENSOR_UNIT_FAHRENHEIT: u32 = 0x02;
const SENSOR_UNIT_KELVIN: u32 = 0x03;

const SENSOR_IOCTL: i32 = ('s' as i32) << 24 | ('e' as i32) << 16 | ('n' as i32) << 8;
const SENSOR_IOCTL_KIND: i32 = SENSOR_IOCTL | 0x01;
const SENSOR_IOCTL_SCALAR: i32 = SENSOR_IOCTL | 0x02;

#[repr(C)]
#[derive(Default)]
struct SensorIoctlKind {
    kind: u64,
    derive: u64,
}

#[repr(C)]
#[derive(Default)]
struct SensorIoctlScalar {
    unit: u32,
    granularity: i32,
    precision: u32,
    padding: u32,
    value: i64,
}

const _: [(); 16] = [(); std::mem::size_of::<SensorIoctlKind>()];
const _: [(); 24] = [(); std::mem::size_of::<SensorIoctlScalar>()];

pub(crate) struct ComponentInner {
    path: PathBuf,
    id: String,
    label: String,
    temperature: Option<f32>,
    max: Option<f32>,
    pub(crate) updated: bool,
}

impl ComponentInner {
    fn new(path: PathBuf, root: &Path) -> Option<Self> {
        let temperature = read_temperature(&path)?;
        let id = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        Some(Self {
            path,
            label: id.clone(),
            id,
            temperature: Some(temperature),
            max: Some(temperature),
            updated: true,
        })
    }

    pub(crate) fn temperature(&self) -> Option<f32> {
        self.temperature
    }

    pub(crate) fn max(&self) -> Option<f32> {
        self.max
    }

    pub(crate) fn critical(&self) -> Option<f32> {
        None
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) fn id(&self) -> Option<&str> {
        Some(&self.id)
    }

    pub(crate) fn refresh(&mut self) {
        self.temperature = read_temperature(&self.path);
        if let Some(temperature) = self.temperature {
            self.max = Some(self.max.map_or(temperature, |max| max.max(temperature)));
        }
    }
}

pub(crate) struct ComponentsInner {
    pub(crate) components: Vec<Component>,
}

impl ComponentsInner {
    pub(crate) fn new() -> Result<Self, Error> {
        Ok(Self {
            components: Vec::new(),
        })
    }

    pub(crate) fn list(&self) -> &[Component] {
        &self.components
    }

    pub(crate) fn list_mut(&mut self) -> &mut [Component] {
        &mut self.components
    }

    pub(crate) fn refresh(&mut self) {
        let root = Path::new(SENSOR_ROOT);
        let mut paths = Vec::new();
        sensor_paths(root, &mut paths);
        paths.sort_unstable();

        for path in paths {
            if let Some(component) = self
                .components
                .iter_mut()
                .find(|component| component.inner.path == path)
            {
                component.refresh();
                component.inner.updated = true;
            } else if let Some(inner) = ComponentInner::new(path, root) {
                self.components.push(Component { inner });
            }
        }
    }
}

fn sensor_paths(directory: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(file_type) if file_type.is_dir() => sensor_paths(&path, paths),
            Ok(_) => paths.push(path),
            Err(_) => {}
        }
    }
}

fn read_temperature(path: &Path) -> Option<f32> {
    let file = File::open(path).ok()?;
    let mut kind = SensorIoctlKind::default();
    // SAFETY: The file descriptor is valid for the duration of the call and `kind` has the exact
    // layout expected by SENSOR_IOCTL_KIND.
    if unsafe { libc::ioctl(file.as_raw_fd(), SENSOR_IOCTL_KIND, &mut kind) } != 0
        || kind.kind != SENSOR_KIND_TEMPERATURE
    {
        return None;
    }

    let mut scalar = SensorIoctlScalar::default();
    // SAFETY: The file descriptor is valid for the duration of the call and `scalar` has the exact
    // layout expected by SENSOR_IOCTL_SCALAR.
    if unsafe { libc::ioctl(file.as_raw_fd(), SENSOR_IOCTL_SCALAR, &mut scalar) } != 0 {
        return None;
    }
    temperature_from_scalar(&scalar)
}

fn temperature_from_scalar(scalar: &SensorIoctlScalar) -> Option<f32> {
    let mut value = scalar.value as f64;
    if scalar.granularity > 1 {
        value /= f64::from(scalar.granularity);
    } else if scalar.granularity < -1 {
        value *= f64::from(scalar.granularity).abs();
    }

    let celsius = match scalar.unit {
        SENSOR_UNIT_CELSIUS => value,
        SENSOR_UNIT_FAHRENHEIT => (value - 32.0) * 5.0 / 9.0,
        SENSOR_UNIT_KELVIN => value - 273.15,
        _ => return None,
    };
    let celsius = celsius as f32;
    celsius.is_finite().then_some(celsius)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar(unit: u32, granularity: i32, value: i64) -> SensorIoctlScalar {
        SensorIoctlScalar {
            unit,
            granularity,
            value,
            ..SensorIoctlScalar::default()
        }
    }

    #[test]
    fn converts_sensor_scalars_to_celsius() {
        assert_eq!(
            temperature_from_scalar(&scalar(SENSOR_UNIT_CELSIUS, 10, 425)),
            Some(42.5)
        );
        assert_eq!(
            temperature_from_scalar(&scalar(SENSOR_UNIT_CELSIUS, -2, 21)),
            Some(42.0)
        );
        assert_eq!(
            temperature_from_scalar(&scalar(SENSOR_UNIT_FAHRENHEIT, 1, 212)),
            Some(100.0)
        );
        assert_eq!(
            temperature_from_scalar(&scalar(SENSOR_UNIT_KELVIN, 100, 27315)),
            Some(0.0)
        );
        assert_eq!(temperature_from_scalar(&scalar(0, 1, 42)), None);
    }

    #[test]
    fn refreshes_component_list() {
        let mut components = crate::Components::new_with_refreshed_list().unwrap();
        assert!(has_unique_ids(&components));

        components.refresh(true);
        assert!(has_unique_ids(&components));
    }

    fn has_unique_ids(components: &[Component]) -> bool {
        components.iter().enumerate().all(|(index, component)| {
            component.id().is_some()
                && !components[..index]
                    .iter()
                    .any(|other| other.id() == component.id())
        })
    }
}
