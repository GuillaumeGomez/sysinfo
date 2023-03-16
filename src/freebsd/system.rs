// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    sys::{component::Component, Cpu, Disk, Networks, Process},
    CpuRefreshKind, LoadAvg, Pid, ProcessRefreshKind, RefreshKind, SystemExt, User,
};

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::time::Duration;

use super::utils::{
    self, boot_time, c_buf_to_string, from_cstr_array, get_frequency_for_cpu, get_sys_value,
    get_sys_value_array, get_sys_value_by_name, get_sys_value_str_by_name, get_system_info,
    init_mib,
};

use libc::c_int;

declare_signals! {
    c_int,
    Signal::Hangup => libc::SIGHUP,
    Signal::Interrupt => libc::SIGINT,
    Signal::Quit => libc::SIGQUIT,
    Signal::Illegal => libc::SIGILL,
    Signal::Trap => libc::SIGTRAP,
    Signal::Abort => libc::SIGABRT,
    Signal::IOT => libc::SIGIOT,
    Signal::Bus => libc::SIGBUS,
    Signal::FloatingPointException => libc::SIGFPE,
    Signal::Kill => libc::SIGKILL,
    Signal::User1 => libc::SIGUSR1,
    Signal::Segv => libc::SIGSEGV,
    Signal::User2 => libc::SIGUSR2,
    Signal::Pipe => libc::SIGPIPE,
    Signal::Alarm => libc::SIGALRM,
    Signal::Term => libc::SIGTERM,
    Signal::Child => libc::SIGCHLD,
    Signal::Continue => libc::SIGCONT,
    Signal::Stop => libc::SIGSTOP,
    Signal::TSTP => libc::SIGTSTP,
    Signal::TTIN => libc::SIGTTIN,
    Signal::TTOU => libc::SIGTTOU,
    Signal::Urgent => libc::SIGURG,
    Signal::XCPU => libc::SIGXCPU,
    Signal::XFSZ => libc::SIGXFSZ,
    Signal::VirtualAlarm => libc::SIGVTALRM,
    Signal::Profiling => libc::SIGPROF,
    Signal::Winch => libc::SIGWINCH,
    Signal::IO => libc::SIGIO,
    Signal::Sys => libc::SIGSYS,
    _ => None,
}

#[doc = include_str!("../../md_doc/system.md")]
pub struct System {
    process_list: HashMap<Pid, Process>,
    mem_total: u64,
    mem_free: u64,
    mem_used: u64,
    swap_total: u64,
    swap_used: u64,
    global_cpu: Cpu,
    cpus: Vec<Cpu>,
    components: Vec<Component>,
    disks: Vec<Disk>,
    networks: Networks,
    users: Vec<User>,
    boot_time: u64,
    system_info: SystemInfo,
    got_cpu_frequency: bool,
}

impl SystemExt for System {
    const IS_SUPPORTED: bool = true;
    const SUPPORTED_SIGNALS: &'static [Signal] = supported_signals();
    const MINIMUM_CPU_UPDATE_INTERVAL: Duration = Duration::from_millis(100);

    fn new_with_specifics(refreshes: RefreshKind) -> System {
        let system_info = SystemInfo::new();

        let mut s = System {
            process_list: HashMap::with_capacity(200),
            mem_total: 0,
            mem_free: 0,
            mem_used: 0,
            swap_total: 0,
            swap_used: 0,
            global_cpu: Cpu::new(String::new(), String::new(), 0),
            cpus: Vec::with_capacity(system_info.nb_cpus as _),
            components: Vec::with_capacity(2),
            disks: Vec::with_capacity(1),
            networks: Networks::new(),
            users: Vec::new(),
            boot_time: boot_time(),
            system_info,
            got_cpu_frequency: false,
        };
        s.refresh_specifics(refreshes);
        s
    }

