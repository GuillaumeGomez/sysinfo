// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::component::Component;
use crate::sys::cpu::*;
use crate::sys::disk::*;
#[cfg(target_os = "macos")]
use crate::sys::ffi;
use crate::sys::network::Networks;
use crate::sys::process::*;
#[cfg(target_os = "macos")]
use core_foundation_sys::base::{kCFAllocatorDefault, CFRelease};

use crate::{
    CpuExt, CpuRefreshKind, LoadAvg, Pid, ProcessRefreshKind, RefreshKind, SystemExt, User,
};

#[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
use crate::ProcessExt;

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::mem;
use std::sync::Arc;
#[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
use std::time::SystemTime;

#[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
use libc::size_t;

use libc::{
    c_char, c_int, c_void, host_statistics64, mach_port_t, sysconf, sysctl, sysctlbyname, timeval,
    vm_statistics64, _SC_PAGESIZE,
};

#[cfg(all(
    target_os = "macos",
    not(feature = "apple-sandbox"),
    target_arch = "aarch64"
))]
use super::inner::component::Components;

#[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
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
    // SIGPOLL doesn't exist on apple targets but since it's an equivalent of SIGIO on unix,
    // we simply use the SIGIO constant.
    Signal::Poll => libc::SIGIO,
    Signal::Sys => libc::SIGSYS,
    _ => None,
}
#[cfg(any(target_os = "ios", feature = "apple-sandbox"))]
declare_signals! {
    c_int,
    _ => None,
}

#[doc = include_str!("../../md_doc/system.md")]
pub struct System {
    process_list: HashMap<Pid, Process>,
    mem_total: u64,
    mem_free: u64,
    mem_available: u64,
    swap_total: u64,
    swap_free: u64,
    global_cpu: Cpu,
    cpus: Vec<Cpu>,
    page_size_kb: u64,
    #[cfg(any(target_os = "ios", target_arch = "x86", target_arch = "x86_64"))]
    components: Vec<Component>,
    #[cfg(all(
        target_os = "macos",
        not(feature = "apple-sandbox"),
        target_arch = "aarch64"
    ))]
    components: Components,
    // Used to get CPU information, not supported on iOS, or inside the default macOS sandbox.
    #[cfg(all(
        target_os = "macos",
        not(feature = "apple-sandbox"),
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    connection: Option<ffi::io_connect_t>,
    disks: Vec<Disk>,
    networks: Networks,
    port: mach_port_t,
    users: Vec<User>,
    boot_time: u64,
    // Used to get disk information, to be more specific, it's needed by the
    // DADiskCreateFromVolumePath function. Not supported on iOS.
    #[cfg(target_os = "macos")]
    session: ffi::SessionWrap,
    #[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
    clock_info: Option<crate::sys::macos::system::SystemTimeInfo>,
    got_cpu_frequency: bool,
}

impl Drop for System {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        unsafe {
            #[cfg(all(
                target_os = "macos",
                not(feature = "apple-sandbox"),
                any(target_arch = "x86", target_arch = "x86_64")
            ))]
            if let Some(conn) = self.connection {
                ffi::IOServiceClose(conn);
            }

            if !self.session.0.is_null() {
                CFRelease(self.session.0 as _);
            }
        }
    }
}

pub(crate) struct Wrap<'a>(pub UnsafeCell<&'a mut HashMap<Pid, Process>>);

unsafe impl<'a> Send for Wrap<'a> {}
unsafe impl<'a> Sync for Wrap<'a> {}

fn boot_time() -> u64 {
    let mut boot_time = timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let mut len = std::mem::size_of::<timeval>();
    let mut mib: [c_int; 2] = [libc::CTL_KERN, libc::KERN_BOOTTIME];

    unsafe {
        if sysctl(
            mib.as_mut_ptr(),
            mib.len() as _,
            &mut boot_time as *mut timeval as *mut _,
            &mut len,
            std::ptr::null_mut(),
            0,
        ) < 0
        {
            0
        } else {
            boot_time.tv_sec as _
        }
    }
}

#[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
fn get_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|n| n.as_secs())
        .unwrap_or(0)
}

impl SystemExt for System {
    const IS_SUPPORTED: bool = true;
    const SUPPORTED_SIGNALS: &'static [Signal] = supported_signals();

