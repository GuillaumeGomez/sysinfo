// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::utils::{get_sys_value_array, get_sys_value_by_name, get_sys_value_str};
use crate::{Cpu, CpuRefreshKind};

use super::ffi;

use libc::{c_int, c_long, c_ulong};

pub(crate) struct CpusWrapper {
    pub(crate) global_cpu_usage: f32,
    pub(crate) cpus: Vec<Cpu>,
    got_cpu_frequency: bool,
    // For the global CPU usage.
    global_cp_times: VecSwitcher<u64>,
    // For each CPU usage.
    cp_times: Vec<VecSwitcher<u64>>,
    nb_cpus: usize,
}

impl CpusWrapper {
    pub(crate) fn new() -> Self {
        unsafe {
            let nb_cpus = super::utils::get_nb_cpus();
            Self {
                global_cpu_usage: 0.,
                cpus: Vec::with_capacity(nb_cpus),
                got_cpu_frequency: false,
                global_cp_times: VecSwitcher::new(vec![0; ffi::CPUSTATES]),
                cp_times: std::iter::repeat_with(|| VecSwitcher::new(vec![0; ffi::CPUSTATES]))
                    .take(nb_cpus)
                    .collect(),
                nb_cpus,
            }
        }
    }

    pub(crate) fn refresh(&mut self, refresh_kind: CpuRefreshKind) {
        if self.cpus.is_empty() {
            let mut frequency = 0;

            // We get the CPU vendor ID in here.
            let vendor_id = unsafe {
                get_sys_value_str(&[libc::CTL_HW, ffi::HW_MODEL])
                    .unwrap_or_else(|| "<unknown>".to_owned())
            };

            for pos in 0..self.nb_cpus {
                self.cpus.push(Cpu {
                    inner: CpuInner::new(format!("cpu {pos}"), vendor_id.clone(), frequency),
                });
            }
            if refresh_kind.frequency() {
                unsafe {
                    get_cpu_frequencies(&mut self.cpus);
                }
                self.got_cpu_frequency = true;
            }
        } else if refresh_kind.frequency() && !self.got_cpu_frequency {
            unsafe {
                get_cpu_frequencies(&mut self.cpus);
            }
            self.got_cpu_frequency = true;
        }
        if refresh_kind.cpu_usage() {
            self.get_cpu_usage();
        }
    }

    fn get_cpu_usage(&mut self) {
        fn compute_cpu_usage(new_cp_times: &[c_ulong], old_cp_times: &[c_ulong]) -> f32 {
            let mut total_time = 0;
            for i in 0..ffi::CPUSTATES {
                total_time += new_cp_times[i];
            }
            if total_time < 1 {
                total_time = 1;
            }
            let total = total_time as f32 * 100.;

            let nice = new_cp_times[ffi::CP_NICE].saturating_sub(old_cp_times[ffi::CP_NICE]) as f32
                / total;
            let user = new_cp_times[ffi::CP_USER].saturating_sub(old_cp_times[ffi::CP_USER]) as f32
                / total;
            let kernel =
                new_cp_times[ffi::CP_SYS].saturating_sub(old_cp_times[ffi::CP_SYS]) as f32 / total;
            nice + user + kernel
        }

        let global_cp_times = self.global_cp_times.get_mut();
        for (pos, (cpu, cp_times)) in self
            .cpus
            .iter_mut()
            .zip(self.cp_times.iter_mut())
            .enumerate()
        {
            unsafe {
                if !get_sys_value_array(
                    &[ffi::CTL_KERN, libc::KERN_CP_TIME, pos as _],
                    cp_times.get_mut(),
                ) {
                    sysinfo_debug!(
                        "wrong CPU time computation, skipping it. Likely invalid CPU usage result"
                    );
                    continue;
                }
            }

            let new_cp_times = cp_times.get_new();
            cpu.inner.cpu_usage = compute_cpu_usage(new_cp_times, cp_times.get_old());

            global_cp_times[ffi::CP_USER] += new_cp_times[ffi::CP_USER];
            global_cp_times[ffi::CP_NICE] += new_cp_times[ffi::CP_NICE];
            global_cp_times[ffi::CP_SYS] += new_cp_times[ffi::CP_SYS];
            global_cp_times[ffi::CP_INTR] += new_cp_times[ffi::CP_INTR];
            global_cp_times[ffi::CP_IDLE] += new_cp_times[ffi::CP_IDLE];
        }
        for global_cp_time in global_cp_times {
            *global_cp_time /= self.cpus.len() as u64;
        }
        self.global_cpu_usage = compute_cpu_usage(
            self.global_cp_times.get_new(),
            self.global_cp_times.get_old(),
        );
    }
}