    fn refresh_memory(&mut self) {
        if self.mem_total == 0 {
            self.mem_total = self.system_info.get_total_memory();
        }
        self.mem_used = self.system_info.get_used_memory();
        self.mem_free = self.system_info.get_free_memory();
        let (swap_used, swap_total) = self.system_info.get_swap_info();
        self.swap_total = swap_total;
        self.swap_used = swap_used;
    }

    fn refresh_cpu_specifics(&mut self, refresh_kind: CpuRefreshKind) {
        if self.cpus.is_empty() {
            let mut frequency = 0;

            // We get the CPU vendor ID in here.
            let vendor_id =
                get_sys_value_str_by_name(b"hw.model\0").unwrap_or_else(|| "<unknown>".to_owned());
            for pos in 0..self.system_info.nb_cpus {
                if refresh_kind.frequency() {
                    unsafe {
                        frequency = get_frequency_for_cpu(pos);
                    }
                }
                self.cpus
                    .push(Cpu::new(format!("cpu {pos}"), vendor_id.clone(), frequency));
            }
            self.global_cpu.vendor_id = vendor_id;
            self.got_cpu_frequency = refresh_kind.frequency();
        } else if refresh_kind.frequency() && !self.got_cpu_frequency {
            for (pos, proc_) in self.cpus.iter_mut().enumerate() {
                unsafe {
                    proc_.frequency = get_frequency_for_cpu(pos as _);
                }
            }
            self.got_cpu_frequency = true;
        }
        if refresh_kind.cpu_usage() {
            self.system_info
                .get_cpu_usage(&mut self.global_cpu, &mut self.cpus);
        }
        if refresh_kind.frequency() {
            self.global_cpu.frequency = self.cpus.get(0).map(|cpu| cpu.frequency).unwrap_or(0);
        }
    }

    fn refresh_components_list(&mut self) {
        if self.cpus.is_empty() {
            self.refresh_cpu();
        }
        self.components = unsafe { super::component::get_components(self.cpus.len()) };
    }

    fn refresh_processes_specifics(&mut self, refresh_kind: ProcessRefreshKind) {
        unsafe { self.refresh_procs(refresh_kind) }
    }

    fn refresh_process_specifics(&mut self, pid: Pid, refresh_kind: ProcessRefreshKind) -> bool {
        unsafe {
            let kd = self.system_info.kd.as_ptr();
            let mut count = 0;
            let procs = libc::kvm_getprocs(kd, libc::KERN_PROC_PROC, 0, &mut count);
            if count < 1 {
                sysinfo_debug!("kvm_getprocs returned nothing...");
                return false;
            }
            let now = super::utils::get_now();

            let fscale = self.system_info.fscale;
            let page_size = self.system_info.page_size as isize;
            let proc_list = utils::WrapMap(UnsafeCell::new(&mut self.process_list));
            let procs: &mut [utils::KInfoProc] =
                std::slice::from_raw_parts_mut(procs as _, count as _);

            #[cfg(feature = "multithread")]
            use rayon::iter::ParallelIterator;

            macro_rules! multi_iter {
                ($name:ident, $($iter:tt)+) => {
                    $name = crate::utils::into_iter(procs).$($iter)+;
                }
            }

            let ret;
            #[cfg(not(feature = "multithread"))]
            multi_iter!(ret, find(|kproc| kproc.ki_pid == pid.0));
            #[cfg(feature = "multithread")]
            multi_iter!(ret, find_any(|kproc| kproc.ki_pid == pid.0));

            let kproc = if let Some(kproc) = ret {
                kproc
            } else {
                return false;
            };
            match super::process::get_process_data(
                kproc,
                &proc_list,
                page_size,
                fscale,
                now,
                refresh_kind,
            ) {
                Ok(Some(proc_)) => {
                    self.add_missing_proc_info(self.system_info.kd.as_ptr(), kproc, proc_);
                    true
                }
                Ok(None) => true,
                Err(_) => false,
            }
        }
    }

