// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use std::collections::HashMap;
use std::fs::{File, read_dir};
use std::io::Read;
use std::path::{Path, PathBuf};

/// More information can be found at
/// http://lxr.free-electrons.com/source/Documentation/hwmon/sysfs-interface
pub struct Component {
    /// Temperature is in celsius.
    pub temperature: f32,
    /// Temperature max value.
    pub max: f32,
    /// The highest temperature before the computer halts.
    pub critical: Option<f32>,
    /// Component's label.
    pub label: String,
    input_file: PathBuf,
}

fn get_file_line(file: &Path) -> Option<String> {
    let mut reader = String::new();
    if let Ok(mut f) = File::open(file) {
        f.read_to_string(&mut reader).unwrap();
        Some(reader)
    } else {
        None
    }
}

fn append_files(components: &mut Vec<Component>, folder: &Path) {
    let mut matchings = HashMap::new();
    if let Ok(dir) = read_dir(folder) {
        for entry in dir {
            if let Ok(entry) = entry {
                let entry = entry.path();
                if entry.is_dir() || !entry.file_name().unwrap().to_str()
                                           .unwrap_or("").starts_with("temp") {
                    continue;
                }
                if let Some(entry) = entry.file_name() {
                    let id = entry.to_str().unwrap()[4..5].parse::<u32>().unwrap();
                    if !matchings.contains_key(&id) {
                        matchings.insert(id, vec!(entry.to_str().unwrap()[6..].to_owned()));
                    } else {
                        matchings.get_mut(&id).unwrap()
                                 .push(entry.to_str().unwrap()[6..].to_owned());
                    }
                }
            }
        }
        for (key, val) in &matchings {
            let mut found_input = None;
            let mut found_label = None;
            for (pos, v) in val.iter().enumerate() {
                match v.as_str() {
                    "input" => { found_input = Some(pos); }
                    "label" => { found_label = Some(pos); }
                    _ => {}
                }
                if found_label.is_some() && found_input.is_some() {
                    let mut p_label = folder.to_path_buf();
                    let mut p_input = folder.to_path_buf();
                    let mut p_crit = folder.to_path_buf();
                    let mut p_max = folder.to_path_buf();

                    p_label.push(&format!("temp{}_label", key));
                    p_input.push(&format!("temp{}_input", key));
                    p_max.push(&format!("temp{}_max", key));
                    p_crit.push(&format!("temp{}_crit", key));
                    let content = get_file_line(p_label.as_path()).unwrap();
                    let label = content.replace("\n", "");
                    let max = if let Some(max) = get_file_line(p_max.as_path()) {
                        Some(max.replace("\n", "").parse::<f32>().unwrap() / 1000f32)
                    } else {
                        None
                    };
                    let crit = if let Some(crit) = get_file_line(p_crit.as_path()) {
                        Some(crit.replace("\n", "").parse::<f32>().unwrap() / 1000f32)
                    } else {
                        None
                    };
                    components.push(Component::new(label, p_input.as_path(), max, crit));
                    break
                }
            }
        }
    }
}

impl Component {
    pub fn get_components() -> Vec<Component> {
        let mut ret = Vec::new();
        if let Ok(dir) = read_dir(&Path::new("/sys/class/hwmon/")) {
            for entry in dir {
                if let Ok(entry) = entry {
                    let entry = entry.path();
                    if !entry.is_dir() || !entry.file_name().unwrap().to_str()
                                                .unwrap_or("").starts_with("hwmon") {
                        continue;
                    }
                    append_files(&mut ret, &entry);
                }
            }
        }
        ret.sort_by(|c1, c2| c1.label.to_lowercase().cmp(&c2.label.to_lowercase()));
        ret
    }

    pub fn new(label: String, input_path: &Path, max: Option<f32>,
               critical: Option<f32>) -> Component {
        let mut c = Component {
            temperature: 0f32,
            label: label,
            input_file: input_path.to_path_buf(),
            max: max.unwrap_or(0.0),
            critical: critical,
        };
        c.update();
        c
    }

    pub fn update(&mut self) {
        let content = get_file_line(self.input_file.as_path()).unwrap();
        self.temperature = content.replace("\n", "").parse::<f32>().unwrap() / 1000f32;
        if self.temperature > self.max {
            self.max = self.temperature;
        }
    }
}