    fn new_with_specifics(refreshes: RefreshKind) -> System {
        unsafe {
            let port = libc::mach_host_self();

            let mut s = System {
                process_list: HashMap::with_capacity(200),
                mem_total: 0,
                mem_free: 0,
                mem_available: 0,
                swap_total: 0,
                swap_free: 0,
                global_cpu: Cpu::new(
                    "0".to_owned(),
                    Arc::new(CpuData::new(std::ptr::null_mut(), 0)),
                    0,
                    String::new(),
                    String::new(),
                ),
                cpus: Vec::new(),
                page_size_kb: sysconf(_SC_PAGESIZE) as u64 / 1_000,
                #[cfg(any(target_os = "ios", target_arch = "x86", target_arch = "x86_64"))]
                components: Vec::with_capacity(2),
                #[cfg(all(
                    target_os = "macos",
                    not(feature = "apple-sandbox"),
                    target_arch = "aarch64"
                ))]
                components: Components::new(),
                #[cfg(all(
                    target_os = "macos",
                    not(feature = "apple-sandbox"),
                    any(target_arch = "x86", target_arch = "x86_64")
                ))]
                connection: get_io_service_connection(),
                disks: Vec::with_capacity(1),
                networks: Networks::new(),
                port,
                users: Vec::new(),
                boot_time: boot_time(),
                #[cfg(target_os = "macos")]
                session: ffi::SessionWrap(::std::ptr::null_mut()),
                #[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
                clock_info: crate::sys::macos::system::SystemTimeInfo::new(port),
                got_cpu_frequency: false,
            };
            s.refresh_specifics(refreshes);
            s
        }
    }

    fn refresh_memory(&mut self) {
        let mut mib = [0, 0];

        unsafe {
            // get system values
            // get swap info
            let mut xs: libc::xsw_usage = mem::zeroed::<libc::xsw_usage>();
            if get_sys_value(
                libc::CTL_VM as _,
                libc::VM_SWAPUSAGE as _,
                mem::size_of::<libc::xsw_usage>(),
                &mut xs as *mut _ as *mut c_void,
                &mut mib,
            ) {
                self.swap_total = xs.xsu_total / 1_000;
                self.swap_free = xs.xsu_avail / 1_000;
            }
            // get ram info
            if self.mem_total < 1 {
                get_sys_value(
                    libc::CTL_HW as _,
                    libc::HW_MEMSIZE as _,
                    mem::size_of::<u64>(),
                    &mut self.mem_total as *mut u64 as *mut c_void,
                    &mut mib,
                );
                self.mem_total /= 1_000;
            }
            let mut count: u32 = libc::HOST_VM_INFO64_COUNT as _;
            let mut stat = mem::zeroed::<vm_statistics64>();
            if host_statistics64(
                self.port,
                libc::HOST_VM_INFO64,
                &mut stat as *mut vm_statistics64 as *mut _,
                &mut count,
            ) == libc::KERN_SUCCESS
            {
                // From the apple documentation:
                //
                // /*
                //  * NB: speculative pages are already accounted for in "free_count",
                //  * so "speculative_count" is the number of "free" pages that are
                //  * used to hold data that was read speculatively from disk but
                //  * haven't actually been used by anyone so far.
                //  */
                self.mem_available = self.mem_total.saturating_sub(
                    u64::from(stat.active_count)
                        .saturating_add(u64::from(stat.inactive_count))
                        .saturating_add(u64::from(stat.wire_count))
                        .saturating_add(u64::from(stat.speculative_count))
                        .saturating_sub(u64::from(stat.purgeable_count))
                        .saturating_mul(self.page_size_kb),
                );
                self.mem_free = u64::from(stat.free_count).saturating_mul(self.page_size_kb);
            }
        }
    }

    #[cfg(any(target_os = "ios", feature = "apple-sandbox"))]
    fn refresh_components_list(&mut self) {}

    #[cfg(all(
        target_os = "macos",
        not(feature = "apple-sandbox"),
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    fn refresh_components_list(&mut self) {
        if let Some(con) = self.connection {
            self.components.clear();
            // getting CPU critical temperature
            let critical_temp = crate::apple::component::get_temperature(
                con,
                &['T' as i8, 'C' as i8, '0' as i8, 'D' as i8, 0],
            );

            for (id, v) in crate::apple::component::COMPONENTS_TEMPERATURE_IDS.iter() {
                if let Some(c) = Component::new((*id).to_owned(), None, critical_temp, v, con) {
                    self.components.push(c);
                }
            }
        }
    }

    #[cfg(all(
        target_os = "macos",
        not(feature = "apple-sandbox"),
        target_arch = "aarch64"
    ))]
    fn refresh_components_list(&mut self) {
        self.components.refresh();
    }

    fn refresh_cpu_specifics(&mut self, refresh_kind: CpuRefreshKind) {
        let cpus = &mut self.cpus;
        if cpus.is_empty() {
            init_cpus(self.port, cpus, &mut self.global_cpu, refresh_kind);
            self.got_cpu_frequency = refresh_kind.frequency();
            return;
        }
        if refresh_kind.frequency() && !self.got_cpu_frequency {
            let frequency = get_cpu_frequency();
            for proc_ in cpus.iter_mut() {
                proc_.set_frequency(frequency);
            }
            self.got_cpu_frequency = true;
        }
        if refresh_kind.cpu_usage() {
            update_cpu_usage(self.port, &mut self.global_cpu, |proc_data, cpu_info| {
                let mut percentage = 0f32;
                let mut offset = 0;
                for proc_ in cpus.iter_mut() {
                    let cpu_usage = compute_usage_of_cpu(proc_, cpu_info, offset);
                    proc_.update(cpu_usage, Arc::clone(&proc_data));
                    percentage += proc_.cpu_usage();

                    offset += libc::CPU_STATE_MAX as isize;
                }
                (percentage, cpus.len())
            });
        }
    }

    #[cfg(any(target_os = "ios", feature = "apple-sandbox"))]
    fn refresh_processes_specifics(&mut self, _refresh_kind: ProcessRefreshKind) {}

    #[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
    fn refresh_processes_specifics(&mut self, refresh_kind: ProcessRefreshKind) {
        use crate::utils::into_iter;

        unsafe {
            let count = libc::proc_listallpids(::std::ptr::null_mut(), 0);
            if count < 1 {
                return;
            }
        }
        if let Some(pids) = get_proc_list() {
            let now = get_now();
            let arg_max = get_arg_max();
            let port = self.port;
            let time_interval = self.clock_info.as_mut().map(|c| c.get_time_interval(port));
            let entries: Vec<Process> = {
                let wrap = &Wrap(UnsafeCell::new(&mut self.process_list));

                #[cfg(feature = "multithread")]
                use rayon::iter::ParallelIterator;

                into_iter(pids)
                    .flat_map(|pid| {
                        match update_process(
                            wrap,
                            pid,
                            arg_max as size_t,
                            time_interval,
                            now,
                            refresh_kind,
                            false,
                        ) {
                            Ok(x) => x,
                            _ => None,
                        }
                    })
                    .collect()
            };
            entries.into_iter().for_each(|entry| {
                self.process_list.insert(entry.pid(), entry);
            });
            self.process_list
                .retain(|_, proc_| std::mem::replace(&mut proc_.updated, false));
        }
    }

    #[cfg(any(target_os = "ios", feature = "apple-sandbox"))]
    fn refresh_process_specifics(&mut self, _pid: Pid, _refresh_kind: ProcessRefreshKind) -> bool {
        false
    }

    #[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
    fn refresh_process_specifics(&mut self, pid: Pid, refresh_kind: ProcessRefreshKind) -> bool {
        let now = get_now();
        let arg_max = get_arg_max();
        let port = self.port;
        let time_interval = self.clock_info.as_mut().map(|c| c.get_time_interval(port));
        match {
            let wrap = Wrap(UnsafeCell::new(&mut self.process_list));
            update_process(
                &wrap,
                pid,
                arg_max as size_t,
                time_interval,
                now,
                refresh_kind,
                true,
            )
        } {
            Ok(Some(p)) => {
                self.process_list.insert(p.pid(), p);
                true
            }
            Ok(_) => true,
            Err(_) => false,
        }
    }

    #[cfg(target_os = "ios")]
    fn refresh_disks_list(&mut self) {}

    #[cfg(target_os = "macos")]
    fn refresh_disks_list(&mut self) {
        unsafe {
            if self.session.0.is_null() {
                self.session.0 = ffi::DASessionCreate(kCFAllocatorDefault as _);
            }
            self.disks = get_disks(self.session.0);
        }
    }

    fn refresh_users_list(&mut self) {
        self.users = crate::apple::users::get_users_list();
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

    fn global_cpu_info(&self) -> &Cpu {
        &self.global_cpu
    }

    fn cpus(&self) -> &[Cpu] {
        &self.cpus
    }

    fn physical_core_count(&self) -> Option<usize> {
        let mut physical_core_count = 0;

        unsafe {
            if get_sys_value_by_name(
                b"hw.physicalcpu\0",
                &mut mem::size_of::<u32>(),
                &mut physical_core_count as *mut usize as *mut c_void,
            ) {
                Some(physical_core_count)
            } else {
                None
            }
        }
    }

    fn networks(&self) -> &Networks {
        &self.networks
    }

    fn networks_mut(&mut self) -> &mut Networks {
        &mut self.networks
    }

    fn total_memory(&self) -> u64 {
        self.mem_total
    }

    fn free_memory(&self) -> u64 {
        self.mem_free
    }

    fn available_memory(&self) -> u64 {
        self.mem_available
    }

    fn used_memory(&self) -> u64 {
        self.mem_total - self.mem_free
    }

    fn total_swap(&self) -> u64 {
        self.swap_total
    }

    fn free_swap(&self) -> u64 {
        self.swap_free
    }

    // TODO: need to be checked
    fn used_swap(&self) -> u64 {
        self.swap_total - self.swap_free
    }

    #[cfg(any(
        target_os = "ios",
        all(target_os = "macos", any(target_arch = "x86", target_arch = "x86_64"))
    ))]
    fn components(&self) -> &[Component] {
        &self.components
    }

    #[cfg(all(
        target_os = "macos",
        not(feature = "apple-sandbox"),
        target_arch = "aarch64"
    ))]
    fn components(&self) -> &[Component] {
        &self.components.inner
    }

    #[cfg(any(
        target_os = "ios",
        all(target_os = "macos", any(target_arch = "x86", target_arch = "x86_64"))
    ))]
    fn components_mut(&mut self) -> &mut [Component] {
        &mut self.components
    }

    #[cfg(all(
        target_os = "macos",
        not(feature = "apple-sandbox"),
        target_arch = "aarch64"
    ))]
    fn components_mut(&mut self) -> &mut [Component] {
        &mut self.components.inner
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

    fn boot_time(&self) -> u64 {
        self.boot_time
    }

    fn name(&self) -> Option<String> {
        get_system_info(libc::KERN_OSTYPE, Some("Darwin"))
    }

    fn long_os_version(&self) -> Option<String> {
        #[cfg(target_os = "macos")]
        let friendly_name = match self.os_version().unwrap_or_default() {
            f_n if f_n.starts_with("10.16")
                | f_n.starts_with("11.0")
                | f_n.starts_with("11.1")
                | f_n.starts_with("11.2") =>
            {
                "Big Sur"
            }
            f_n if f_n.starts_with("10.15") => "Catalina",
            f_n if f_n.starts_with("10.14") => "Mojave",
            f_n if f_n.starts_with("10.13") => "High Sierra",
            f_n if f_n.starts_with("10.12") => "Sierra",
            f_n if f_n.starts_with("10.11") => "El Capitan",
            f_n if f_n.starts_with("10.10") => "Yosemite",
            f_n if f_n.starts_with("10.9") => "Mavericks",
            f_n if f_n.starts_with("10.8") => "Mountain Lion",
            f_n if f_n.starts_with("10.7") => "Lion",
            f_n if f_n.starts_with("10.6") => "Snow Leopard",
            f_n if f_n.starts_with("10.5") => "Leopard",
            f_n if f_n.starts_with("10.4") => "Tiger",
            f_n if f_n.starts_with("10.3") => "Panther",
            f_n if f_n.starts_with("10.2") => "Jaguar",
            f_n if f_n.starts_with("10.1") => "Puma",
            f_n if f_n.starts_with("10.0") => "Cheetah",
            _ => "",
        };

        #[cfg(target_os = "macos")]
        let long_name = Some(format!(
            "MacOS {} {}",
            self.os_version().unwrap_or_default(),
            friendly_name
        ));

        #[cfg(target_os = "ios")]
        let long_name = Some(format!("iOS {}", self.os_version().unwrap_or_default()));

        long_name
    }

    fn host_name(&self) -> Option<String> {
        get_system_info(libc::KERN_HOSTNAME, None)
    }

    fn kernel_version(&self) -> Option<String> {
        get_system_info(libc::KERN_OSRELEASE, None)
    }

    fn os_version(&self) -> Option<String> {
        unsafe {
            // get the size for the buffer first
            let mut size = 0;
            if get_sys_value_by_name(b"kern.osproductversion\0", &mut size, std::ptr::null_mut())
                && size > 0
            {
                // now create a buffer with the size and get the real value
                let mut buf = vec![0_u8; size as usize];

                if get_sys_value_by_name(
                    b"kern.osproductversion\0",
                    &mut size,
                    buf.as_mut_ptr() as *mut c_void,
                ) {
                    if let Some(pos) = buf.iter().position(|x| *x == 0) {
                        // Shrink buffer to terminate the null bytes
                        buf.resize(pos, 0);
                    }

                    String::from_utf8(buf).ok()
                } else {
                    // getting the system value failed
                    None
                }
            } else {
                // getting the system value failed, or did not return a buffer size
                None
            }
        }
    }
}