    fn refresh_disks_list(&mut self) {
        self.disks = unsafe { super::disk::get_all_disks() };
    }

    fn refresh_users_list(&mut self) {
        self.users = crate::users::get_users_list();
    }

    // COMMON PART
    //
    // Need to be moved into a "common" file to avoid duplication.

    fn processes(&self) -> &HashMap<Pid, Process> {
        &self.process_list
    }

    fn process(&self, pid: Pid) -> Option<&Process> {
        self.process_list.get(&pid)
    }

    fn networks(&self) -> &Networks {
        &self.networks
    }

    fn networks_mut(&mut self) -> &mut Networks {
        &mut self.networks
    }

    fn global_cpu_info(&self) -> &Cpu {
        &self.global_cpu
    }

    fn cpus(&self) -> &[Cpu] {
        &self.cpus
    }

    fn physical_core_count(&self) -> Option<usize> {
        let mut physical_core_count: u32 = 0;

        unsafe {
            if get_sys_value_by_name(b"hw.ncpu\0", &mut physical_core_count) {
                Some(physical_core_count as _)
            } else {
                None
            }
        }
    }

    fn total_memory(&self) -> u64 {
        self.mem_total
    }

    fn free_memory(&self) -> u64 {
        self.mem_free
    }

    fn available_memory(&self) -> u64 {
        self.mem_free
    }

    fn used_memory(&self) -> u64 {
        self.mem_used
    }

    fn total_swap(&self) -> u64 {
        self.swap_total
    }

    fn free_swap(&self) -> u64 {
        self.swap_total - self.swap_used
    }

    // TODO: need to be checked
    fn used_swap(&self) -> u64 {
        self.swap_used
    }

    fn components(&self) -> &[Component] {
        &self.components
    }

    fn components_mut(&mut self) -> &mut [Component] {
        &mut self.components
    }

    fn disks(&self) -> &[Disk] {
        &self.disks
    }

    fn disks_mut(&mut self) -> &mut [Disk] {
        &mut self.disks
    }

    fn sort_disks_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Disk, &Disk) -> std::cmp::Ordering,
    {
        self.disks.sort_unstable_by(compare);
    }

    fn uptime(&self) -> u64 {
        unsafe {
            let csec = libc::time(::std::ptr::null_mut());

            libc::difftime(csec, self.boot_time as _) as u64
        }
    }

    fn boot_time(&self) -> u64 {
        self.boot_time
    }

    fn load_average(&self) -> LoadAvg {
        let mut loads = vec![0f64; 3];
        unsafe {
            libc::getloadavg(loads.as_mut_ptr(), 3);
            LoadAvg {
                one: loads[0],
                five: loads[1],
                fifteen: loads[2],
            }
        }
    }

    fn users(&self) -> &[User] {
        &self.users
    }

    fn name(&self) -> Option<String> {
        self.system_info.get_os_name()
    }

    fn long_os_version(&self) -> Option<String> {
        self.system_info.get_os_release_long()
    }

    fn host_name(&self) -> Option<String> {
        self.system_info.get_hostname()
    }

    fn kernel_version(&self) -> Option<String> {
        self.system_info.get_kernel_version()
    }

    fn os_version(&self) -> Option<String> {
        self.system_info.get_os_release()
    }

    fn distribution_id(&self) -> String {
        std::env::consts::OS.to_owned()
    }
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

