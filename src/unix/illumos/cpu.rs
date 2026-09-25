// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::CStr;
use std::ops::RangeInclusive;

use crate::{Cpu, CpuRefreshKind};

use super::kstat::KstatReader;

const CPU_MODULE: &CStr = c"cpu";
const CPU_INFO_MODULE: &CStr = c"cpu_info";
const CPU_SYS_NAME: &CStr = c"sys";

pub(super) fn possible_cpu_ids() -> RangeInclusive<i32> {
    let max_id =
        unsafe { libc::sysconf(libc::_SC_CPUID_MAX) }.clamp(-1, i32::MAX as libc::c_long) as i32;
    0..=max_id
}

pub(crate) struct CpusWrapper {
    pub(crate) global_cpu_usage: f32,
    pub(crate) cpus: Vec<Cpu>,
    cpu_ids: Vec<i32>,
    old_times: Vec<CpuTimes>,
    got_frequency: bool,
}

#[derive(Clone, Copy, Default)]
struct CpuTimes {
    user: u64,
    kernel: u64,
    wait: u64,
    idle: u64,
}

impl CpuTimes {
    fn usage_since(self, old: Self) -> f32 {
        let user = self.user.saturating_sub(old.user);
        let kernel = self.kernel.saturating_sub(old.kernel);
        let wait = self.wait.saturating_sub(old.wait);
        let idle = self.idle.saturating_sub(old.idle);
        let total = user
            .saturating_add(kernel)
            .saturating_add(wait)
            .saturating_add(idle);
        if total == 0 {
            0.
        } else {
            (user.saturating_add(kernel) as f32 * 100.) / total as f32
        }
    }

    fn add(&mut self, other: Self) {
        self.user = self.user.saturating_add(other.user);
        self.kernel = self.kernel.saturating_add(other.kernel);
        self.wait = self.wait.saturating_add(other.wait);
        self.idle = self.idle.saturating_add(other.idle);
    }
}

impl CpusWrapper {
    pub(crate) fn new() -> Self {
        Self {
            global_cpu_usage: 0.,
            cpus: Vec::new(),
            cpu_ids: Vec::new(),
            old_times: Vec::new(),
            got_frequency: false,
        }
    }

    pub(crate) fn refresh(&mut self, refresh_kind: CpuRefreshKind) {
        let Some(kstat) = KstatReader::new() else {
            sysinfo_debug!("failed to open libkstat");
            return;
        };

        if self.cpus.is_empty() {
            self.populate(&kstat, refresh_kind.frequency());
        } else if refresh_kind.frequency() && !self.got_frequency {
            self.refresh_frequencies(&kstat);
        }

        if refresh_kind.usage() {
            self.refresh_usage(&kstat);
        }
    }

    fn populate(&mut self, kstat: &KstatReader, include_frequency: bool) {
        for id in possible_cpu_ids() {
            let Some(info) = kstat.lookup(Some(CPU_INFO_MODULE), id, None) else {
                continue;
            };
            let vendor_id = info
                .string("vendor_id")
                .unwrap_or_else(|| "unknown".to_owned());
            let brand = info
                .string("brand")
                .or_else(|| info.string("implementation"))
                .unwrap_or_else(|| "unknown".to_owned());
            let frequency = if include_frequency {
                info.integer("clock_MHz").unwrap_or(0)
            } else {
                0
            };
            self.cpu_ids.push(id);
            self.old_times.push(CpuTimes::default());
            self.cpus.push(Cpu {
                inner: CpuInner {
                    cpu_usage: 0.,
                    name: format!("cpu {id}"),
                    vendor_id,
                    brand,
                    frequency,
                },
            });
        }
        self.got_frequency = include_frequency;
    }

    fn refresh_frequencies(&mut self, kstat: &KstatReader) {
        for (&id, cpu) in self.cpu_ids.iter().zip(&mut self.cpus) {
            if let Some(info) = kstat.lookup(Some(CPU_INFO_MODULE), id, None) {
                cpu.inner.frequency = info.integer("clock_MHz").unwrap_or(0);
            }
        }
        self.got_frequency = true;
    }

    fn refresh_usage(&mut self, kstat: &KstatReader) {
        let mut global_new = CpuTimes::default();
        let mut global_old = CpuTimes::default();

        for ((&id, old), cpu) in self
            .cpu_ids
            .iter()
            .zip(&mut self.old_times)
            .zip(&mut self.cpus)
        {
            let Some(stat) = kstat.lookup(Some(CPU_MODULE), id, Some(CPU_SYS_NAME)) else {
                continue;
            };
            let new = CpuTimes {
                user: stat.integer("cpu_ticks_user").unwrap_or(0),
                kernel: stat.integer("cpu_ticks_kernel").unwrap_or(0),
                wait: stat.integer("cpu_ticks_wait").unwrap_or(0),
                idle: stat.integer("cpu_ticks_idle").unwrap_or(0),
            };
            cpu.inner.cpu_usage = new.usage_since(*old);
            global_new.add(new);
            global_old.add(*old);
            *old = new;
        }
        self.global_cpu_usage = global_new.usage_since(global_old);
    }
}

pub(crate) struct CpuInner {
    pub(crate) cpu_usage: f32,
    name: String,
    pub(crate) vendor_id: String,
    pub(crate) brand: String,
    pub(crate) frequency: u64,
}

impl CpuInner {
    pub(crate) fn cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn frequency(&self) -> u64 {
        self.frequency
    }

    pub(crate) fn vendor_id(&self) -> &str {
        &self.vendor_id
    }

    pub(crate) fn brand(&self) -> &str {
        &self.brand
    }
}
