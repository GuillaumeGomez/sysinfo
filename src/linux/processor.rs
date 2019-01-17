// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

#![allow(clippy::too_many_arguments)]

use ::ProcessorExt;

/// Struct containing values to compute a CPU usage.
#[derive(Clone, Copy)]
pub struct CpuValues {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
    guest: u64,
    guest_nice: u64,
}

impl CpuValues {
    /// Creates a new instance of `CpuValues` with everything set to `0`.
    pub fn new() -> CpuValues {
        CpuValues {
            user: 0,
            nice: 0,
            system: 0,
            idle: 0,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
            guest_nice: 0,
        }
    }

    /// Creates a new instance of `CpuValues` with everything set to the corresponding argument.
    pub fn new_with_values(user: u64, nice: u64, system: u64, idle: u64, iowait: u64,
                           irq: u64, softirq: u64, steal: u64, guest: u64,
                           guest_nice: u64) -> CpuValues {
        CpuValues {
            user,
            nice,
            system,
            idle,
            iowait,
            irq,
            softirq,
            steal,
            guest,
            guest_nice,
        }
    }

    /*pub fn is_zero(&self) -> bool {
        self.user == 0 && self.nice == 0 && self.system == 0 && self.idle == 0 &&
        self.iowait == 0 && self.irq == 0 && self.softirq == 0 && self.steal == 0 &&
        self.guest == 0 && self.guest_nice == 0
    }*/

    /// Sets the given argument to the corresponding fields.
    pub fn set(&mut self, user: u64, nice: u64, system: u64, idle: u64, iowait: u64,
               irq: u64, softirq: u64, steal: u64, guest: u64, guest_nice: u64) {
        self.user = user;
        self.nice = nice;
        self.system = system;
        self.idle = idle;
        self.iowait = iowait;
        self.irq = irq;
        self.softirq = softirq;
        self.steal = steal;
        self.guest = guest;
        self.guest_nice = guest_nice;
    }

    /// Returns work time.
    pub fn work_time(&self) -> u64 {
        self.user + self.nice + self.system
    }

    /// Returns total time.
    pub fn total_time(&self) -> u64 {
        self.work_time() + self.idle + self.iowait + self.irq + self.softirq
            + self.steal + self.guest + self.guest_nice
    }
}

/// Struct containing a processor information.
pub struct Processor {
    old_values: CpuValues,
    new_values: CpuValues,
    name: String,
    cpu_usage: f32,
    total_time: u64,
    old_total_time: u64,
}

impl Processor {
    #[allow(dead_code)]
    fn new() -> Processor {
        Processor {
            name: String::new(),
            old_values: CpuValues::new(),
            new_values: CpuValues::new(),
            cpu_usage: 0f32,
            total_time: 0,
            old_total_time: 0,
        }
    }

    fn new_with_values(name: &str, user: u64, nice: u64, system: u64, idle: u64, iowait: u64,
                       irq: u64, softirq: u64, steal: u64, guest: u64,
                       guest_nice: u64) -> Processor {
        Processor {
            name: name.to_owned(),
            old_values: CpuValues::new(),
            new_values: CpuValues::new_with_values(user, nice, system, idle, iowait, irq,
                softirq, steal, guest, guest_nice),
            cpu_usage: 0f32,
            total_time: 0,
            old_total_time: 0,
        }
    }

    fn set(&mut self, user: u64, nice: u64, system: u64, idle: u64, iowait: u64,
           irq: u64, softirq: u64, steal: u64, guest: u64, guest_nice: u64) {
        fn min(a: u64, b: u64) -> f32 {
            (if a == b { 1 } else if a > b { a - b } else { b - a }) as f32
        }
        //if !self.new_values.is_zero() {
            self.old_values = self.new_values;
        //}
        self.new_values.set(user, nice, system, idle, iowait, irq, softirq, steal,
            guest, guest_nice);
        self.cpu_usage = min(self.new_values.work_time(), self.old_values.work_time()) /
            min(self.new_values.total_time(), self.old_values.total_time());
        self.old_total_time = self.old_values.total_time();
        self.total_time = self.new_values.total_time();
    }
}

impl ProcessorExt for Processor {
    fn get_cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    fn get_name(&self) -> &str {
        &self.name
    }
}

pub fn new_processor(name: &str, user: u64, nice: u64, system: u64, idle: u64, iowait: u64,
                     irq: u64, softirq: u64, steal: u64, guest: u64, guest_nice: u64) -> Processor {
    Processor::new_with_values(name, user, nice, system, idle, iowait, irq, softirq, steal,
        guest, guest_nice)
}

pub fn set_processor(p: &mut Processor, user: u64, nice: u64, system: u64, idle: u64, iowait: u64,
                     irq: u64, softirq: u64, steal: u64, guest: u64, guest_nice: u64) {
    p.set(user, nice, system, idle, iowait, irq, softirq, steal,
        guest, guest_nice)
}

pub fn get_raw_times(p: &Processor) -> (u64, u64) {
    (p.new_values.total_time(), p.old_values.total_time())
}