impl System {
    unsafe fn refresh_procs(&mut self, refresh_kind: ProcessRefreshKind) {
        let kd = self.system_info.kd.as_ptr();
        let procs = {
            let mut count = 0;
            let procs = libc::kvm_getprocs(kd, libc::KERN_PROC_PROC, 0, &mut count);
            if count < 1 {
                sysinfo_debug!("kvm_getprocs returned nothing...");
                return;
            }
            #[cfg(feature = "multithread")]
            use rayon::iter::{ParallelIterator, ParallelIterator as IterTrait};
            #[cfg(not(feature = "multithread"))]
            use std::iter::Iterator as IterTrait;

            let fscale = self.system_info.fscale;
            let page_size = self.system_info.page_size as isize;
            let now = super::utils::get_now();
            let proc_list = utils::WrapMap(UnsafeCell::new(&mut self.process_list));
            let procs: &mut [utils::KInfoProc] =
                std::slice::from_raw_parts_mut(procs as _, count as _);

            IterTrait::filter_map(crate::utils::into_iter(procs), |kproc| {
                super::process::get_process_data(
                    kproc,
                    &proc_list,
                    page_size,
                    fscale,
                    now,
                    refresh_kind,
                )
                .ok()
                .and_then(|p| p.map(|p| (kproc, p)))
            })
            .collect::<Vec<_>>()
        };

        // We remove all processes that don't exist anymore.
        self.process_list
            .retain(|_, v| std::mem::replace(&mut v.updated, false));

        for (kproc, proc_) in procs {
            self.add_missing_proc_info(kd, kproc, proc_);
        }
    }

    unsafe fn add_missing_proc_info(
        &mut self,
        kd: *mut libc::kvm_t,
        kproc: &libc::kinfo_proc,
        mut proc_: Process,
    ) {
        proc_.cmd = from_cstr_array(libc::kvm_getargv(kd, kproc, 0) as _);
        self.system_info.get_proc_missing_info(kproc, &mut proc_);
        if !proc_.cmd.is_empty() {
            // First, we try to retrieve the name from the command line.
            let p = Path::new(&proc_.cmd[0]);
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                proc_.name = name.to_owned();
            }
            if proc_.root.as_os_str().is_empty() {
                if let Some(parent) = p.parent() {
                    proc_.root = parent.to_path_buf();
                }
            }
        }
        if proc_.name.is_empty() {
            // The name can be cut short because the `ki_comm` field size is limited,
            // which is why we prefer to get the name from the command line as much as
            // possible.
            proc_.name = c_buf_to_string(&kproc.ki_comm).unwrap_or_default();
        }
        proc_.environ = from_cstr_array(libc::kvm_getenvv(kd, kproc, 0) as _);
        self.process_list.insert(proc_.pid, proc_);
    }
}

#[derive(Debug)]
struct Zfs {
    enabled: bool,
    mib_arcstats_size: [c_int; 5],
}

impl Zfs {
    fn new() -> Self {
        let mut zfs = Self {
            enabled: false,
            mib_arcstats_size: Default::default(),
        };
        unsafe {
            init_mib(
                b"kstat.zfs.misc.arcstats.size\0",
                &mut zfs.mib_arcstats_size,
            );
            let mut arc_size: u64 = 0;
            if get_sys_value(&zfs.mib_arcstats_size, &mut arc_size) {
                zfs.enabled = arc_size != 0;
            }
        }
        zfs
    }

    fn arc_size(&self) -> Option<u64> {
        if self.enabled {
            let mut arc_size: u64 = 0;
            unsafe {
                get_sys_value(&self.mib_arcstats_size, &mut arc_size);
                Some(arc_size)
            }
        } else {
            None
        }
    }
}

/// This struct is used to get system information more easily.
#[derive(Debug)]
struct SystemInfo {
    hw_physical_memory: [c_int; 2],
    page_size: c_int,
    virtual_page_count: [c_int; 4],
    virtual_wire_count: [c_int; 4],
    virtual_active_count: [c_int; 4],
    virtual_cache_count: [c_int; 4],
    virtual_inactive_count: [c_int; 4],
    virtual_free_count: [c_int; 4],
    os_type: [c_int; 2],
    os_release: [c_int; 2],
    kern_version: [c_int; 2],
    hostname: [c_int; 2],
    buf_space: [c_int; 2],
    nb_cpus: c_int,
    kd: NonNull<libc::kvm_t>,
    // For these two fields, we could use `kvm_getcptime` but the function isn't very efficient...
    mib_cp_time: [c_int; 2],
    mib_cp_times: [c_int; 2],
    // For the global CPU usage.
    cp_time: utils::VecSwitcher<libc::c_ulong>,
    // For each CPU usage.
    cp_times: utils::VecSwitcher<libc::c_ulong>,
    /// From FreeBSD manual: "The kernel fixed-point scale factor". It's used when computing
    /// processes' CPU usage.
    fscale: f32,
    procstat: *mut libc::procstat,
    zfs: Zfs,
}

