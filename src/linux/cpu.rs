// Take a look at the license at the top of the repository in the LICENSE file.

#![allow(clippy::too_many_arguments)]

use std::collections::HashSet;
use std::fs::File;
use std::io::Read;

use crate::CpuExt;

/// Struct containing values to compute a CPU usage.
#[derive(Clone, Copy)]
pub(crate) struct CpuValues {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
    _guest: u64,
    _guest_nice: u64,
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
            _guest: 0,
            _guest_nice: 0,
        }
    }

    /// Creates a new instance of `CpuValues` with everything set to the corresponding argument.
    pub fn new_with_values(
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
    ) -> CpuValues {
        CpuValues {
            user,
            nice,
            system,
            idle,
            iowait,
            irq,
            softirq,
            steal,
            _guest: guest,
            _guest_nice: guest_nice,
        }
    }

    /*pub fn is_zero(&self) -> bool {
        self.user == 0 && self.nice == 0 && self.system == 0 && self.idle == 0 &&
        self.iowait == 0 && self.irq == 0 && self.softirq == 0 && self.steal == 0 &&
        self.guest == 0 && self.guest_nice == 0
    }*/

    /// Sets the given argument to the corresponding fields.
    pub fn set(
        &mut self,
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
    ) {
        self.user = user;
        self.nice = nice;
        self.system = system;
        self.idle = idle;
        self.iowait = iowait;
        self.irq = irq;
        self.softirq = softirq;
        self.steal = steal;
        self._guest = guest;
        self._guest_nice = guest_nice;
    }

    /// Returns work time.
    pub fn work_time(&self) -> u64 {
        self.user
            .saturating_add(self.nice)
            .saturating_add(self.system)
            .saturating_add(self.irq)
            .saturating_add(self.softirq)
            .saturating_add(self.steal)
    }

    /// Returns total time.
    pub fn total_time(&self) -> u64 {
        // `guest` is already included in `user`
        // `guest_nice` is already included in `nice`
        self.work_time()
            .saturating_add(self.idle)
            .saturating_add(self.iowait)
    }
}

#[doc = include_str!("../../md_doc/cpu.md")]
pub struct Cpu {
    old_values: CpuValues,
    new_values: CpuValues,
    pub(crate) name: String,
    cpu_usage: f32,
    total_time: u64,
    old_total_time: u64,
    pub(crate) frequency: u64,
    pub(crate) vendor_id: String,
    pub(crate) brand: String,
}

impl Cpu {
    pub(crate) fn new_with_values(
        name: &str,
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
        frequency: u64,
        vendor_id: String,
        brand: String,
    ) -> Cpu {
        Cpu {
            name: name.to_owned(),
            old_values: CpuValues::new(),
            new_values: CpuValues::new_with_values(
                user, nice, system, idle, iowait, irq, softirq, steal, guest, guest_nice,
            ),
            cpu_usage: 0f32,
            total_time: 0,
            old_total_time: 0,
            frequency,
            vendor_id,
            brand,
        }
    }

    pub(crate) fn set(
        &mut self,
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
    ) {
        macro_rules! min {
            ($a:expr, $b:expr) => {
                if $a > $b {
                    ($a - $b) as f32
                } else {
                    1.
                }
            };
        }
        self.old_values = self.new_values;
        self.new_values.set(
            user, nice, system, idle, iowait, irq, softirq, steal, guest, guest_nice,
        );
        self.total_time = self.new_values.total_time();
        self.old_total_time = self.old_values.total_time();
        self.cpu_usage = min!(self.new_values.work_time(), self.old_values.work_time())
            / min!(self.total_time, self.old_total_time)
            * 100.;
        if self.cpu_usage > 100. {
            self.cpu_usage = 100.; // to prevent the percentage to go above 100%
        }
    }
}

impl CpuExt for Cpu {
    fn cpu_usage(&self) -> f32 {
        self.cpu_usage
    }

    fn name(&self) -> &str {
        &self.name
    }

    /// Returns the CPU frequency in MHz.
    fn frequency(&self) -> u64 {
        self.frequency
    }

    fn vendor_id(&self) -> &str {
        &self.vendor_id
    }

    fn brand(&self) -> &str {
        &self.brand
    }
}

pub(crate) fn get_raw_times(p: &Cpu) -> (u64, u64) {
    (p.total_time, p.old_total_time)
}