impl Default for System {
    fn default() -> System {
        System::new()
    }
}

// code from https://github.com/Chris911/iStats
// Not supported on iOS, or in the default macOS
#[cfg(all(
    target_os = "macos",
    not(feature = "apple-sandbox"),
    any(target_arch = "x86", target_arch = "x86_64")
))]
fn get_io_service_connection() -> Option<ffi::io_connect_t> {
    let mut master_port: mach_port_t = 0;
    let mut iterator: ffi::io_iterator_t = 0;

    unsafe {
        ffi::IOMasterPort(libc::MACH_PORT_NULL, &mut master_port);

        let matching_dictionary = ffi::IOServiceMatching(b"AppleSMC\0".as_ptr() as *const i8);
        let result =
            ffi::IOServiceGetMatchingServices(master_port, matching_dictionary, &mut iterator);
        if result != ffi::KIO_RETURN_SUCCESS {
            sysinfo_debug!("Error: IOServiceGetMatchingServices() = {}", result);
            return None;
        }

        let device = ffi::IOIteratorNext(iterator);
        ffi::IOObjectRelease(iterator);
        if device == 0 {
            sysinfo_debug!("Error: no SMC found");
            return None;
        }

        let mut conn = 0;
        let result = ffi::IOServiceOpen(device, libc::mach_task_self(), 0, &mut conn);
        ffi::IOObjectRelease(device);
        if result != ffi::KIO_RETURN_SUCCESS {
            sysinfo_debug!("Error: IOServiceOpen() = {}", result);
            return None;
        }

        Some(conn)
    }
}

