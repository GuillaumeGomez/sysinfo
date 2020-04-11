//
// Sysinfo
//
// Copyright (c) 2018 Guillaume Gomez
//

use ComponentExt;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{metadata, read_dir, File};
use std::io::Read;
use std::path::{Path, PathBuf};

/// More information can be found at [kernel.org][k].
///
/// Note: these may not be present on virtual Linux systems, such as **Docker**
/// or **Windows Subsystem for Linux**. These hosts do not expose this information
/// and therefore `Component` elements may be missing or not as expected.
///
/// [k]: https://www.kernel.org/doc/Documentation/hwmon/sysfs-interface
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
    metadata(path)
        .ok()
        .map(|m| m.is_file())
        .unwrap_or_else(|| false)
}

fn append_files(components: &mut Vec<Component>, folder: &Path) {
    let mut matchings: HashMap<u32, Vec<String>> = HashMap::with_capacity(10);

    if let Ok(dir) = read_dir(folder) {
        for entry in dir {
            if let Ok(entry) = entry {
                let entry = entry.path();
                if entry.is_dir()
                    || !entry
                        .file_name()
                        .unwrap_or_else(|| OsStr::new("/"))
                        .to_str()
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
            if let (Some(_), Some(found_input)) = (found_label, found_input) {
                let mut p_label = folder.to_path_buf();
                let mut p_input = folder.to_path_buf();
                let mut p_crit = folder.to_path_buf();
                let mut p_max = folder.to_path_buf();

                p_label.push(&format!("temp{}_label", key));
                p_input.push(&format!("temp{}{}", key, val[found_input]));
                p_max.push(&format!("temp{}_max", key));
                p_crit.push(&format!("temp{}_crit", key));
                if is_file(&p_input) {
                    let label = get_file_line(p_label.as_path(), 10)
                        .unwrap_or_else(|| format!("Component {}", key)) // needed for raspberry pi
                        .replace("\n", "");
                    let max = if let Some(max) = get_file_line(p_max.as_path(), 10) {
                        Some(max.replace("\n", "").parse::<f32>().unwrap_or(100_000f32) / 1000f32)
                    } else {
                        None
                    };
                    let crit = if let Some(crit) = get_file_line(p_crit.as_path(), 10) {
                        Some(crit.replace("\n", "").parse::<f32>().unwrap_or(100_000f32) / 1000f32)
                    } else {
                        None
                    };
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
    fn get_temperature(&self) -> f32 {
        self.temperature
    }

    fn get_max(&self) -> f32 {
        self.max
    }

    fn get_critical(&self) -> Option<f32> {
        self.critical
    }

    fn get_label(&self) -> &str {
        &self.label
    }

    fn refresh(&mut self) {
        if let Some(content) = get_file_line(self.input_file.as_path(), 10) {
            self.temperature = content
                .replace("\n", "")
                .parse::<f32>()
                .unwrap_or(100_000f32)
                / 1000f32;
            if self.temperature > self.max {
                self.max = self.temperature;
            }
        }
    }
}

pub fn get_components() -> Vec<Component> {
    if let Ok(dir) = read_dir(&Path::new("/sys/class/hwmon/")) {
        let mut components = Vec::with_capacity(10);

        for entry in dir {
            if let Ok(entry) = entry {
                let entry = entry.path();
                if !entry.is_dir()
                    || !entry
                        .file_name()
                        .unwrap_or_else(|| OsStr::new("/"))
                        .to_str()
                        .unwrap_or("")
                        .starts_with("hwmon")
                {
                    continue;
                }
                append_files(&mut components, &entry);
            }
        }
        components.sort_by(|c1, c2| c1.label.to_lowercase().cmp(&c2.label.to_lowercase()));
        components
    } else if is_file("/sys/class/thermal/thermal_zone0/temp") {
        // Specfic to raspberry pi.
        vec![Component::new(
            "CPU".to_owned(),
            Path::new("/sys/class/thermal/thermal_zone0/temp"),
            None,
            None,
        )]
    } else {
        Vec::new()
    }
}