pub(crate) fn get_cpu_frequency(cpu_core_index: usize) -> u64 {
    let mut s = String::new();
    if File::open(format!(
        "/sys/devices/system/cpu/cpu{}/cpufreq/scaling_cur_freq",
        cpu_core_index
    ))
    .and_then(|mut f| f.read_to_string(&mut s))
    .is_ok()
    {
        let freq_option = s.trim().split('\n').next();
        if let Some(freq_string) = freq_option {
            if let Ok(freq) = freq_string.parse::<u64>() {
                return freq / 1000;
            }
        }
    }
    s.clear();
    if File::open("/proc/cpuinfo")
        .and_then(|mut f| f.read_to_string(&mut s))
        .is_err()
    {
        return 0;
    }
    let find_cpu_mhz = s.split('\n').find(|line| {
        line.starts_with("cpu MHz\t")
            || line.starts_with("BogoMIPS")
            || line.starts_with("clock\t")
            || line.starts_with("bogomips per cpu")
    });
    find_cpu_mhz
        .and_then(|line| line.split(':').last())
        .and_then(|val| val.replace("MHz", "").trim().parse::<f64>().ok())
        .map(|speed| speed as u64)
        .unwrap_or_default()
}

#[allow(unused_assignments)]
pub(crate) fn get_physical_core_count() -> Option<usize> {
    let mut s = String::new();
    if let Err(_e) = File::open("/proc/cpuinfo").and_then(|mut f| f.read_to_string(&mut s)) {
        sysinfo_debug!("Cannot read `/proc/cpuinfo` file: {:?}", _e);
        return None;
    }

    macro_rules! add_core {
        ($core_ids_and_physical_ids:ident, $core_id:ident, $physical_id:ident, $cpu:ident) => {{
            if !$core_id.is_empty() && !$physical_id.is_empty() {
                $core_ids_and_physical_ids.insert(format!("{} {}", $core_id, $physical_id));
            } else if !$cpu.is_empty() {
                // On systems with only physical cores like raspberry, there is no "core id" or
                // "physical id" fields. So if one of them is missing, we simply use the "CPU"
                // info and count it as a physical core.
                $core_ids_and_physical_ids.insert($cpu.to_owned());
            }
            $core_id = "";
            $physical_id = "";
            $cpu = "";
        }};
    }

    let mut core_ids_and_physical_ids: HashSet<String> = HashSet::new();
    let mut core_id = "";
    let mut physical_id = "";
    let mut cpu = "";

    for line in s.lines() {
        if line.is_empty() {
            add_core!(core_ids_and_physical_ids, core_id, physical_id, cpu);
        } else if line.starts_with("processor") {
            cpu = line
                .splitn(2, ':')
                .last()
                .map(|x| x.trim())
                .unwrap_or_default();
        } else if line.starts_with("core id") {
            core_id = line
                .splitn(2, ':')
                .last()
                .map(|x| x.trim())
                .unwrap_or_default();
        } else if line.starts_with("physical id") {
            physical_id = line
                .splitn(2, ':')
                .last()
                .map(|x| x.trim())
                .unwrap_or_default();
        }
    }
    add_core!(core_ids_and_physical_ids, core_id, physical_id, cpu);

    Some(core_ids_and_physical_ids.len())
}

/// Returns the brand/vendor string for the first CPU (which should be the same for all CPUs).
pub(crate) fn get_vendor_id_and_brand() -> (String, String) {
    let mut s = String::new();
    if File::open("/proc/cpuinfo")
        .and_then(|mut f| f.read_to_string(&mut s))
        .is_err()
    {
        return (String::new(), String::new());
    }

    fn get_value(s: &str) -> String {
        s.split(':')
            .last()
            .map(|x| x.trim().to_owned())
            .unwrap_or_default()
    }

    let mut vendor_id = None;
    let mut brand = None;

    for it in s.split('\n') {
        if it.starts_with("vendor_id\t") {
            vendor_id = Some(get_value(it));
        } else if it.starts_with("model name\t") {
            brand = Some(get_value(it));
        } else {
            continue;
        }
        if brand.is_some() && vendor_id.is_some() {
            break;
        }
    }
    (vendor_id.unwrap_or_default(), brand.unwrap_or_default())
}