pub(crate) struct CpuInner {
    pub(crate) cpu_usage: f32,
    name: String,
    pub(crate) vendor_id: String,
    pub(crate) frequency: u64,
}

impl CpuInner {
    pub(crate) fn new(name: String, vendor_id: String, frequency: u64) -> Self {
        Self {
            cpu_usage: 0.,
            name,
            vendor_id,
            frequency,
        }
    }

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
        ""
    }
}

pub(crate) fn physical_core_count() -> Option<usize> {
    let mut physical_core_count: u32 = 0;

    unsafe {
        if get_sys_value_by_name(b"hw.ncpu\0", &mut physical_core_count) {
            Some(physical_core_count as _)
        } else {
            None
        }
    }
}

// "newer" way to get CPU frequency.
unsafe fn get_cpu_frequencies_newer(cpus: &mut [Cpu]) -> bool {
    let mut freq: c_long = 0;

    for (cpu_nb, cpu) in cpus.iter_mut().enumerate() {
        unsafe {
            if !get_sys_value_by_name(
                format!("machdep.cpufreq.cpu{cpu_nb}.current\0").as_bytes(),
                &mut freq,
            ) {
                cpu.inner.frequency = 0;
                return false;
            }
            cpu.inner.frequency = freq as _;
        }
    }
    return true;
}

unsafe fn get_cpu_frequencies(cpus: &mut [Cpu]) {
    unsafe {
        if get_cpu_frequencies_newer(cpus) {
            // We got our frequencies, nothing more to be done.
            return;
        }
    }
    // Now we try our luck through legacy `sysctl` calls.
    const FREQ_SYSCTLS: &[(&[u8], c_long)] = &[
        (b"machdep.est.frequency.current", 1),
        (b"machdep.powernow.frequency.current", 1),
        (b"machdep.intrepid.frequency.current", 1),
        (b"machdep.loongson.frequency.current", 1),
        (b"machdep.cpu.frequency.current", 1),
        (b"machdep.frequency.current", 1),
        (b"machdep.tsc_freq", 1000000),
    ];

    let mut freq: c_long = 0;
    for (freq_sysctl, scale) in FREQ_SYSCTLS {
        unsafe {
            if get_sys_value_by_name(freq_sysctl, &mut freq) {
                for cpu in cpus {
                    cpu.inner.frequency = freq as _;
                }
                return;
            }
        }
    }
    sysinfo_debug!("failed to retrieve CPUs frequency");
}

/// This struct is used to switch between the "old" and "new" every time you use "get_mut".
#[derive(Debug)]
pub(crate) struct VecSwitcher<T> {
    v1: Vec<T>,
    v2: Vec<T>,
    first: bool,
}

impl<T: Clone> VecSwitcher<T> {
    pub fn new(v1: Vec<T>) -> Self {
        let v2 = v1.clone();

        Self {
            v1,
            v2,
            first: true,
        }
    }

    pub fn get_mut(&mut self) -> &mut [T] {
        self.first = !self.first;
        if self.first {
            // It means that `v2` will be the "new".
            &mut self.v2
        } else {
            // It means that `v1` will be the "new".
            &mut self.v1
        }
    }

    pub fn get_old(&self) -> &[T] {
        if self.first { &self.v1 } else { &self.v2 }
    }

    pub fn get_new(&self) -> &[T] {
        if self.first { &self.v2 } else { &self.v1 }
    }
}