#[cfg(all(target_os = "macos", not(feature = "apple-sandbox")))]
fn get_arg_max() -> usize {
    let mut mib = [libc::CTL_KERN, libc::KERN_ARGMAX];
    let mut arg_max = 0i32;
    let mut size = mem::size_of::<c_int>();
    unsafe {
        if sysctl(
            mib.as_mut_ptr(),
            mib.len() as _,
            (&mut arg_max) as *mut i32 as *mut c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        ) == -1
        {
            4096 // We default to this value
        } else {
            arg_max as usize
        }
    }
}

pub(crate) unsafe fn get_sys_value(
    high: u32,
    low: u32,
    mut len: usize,
    value: *mut c_void,
    mib: &mut [i32; 2],
) -> bool {
    mib[0] = high as i32;
    mib[1] = low as i32;
    sysctl(
        mib.as_mut_ptr(),
        mib.len() as _,
        value,
        &mut len as *mut usize,
        std::ptr::null_mut(),
        0,
    ) == 0
}

unsafe fn get_sys_value_by_name(name: &[u8], len: &mut usize, value: *mut c_void) -> bool {
    sysctlbyname(
        name.as_ptr() as *const c_char,
        value,
        len,
        std::ptr::null_mut(),
        0,
    ) == 0
}

fn get_system_info(value: c_int, default: Option<&str>) -> Option<String> {
    let mut mib: [c_int; 2] = [libc::CTL_KERN, value];
    let mut size = 0;

    unsafe {
        // Call first to get size
        sysctl(
            mib.as_mut_ptr(),
            mib.len() as _,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        );

        // exit early if we did not update the size
        if size == 0 {
            default.map(|s| s.to_owned())
        } else {
            // set the buffer to the correct size
            let mut buf = vec![0_u8; size as usize];

            if sysctl(
                mib.as_mut_ptr(),
                mib.len() as _,
                buf.as_mut_ptr() as _,
                &mut size,
                std::ptr::null_mut(),
                0,
            ) == -1
            {
                // If command fails return default
                default.map(|s| s.to_owned())
            } else {
                if let Some(pos) = buf.iter().position(|x| *x == 0) {
                    // Shrink buffer to terminate the null bytes
                    buf.resize(pos, 0);
                }

                String::from_utf8(buf).ok()
            }
        }
    }
}
