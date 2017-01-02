// 
// Sysinfo
// 
// Copyright (c) 2017 Guillaume Gomez
//

pub struct Processor {
    name: String,
    cpu_usage: f32,
}

impl Processor {
    #[allow(dead_code)]
    fn new() -> Processor {
        Processor {
            name: String::new(),
            cpu_usage: 0f32,
        }
    }

    fn new_with_values(name: &str) -> Processor {
        Processor {
            name: name.to_owned(),
            cpu_usage: 0f32,
        }
    }

    fn set(&mut self) {
    }

    pub fn get_cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    pub fn get_name<'a>(&'a self) -> &'a str {
        &self.name
    }
}

pub fn create_processor(name: &str) -> Processor {
    Processor::new_with_values(name)
}
