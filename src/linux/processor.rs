// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use std::fmt::{self, Formatter, Debug};

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

    pub fn new_with_values(user: u64, nice: u64, system: u64, idle: u64, iowait: u64,
                           irq: u64, softirq: u64, steal: u64, guest: u64,
                           guest_nice: u64) -> CpuValues {
        CpuValues {
            user: user,
            nice: nice,
            system: system,
            idle: idle,
            iowait: iowait,
            irq: irq,
            softirq: softirq,
            steal: steal,
            guest: guest,
            guest_nice: guest_nice
        }
    }

    /*pub fn is_zero(&self) -> bool {
        self.user == 0 && self.nice == 0 && self.system == 0 && self.idle == 0 &&
        self.iowait == 0 && self.irq == 0 && self.softirq == 0 && self.steal == 0 &&
        self.guest == 0 && self.guest_nice == 0
    }*/

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

    pub fn work_time(&self) -> u64 {
        self.user + self.nice + self.system
    }

    pub fn total_time(&self) -> u64 {
        self.work_time() + self.idle + self.iowait + self.irq + self.softirq
            + self.steal + self.guest + self.guest_nice
    }
}

pub struct Processor {
    old_values: CpuValues,
    new_values: CpuValues,
    name: String,
    cpu_usage: f32,
    total_time: u64,
    old_total_time: u64,
}

impl Processor {
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
            //if !self.new_values.is_zero() {
                self.old_values = self.new_values;
            //}
            self.new_values.set(user, nice, system, idle, iowait, irq, softirq, steal,
                guest, guest_nice);
            self.cpu_usage = (self.new_values.work_time() - self.old_values.work_time()) as f32 /
                (self.new_values.total_time() - self.old_values.total_time()) as f32;
            self.old_total_time = self.old_values.total_time();
            self.total_time = self.new_values.total_time();
    }

    pub fn get_cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    pub fn get_name<'a>(&'a self) -> &'a str {
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