// This is needed because `kd: *mut libc::kvm_t` isn't thread-safe.
unsafe impl Send for SystemInfo {}
unsafe impl Sync for SystemInfo {}

impl SystemInfo {
    fn new() -> Self {
        unsafe {
            let mut errbuf =
                MaybeUninit::<[libc::c_char; libc::_POSIX2_LINE_MAX as usize]>::uninit();
            let kd = NonNull::new(libc::kvm_openfiles(
                std::ptr::null(),
                b"/dev/null\0".as_ptr() as *const _,
                std::ptr::null(),
                0,
                errbuf.as_mut_ptr() as *mut _,
            ))
            .expect("kvm_openfiles failed");

            let mut smp: c_int = 0;
            let mut nb_cpus: c_int = 1;
            if !get_sys_value_by_name(b"kern.smp.active\0", &mut smp) {
                smp = 0;
            }
            #[allow(clippy::collapsible_if)] // I keep as is for readability reasons.
            if smp != 0 {
                if !get_sys_value_by_name(b"kern.smp.cpus\0", &mut nb_cpus) || nb_cpus < 1 {
                    nb_cpus = 1;
                }
            }

            let mut si = SystemInfo {
                hw_physical_memory: Default::default(),
                page_size: 0,
                virtual_page_count: Default::default(),
                virtual_wire_count: Default::default(),
                virtual_active_count: Default::default(),
                virtual_cache_count: Default::default(),
                virtual_inactive_count: Default::default(),
                virtual_free_count: Default::default(),
                buf_space: Default::default(),
                os_type: Default::default(),
                os_release: Default::default(),
                kern_version: Default::default(),
                hostname: Default::default(),
                nb_cpus,
                kd,
                mib_cp_time: Default::default(),
                mib_cp_times: Default::default(),
                cp_time: utils::VecSwitcher::new(vec![0; libc::CPUSTATES as usize]),
                cp_times: utils::VecSwitcher::new(vec![
                    0;
                    nb_cpus as usize * libc::CPUSTATES as usize
                ]),
                fscale: 0.,
                procstat: std::ptr::null_mut(),
                zfs: Zfs::new(),
            };
            let mut fscale: c_int = 0;
            if !get_sys_value_by_name(b"kern.fscale\0", &mut fscale) {
                // Default value used in htop.
                fscale = 2048;
            }
            si.fscale = fscale as f32;

            if !get_sys_value_by_name(b"vm.stats.vm.v_page_size\0", &mut si.page_size) {
                panic!("cannot get page size...");
            }

            init_mib(b"hw.physmem\0", &mut si.hw_physical_memory);
            init_mib(b"vm.stats.vm.v_page_count\0", &mut si.virtual_page_count);
            init_mib(b"vm.stats.vm.v_wire_count\0", &mut si.virtual_wire_count);
            init_mib(
                b"vm.stats.vm.v_active_count\0",
                &mut si.virtual_active_count,
            );
            init_mib(b"vm.stats.vm.v_cache_count\0", &mut si.virtual_cache_count);
            init_mib(
                b"vm.stats.vm.v_inactive_count\0",
                &mut si.virtual_inactive_count,
            );
            init_mib(b"vm.stats.vm.v_free_count\0", &mut si.virtual_free_count);
            init_mib(b"vfs.bufspace\0", &mut si.buf_space);

            init_mib(b"kern.ostype\0", &mut si.os_type);
            init_mib(b"kern.osrelease\0", &mut si.os_release);
            init_mib(b"kern.version\0", &mut si.kern_version);
            init_mib(b"kern.hostname\0", &mut si.hostname);

            init_mib(b"kern.cp_time\0", &mut si.mib_cp_time);
            init_mib(b"kern.cp_times\0", &mut si.mib_cp_times);

            si
        }
    }

