// Take a look at the license at the top of the repository in the LICENSE file.

use crate::ComponentExt;

use std::collections::HashMap;
use std::fs::{metadata, read_dir, File};
use std::io::Read;
use std::path::{Path, PathBuf};

#[doc = include_str!("../../md_doc/component.md")]
pub struct Component {
    temperature: f32,
    max: f32,
    critical: Option<f32>,
    label: String,
    input_file: PathBuf,
}

fn get_file_line(file: &Path, capacity: usize) -> Option<String> {
    let mut reader = String::with_capacity(capacity);
    if let Ok(mut f) = File::open(file) {
        if f.read_to_string(&mut reader).is_ok() {
            Some(reader)
        } else {
            None
        }
    } else {
        None
    }
}

fn is_file<T: AsRef<Path>>(path: T) -> bool {
    metadata(path).ok().map(|m| m.is_file()).unwrap_or(false)
}

/// Read out `hwmon` info (hardware monitor) from `folder` [Path]
/// for value path to check at refresh and files containing `max`,
/// `critical value and `label` and store this info in `components`.
///
/// What is read:
///
/// - Optionnal: sensor label, in the general case content of `tempN_label`
///   see below for special cases
/// - Optionnal: max value defined in `tempN_max`
/// - Optionnal: critical value defined in `tempN_crit`
///
/// Where `N` is a u32 associated to a sensor like `temp1_max`, `temp1_input`.
///
/// ## Special case: Disk drive with `drivetemp` module
///
/// For some crazy rational, hardware monitoring exposed by `drivetemp` has no label so the `label of a `Component`
/// deduced.
///
/// So the `device/model` content will be used instead of `tempN_label` content.
///
/// ## Special case: RasperryPy CPU and some vendor sensors
///
/// Have empty label, file exist but is empty, they will be labelled `Components N`.
///
/// ## Doc to Linux kernel API.
///
/// Kernel hwmon API: https://www.kernel.org/doc/html/latest/hwmon/hwmon-kernel-api.html
/// DriveTemp kernel API: https://docs.kernel.org/gpu/amdgpu/thermal.html#hwmon-interfaces
/// Amdgpu hwmon interface: https://www.kernel.org/doc/html/latest/hwmon/drivetemp.html
fn append_files(components: &mut Vec<Component>, folder: &Path) {
    let mut matchings: HashMap<u32, Vec<String>> = HashMap::with_capacity(10);

    if let Ok(dir) = read_dir(folder) {
        for entry in dir.flatten() {
            let entry = entry.path();
            if entry.is_dir()
                || !entry
                    .file_name()
                    .and_then(|x| x.to_str())
                    .unwrap_or("")
                    .starts_with("temp")
            {
                continue;
            }
            if let Some(entry) = entry.file_name() {
                if let Some(entry) = entry.to_str() {
                    let mut parts = entry.split('_');
                    if let Some(Some(id)) = parts.next().map(|s| s[4..].parse::<u32>().ok()) {
                        matchings
                            .entry(id)
                            .or_insert_with(|| Vec::with_capacity(5))
                            .push(
                                parts
                                    .next()
                                    .map(|s| format!("_{}", s))
                                    .unwrap_or_else(String::new),
                            );
                    }
                }
            }
        }
        for (key, val) in &matchings {
            let mut found_input = None;
            let mut found_label = None;
            for (pos, v) in val.iter().enumerate() {
                match v.as_str() {
                    // raspberry has empty string for temperature input
                    "_input" | "" => {
                        found_input = Some(pos);
                    }
                    "_label" => {
                        found_label = Some(pos);
                    }
                    _ => {}
                }
            }

            if let Some(found_input) = found_input {
                let mut p_label = folder.to_path_buf();
                let mut p_input = folder.to_path_buf();
                let mut p_crit = folder.to_path_buf();
                let mut p_max = folder.to_path_buf();

                // Disk have no label. Why? Don't know.
                // So we use the model name instead.
                let fragment_label = match found_label {
                    Some(_) => format!("temp{}_label", key),
                    None => "device/model".to_string(),
                };

                p_label.push(&fragment_label);
                p_input.push(&format!("temp{}{}", key, val[found_input]));
                p_max.push(&format!("temp{}_max", key));
                p_crit.push(&format!("temp{}_crit", key));
                if is_file(&p_input) {
                    let label = get_file_line(p_label.as_path(), 10)
                        .unwrap_or_else(|| format!("Component {}", key)) // needed for raspberry pi
                        .replace('\n', "");
                    let max = get_file_line(p_max.as_path(), 10).map(|max| {
                        max.replace('\n', "").parse::<f32>().unwrap_or(100_000f32) / 1000f32
                    });
                    let crit = get_file_line(p_crit.as_path(), 10).map(|crit| {
                        crit.replace('\n', "").parse::<f32>().unwrap_or(100_000f32) / 1000f32
                    });
                    components.push(Component::new(label, p_input.as_path(), max, crit));
                }
            }
        }
    }
}

impl Component {
    /// Creates a new component with the given information.
    pub(crate) fn new(
        label: String,
        input_path: &Path,
        max: Option<f32>,
        critical: Option<f32>,
    ) -> Component {
        let mut c = Component {
            temperature: 0f32,
            label,
            input_file: input_path.to_path_buf(),
            max: max.unwrap_or(0.0),
            critical,
        };
        c.refresh();
        c
    }
}

impl ComponentExt for Component {
    fn temperature(&self) -> f32 {
        self.temperature
    }

    fn max(&self) -> f32 {
        self.max
    }

    fn critical(&self) -> Option<f32> {
        self.critical
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn refresh(&mut self) {
        if let Some(content) = get_file_line(self.input_file.as_path(), 10) {
            self.temperature = content
                .replace('\n', "")
                .parse::<f32>()
                .unwrap_or(100_000f32)
                / 1000f32;
            if self.temperature > self.max {
                self.max = self.temperature;
            }
        }
    }
}

pub(crate) fn get_components() -> Vec<Component> {
    let mut components = Vec::with_capacity(10);
    if let Ok(dir) = read_dir(&Path::new("/sys/class/hwmon/")) {
        for entry in dir.flatten() {
            let entry = entry.path();
            if !entry.is_dir()
                || !entry
                    .file_name()
                    .and_then(|x| x.to_str())
                    .unwrap_or("")
                    .starts_with("hwmon")
            {
                continue;
            }
            append_files(&mut components, &entry);
        }
        components.sort_by(|c1, c2| c1.label.to_lowercase().cmp(&c2.label.to_lowercase()));
    }
    if is_file("/sys/class/thermal/thermal_zone0/temp") {
        // Specfic to raspberry pi.
        components.push(Component::new(
            "CPU".to_owned(),
            Path::new("/sys/class/thermal/thermal_zone0/temp"),
            None,
            None,
        ));
    }
    components
}
