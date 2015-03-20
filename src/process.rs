use std::fmt::{self, Formatter, Debug};
use libc::{c_int};

#[derive(Copy)]
pub struct CpuValues {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64
}

impl CpuValues {
    pub fn new() -> CpuValues {
        CpuValues {
            user: 0,
            nice: 0,
            system: 0,
            idle: 0
        }
    }

    pub fn new_with_values(user: u64, nice: u64, system: u64, idle: u64) -> CpuValues {
        CpuValues {
            user: user,
            nice: nice,
            system: system,
            idle: idle
        }
    }

    pub fn is_zero(&self) -> bool {
        self.user == 0 && self.nice == 0 && self.system == 0 && self.idle == 0
    }

    pub fn set(&mut self, user: u64, nice: u64, system: u64, idle: u64) {
        self.user = user;
        self.nice = nice;
        self.system = system;
        self.idle = idle;
    }
}

pub struct Process {
    old_values: CpuValues,
    new_values: CpuValues,
    name: String,
    cpu_usage: f32
}

impl Process {
    fn new() -> Process {
        Process {
            name: String::new(),
            old_values: CpuValues::new(),
            new_values: CpuValues::new(),
            cpu_usage: 0f32
        }
    }

    fn new_with_values(name: &str, user: u64, nice: u64, system: u64, idle: u64) -> Process {
        Process {
            name: String::from_str(name),
            old_values: CpuValues::new_with_values(user, nice, system, idle),
            new_values: CpuValues::new(),
            cpu_usage: 0f32
        }
    }

    fn set(&mut self, user: u64, nice: u64, system: u64, idle: u64) {
        if self.old_values.is_zero() {
            self.old_values.set(user, nice, system, idle);
        } else {
            if !self.new_values.is_zero() {
                self.old_values = self.new_values;
            }
            self.new_values.set(user, nice, system, idle);
            self.cpu_usage = ((self.new_values.user + self.new_values.nice + self.new_values.system) -
                (self.old_values.user + self.old_values.nice + self.old_values.system)) as f32 /
                ((self.new_values.user + self.new_values.nice + self.new_values.system + self.new_values.idle) -
                (self.old_values.user + self.old_values.nice + self.old_values.system + self.old_values.idle)) as f32;
        }
    }

    pub fn get_cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    pub fn get_name<'a>(&'a self) -> &'a str {
        self.name.as_slice()
    }
}

impl Debug for Process {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}: {}%", self.name, self.cpu_usage)
    }
}

pub fn new_process(name: &str, user: u64, nice: u64, system: u64, idle: u64) -> Process {
    Process::new_with_values(name, user, nice, system, idle)
}

pub fn set_process(p: &mut Process, user: u64, nice: u64, system: u64, idle: u64) {
    p.set(user, nice, system, idle)
}