    fn get_os_name(&self) -> Option<String> {
        get_system_info(&[self.os_type[0], self.os_type[1]], Some("FreeBSD"))
    }

    fn get_kernel_version(&self) -> Option<String> {
        get_system_info(&[self.kern_version[0], self.kern_version[1]], None)
    }

    fn get_os_release_long(&self) -> Option<String> {
        get_system_info(&[self.os_release[0], self.os_release[1]], None)
    }

    fn get_os_release(&self) -> Option<String> {
        // It returns something like "13.0-RELEASE". We want to keep everything until the "-".
        get_system_info(&[self.os_release[0], self.os_release[1]], None)
            .and_then(|s| s.split('-').next().map(|s| s.to_owned()))
    }

    fn get_hostname(&self) -> Option<String> {
        get_system_info(&[self.hostname[0], self.hostname[1]], Some(""))
    }

    /// Returns (used, total).
    fn get_swap_info(&self) -> (u64, u64) {
        // Magic number used in htop. Cannot find how they got it when reading `kvm_getswapinfo`
        // source code so here we go...
        const LEN: usize = 16;
        let mut swap = MaybeUninit::<[libc::kvm_swap; LEN]>::uninit();
        unsafe {
            let nswap =
                libc::kvm_getswapinfo(self.kd.as_ptr(), swap.as_mut_ptr() as *mut _, LEN as _, 0)
                    as usize;
            if nswap < 1 {
                return (0, 0);
            }
            let swap =
                std::slice::from_raw_parts(swap.as_ptr() as *mut libc::kvm_swap, nswap.min(LEN));
            let (used, total) = swap.iter().fold((0, 0), |(used, total): (u64, u64), swap| {
                (
                    used.saturating_add(swap.ksw_used as _),
                    total.saturating_add(swap.ksw_total as _),
                )
            });
            (
                used.saturating_mul(self.page_size as _),
                total.saturating_mul(self.page_size as _),
            )
        }
    }

    fn get_total_memory(&self) -> u64 {
        let mut nb_pages: u64 = 0;
        unsafe {
            if get_sys_value(&self.virtual_page_count, &mut nb_pages) {
                return nb_pages.saturating_mul(self.page_size as _);
            }

            // This is a fallback. It includes all the available memory, not just the one available for
            // the users.
            let mut total_memory: u64 = 0;
            get_sys_value(&self.hw_physical_memory, &mut total_memory);
            total_memory
        }
    }

    fn get_used_memory(&self) -> u64 {
        let mut mem_active: u64 = 0;
        let mut mem_wire: u64 = 0;

        unsafe {
            get_sys_value(&self.virtual_active_count, &mut mem_active);
            get_sys_value(&self.virtual_wire_count, &mut mem_wire);

            let mut mem_wire = mem_wire.saturating_mul(self.page_size as _);
            // We need to subtract "ZFS ARC" from the "wired memory" because it should belongs to cache
            // but the kernel reports it as "wired memory" instead...
            if let Some(arc_size) = self.zfs.arc_size() {
                mem_wire -= arc_size;
            }
            mem_active
                .saturating_mul(self.page_size as _)
                .saturating_add(mem_wire)
        }
    }

