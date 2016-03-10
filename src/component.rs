// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use std::collections::HashMap;
use std::fs::{File, read_dir};
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct Component {
    pub temperature: f32, // temperature is in celsius
    pub label: String,
    input_file: PathBuf,
    label_file: PathBuf,
}

fn append_files(components: &mut Vec<Component>, folder: &Path) {
    let mut matchings = HashMap::new();
    if let Ok(dir) = read_dir(folder) {
        for entry in dir {
            if let Ok(entry) = entry {
                let entry = entry.path();
                if entry.is_dir() || !entry.file_name().unwrap().to_str().unwrap_or("").starts_with("temp") {
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

                    p_label.push(&format!("temp{}_label", key));
                    p_input.push(&format!("temp{}_input", key));
                    components.push(Component::new(p_label.as_path(), p_input.as_path()));
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
                    if !entry.is_dir() || !entry.file_name().unwrap().to_str().unwrap_or("").starts_with("hwmon") {
                        continue;
                    }
                    append_files(&mut ret, &entry);
                }
            }
        }
        ret
    }

    pub fn new(label_path: &Path, input_path: &Path) -> Component {
        let mut c = Component {
            temperature: 0f32,
            label: String::new(),
            input_file: input_path.to_path_buf(),
            label_file: label_path.to_path_buf(),
        };
        c.update();
        c
    }

    pub fn update(&mut self) {
        let mut f = File::open(self.label_file.as_path()).unwrap();
        let mut reader = String::new();
        f.read_to_string(&mut reader).unwrap();
        self.label = reader.replace("\n", "");
        let mut f = File::open(self.input_file.as_path()).unwrap();
        f.read_to_string(&mut reader).unwrap();
        self.temperature = reader.replace("\n", "").parse::<f32>().unwrap() / 1000f32;
    }
}