    fn get_free_memory(&self) -> u64 {
        let mut buffers_mem: u64 = 0;
        let mut inactive_mem: u64 = 0;
        let mut cached_mem: u64 = 0;
        let mut free_mem: u64 = 0;

        unsafe {
            get_sys_value(&self.buf_space, &mut buffers_mem);
            get_sys_value(&self.virtual_inactive_count, &mut inactive_mem);
            get_sys_value(&self.virtual_cache_count, &mut cached_mem);
            get_sys_value(&self.virtual_free_count, &mut free_mem);
            // For whatever reason, buffers_mem is already the right value...
            buffers_mem
                .saturating_add(inactive_mem.saturating_mul(self.page_size as _))
                .saturating_add(cached_mem.saturating_mul(self.page_size as _))
                .saturating_add(free_mem.saturating_mul(self.page_size as _))
        }
    }

    fn get_cpu_usage(&mut self, global: &mut Cpu, cpus: &mut [Cpu]) {
        unsafe {
            get_sys_value_array(&self.mib_cp_time, self.cp_time.get_mut());
            get_sys_value_array(&self.mib_cp_times, self.cp_times.get_mut());
        }

        fn fill_cpu(proc_: &mut Cpu, new_cp_time: &[libc::c_ulong], old_cp_time: &[libc::c_ulong]) {
            let mut total_new: u64 = 0;
            let mut total_old: u64 = 0;
            let mut cp_diff: libc::c_ulong = 0;

            for i in 0..(libc::CPUSTATES as usize) {
                // We obviously don't want to get the idle part of the CPU usage, otherwise
                // we would always be at 100%...
                if i != libc::CP_IDLE as usize {
                    cp_diff += new_cp_time[i] - old_cp_time[i];
                }
                let mut tmp: u64 = new_cp_time[i] as _;
                total_new += tmp;
                tmp = old_cp_time[i] as _;
                total_old += tmp;
            }

            let total_diff = total_new - total_old;
            if total_diff < 1 {
                proc_.cpu_usage = 0.;
            } else {
                proc_.cpu_usage = cp_diff as f32 / total_diff as f32 * 100.;
            }
        }

        fill_cpu(global, self.cp_time.get_new(), self.cp_time.get_old());
        let old_cp_times = self.cp_times.get_old();
        let new_cp_times = self.cp_times.get_new();
        for (pos, proc_) in cpus.iter_mut().enumerate() {
            let index = pos * libc::CPUSTATES as usize;

            fill_cpu(proc_, &new_cp_times[index..], &old_cp_times[index..]);
        }
    }

    #[allow(clippy::collapsible_if)] // I keep as is for readability reasons.
    unsafe fn get_proc_missing_info(&mut self, kproc: &libc::kinfo_proc, proc_: &mut Process) {
        if self.procstat.is_null() {
            self.procstat = libc::procstat_open_sysctl();
        }
        if self.procstat.is_null() {
            return;
        }
        let head = libc::procstat_getfiles(self.procstat, kproc as *const _ as usize as *mut _, 0);
        if head.is_null() {
            return;
        }
        let mut entry = (*head).stqh_first;
        let mut done = 0;
        while !entry.is_null() && done < 2 {
            {
                let tmp = &*entry;
                if tmp.fs_uflags & libc::PS_FST_UFLAG_CDIR != 0 {
                    if !tmp.fs_path.is_null() {
                        if let Ok(p) = CStr::from_ptr(tmp.fs_path).to_str() {
                            proc_.cwd = PathBuf::from(p);
                            done += 1;
                        }
                    }
                } else if tmp.fs_uflags & libc::PS_FST_UFLAG_RDIR != 0 {
                    if !tmp.fs_path.is_null() {
                        if let Ok(p) = CStr::from_ptr(tmp.fs_path).to_str() {
                            proc_.root = PathBuf::from(p);
                            done += 1;
                        }
                    }
                }
            }
            entry = (*entry).next.stqe_next;
        }
        libc::procstat_freefiles(self.procstat, head);
    }
}

impl Drop for SystemInfo {
    fn drop(&mut self) {
        unsafe {
            libc::kvm_close(self.kd.as_ptr());
            if !self.procstat.is_null() {
                libc::procstat_close(self.procstat);
            }
        }
    }
}
