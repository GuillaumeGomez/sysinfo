// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{
    CpuRefreshKind, LoadAvg, Networks, Pid, ProcessExt, ProcessRefreshKind, RefreshKind, SystemExt,
    User,
};
use winapi::um::winreg::HKEY_LOCAL_MACHINE;

use crate::sys::component::{self, Component};
use crate::sys::cpu::*;
use crate::sys::disk::{get_disks, Disk};
use crate::sys::process::{get_start_time, update_memory, Process};
use crate::sys::tools::*;
use crate::sys::users::get_users;
use crate::sys::utils::{get_now, get_reg_string_value, get_reg_value_u32};

use crate::utils::into_iter;

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::mem::{size_of, zeroed};
use std::time::{Duration, SystemTime};

use ntapi::ntexapi::{
    NtQuerySystemInformation, SystemProcessInformation, SYSTEM_PROCESS_INFORMATION,
};
use winapi::ctypes::wchar_t;
use winapi::shared::minwindef::{FALSE, TRUE};
use winapi::shared::ntdef::{PVOID, ULONG};
use winapi::shared::ntstatus::STATUS_INFO_LENGTH_MISMATCH;
use winapi::um::minwinbase::STILL_ACTIVE;
use winapi::um::processthreadsapi::GetExitCodeProcess;
use winapi::um::psapi::{GetPerformanceInfo, PERFORMANCE_INFORMATION};
use winapi::um::sysinfoapi::{
    ComputerNamePhysicalDnsHostname, GetComputerNameExW, GetTickCount64, GlobalMemoryStatusEx,
    MEMORYSTATUSEX,
};
use winapi::um::winnt::HANDLE;

declare_signals! {
    (),
    Signal::Kill => (),
    _ => None,
}

#[doc = include_str!("../../md_doc/system.md")]
pub struct System {
    process_list: HashMap<Pid, Process>,
    mem_total: u64,
    mem_available: u64,
    swap_total: u64,
    swap_used: u64,
    cpus: CpusWrapper,
    components: Vec<Component>,
    disks: Vec<Disk>,
    query: Option<Query>,
    networks: Networks,
    boot_time: u64,
    users: Vec<User>,
}

static WINDOWS_ELEVEN_BUILD_NUMBER: u32 = 22000;

impl System {
    fn is_windows_eleven(&self) -> bool {
        WINDOWS_ELEVEN_BUILD_NUMBER
            <= self
                .kernel_version()
                .unwrap_or_default()
                .parse()
                .unwrap_or(0)
    }
}

// Useful for parallel iterations.
struct Wrap<T>(T);

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl<T> Send for Wrap<T> {}
unsafe impl<T> Sync for Wrap<T> {}

unsafe fn boot_time() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs().saturating_sub(GetTickCount64() / 1_000),
        Err(_e) => {
            sysinfo_debug!("Failed to compute boot time: {:?}", _e);
            0
        }
    }
}

impl SystemExt for System {
    const IS_SUPPORTED: bool = true;
    const SUPPORTED_SIGNALS: &'static [Signal] = supported_signals();
    const MINIMUM_CPU_UPDATE_INTERVAL: Duration = Duration::from_millis(200);

    #[allow(non_snake_case)]
    fn new_with_specifics(refreshes: RefreshKind) -> System {
        let mut s = System {
            process_list: HashMap::with_capacity(500),
            mem_total: 0,
            mem_available: 0,
            swap_total: 0,
            swap_used: 0,
            cpus: CpusWrapper::new(),
            components: Vec::new(),
            disks: Vec::with_capacity(2),
            query: None,
            networks: Networks::new(),
            boot_time: unsafe { boot_time() },
            users: Vec::new(),
        };
        s.refresh_specifics(refreshes);
        s
    }

    fn refresh_cpu_specifics(&mut self, refresh_kind: CpuRefreshKind) {
        if self.query.is_none() {
            self.query = Query::new();
            if let Some(ref mut query) = self.query {
                add_english_counter(
                    r"\Processor(_Total)\% Idle Time".to_string(),
                    query,
                    get_key_used(self.cpus.global_cpu_mut()),
                    "tot_0".to_owned(),
                );
                for (pos, proc_) in self.cpus.iter_mut(refresh_kind).enumerate() {
                    add_english_counter(
                        format!(r"\Processor({pos})\% Idle Time"),
                        query,
                        get_key_used(proc_),
                        format!("{pos}_0"),
                    );
                }
            }
        }
        if let Some(ref mut query) = self.query {
            query.refresh();
            let mut total_idle_time = None;
            if let Some(ref key_used) = *get_key_used(self.cpus.global_cpu_mut()) {
                total_idle_time = Some(
                    query
                        .get(&key_used.unique_id)
                        .expect("global_key_idle disappeared"),
                );
            }
            if let Some(total_idle_time) = total_idle_time {
                self.cpus
                    .global_cpu_mut()
                    .set_cpu_usage(100.0 - total_idle_time);
            }
            for p in self.cpus.iter_mut(refresh_kind) {
                let mut idle_time = None;
                if let Some(ref key_used) = *get_key_used(p) {
                    idle_time = Some(
                        query
                            .get(&key_used.unique_id)
                            .expect("key_used disappeared"),
                    );
                }
                if let Some(idle_time) = idle_time {
                    p.set_cpu_usage(100.0 - idle_time);
                }
            }
            if refresh_kind.frequency() {
                self.cpus.get_frequencies();
            }
        }
    }

    fn refresh_memory(&mut self) {
        unsafe {
            let mut mem_info: MEMORYSTATUSEX = zeroed();
            mem_info.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
            GlobalMemoryStatusEx(&mut mem_info);
            self.mem_total = mem_info.ullTotalPhys as _;
            self.mem_available = mem_info.ullAvailPhys as _;
            let mut perf_info: PERFORMANCE_INFORMATION = zeroed();
            if GetPerformanceInfo(&mut perf_info, size_of::<PERFORMANCE_INFORMATION>() as u32)
                == TRUE
            {
                let swap_total = perf_info.PageSize.saturating_mul(
                    perf_info
                        .CommitLimit
                        .saturating_sub(perf_info.PhysicalTotal),
                );
                let swap_used = perf_info.PageSize.saturating_mul(
                    perf_info
                        .CommitTotal
                        .saturating_sub(perf_info.PhysicalTotal),
                );
                self.swap_total = swap_total as _;
                self.swap_used = swap_used as _;
            }
        }
    }

    fn refresh_components_list(&mut self) {
        self.components = component::get_components();
    }

    #[allow(clippy::map_entry)]
    fn refresh_process_specifics(&mut self, pid: Pid, refresh_kind: ProcessRefreshKind) -> bool {
        let now = get_now();
        let nb_cpus = self.cpus.len() as u64;

        if let Some(proc_) = self.process_list.get_mut(&pid) {
            if let Some(ret) = refresh_existing_process(proc_, nb_cpus, now, refresh_kind) {
                return ret;
            }
            // We need to re-make the process because the PID owner changed.
        }
        if let Some(mut p) = Process::new_from_pid(pid, now, refresh_kind) {
            p.update(refresh_kind, nb_cpus, now);
            p.updated = false;
            self.process_list.insert(pid, p);
            true
        } else {
            false
        }
    }

    #[allow(clippy::cast_ptr_alignment)]
    fn refresh_processes_specifics(&mut self, refresh_kind: ProcessRefreshKind) {
        // Windows 10 notebook requires at least 512KiB of memory to make it in one go
        let mut buffer_size: usize = 512 * 1024;
        let now = get_now();

        loop {
            let mut process_information: Vec<u8> = Vec::with_capacity(buffer_size);
            let mut cb_needed = 0;

            unsafe {
                process_information.set_len(buffer_size);
                let ntstatus = NtQuerySystemInformation(
                    SystemProcessInformation,
                    process_information.as_mut_ptr() as PVOID,
                    buffer_size as ULONG,
                    &mut cb_needed,
                );

                if ntstatus != STATUS_INFO_LENGTH_MISMATCH {
                    if ntstatus < 0 {
                        sysinfo_debug!(
                            "Couldn't get process infos: NtQuerySystemInformation returned {}",
                            ntstatus
                        );
                    }

                    // Parse the data block to get process information
                    let mut process_ids = Vec::with_capacity(500);
                    let mut process_information_offset = 0;
                    loop {
                        let p = process_information
                            .as_ptr()
                            .offset(process_information_offset)
                            as *const SYSTEM_PROCESS_INFORMATION;
                        let pi = &*p;

                        process_ids.push(Wrap(p));

                        if pi.NextEntryOffset == 0 {
                            break;
                        }

                        process_information_offset += pi.NextEntryOffset as isize;
                    }
                    let process_list = Wrap(UnsafeCell::new(&mut self.process_list));
                    let nb_cpus = if refresh_kind.cpu() {
                        self.cpus.len() as u64
                    } else {
                        0
                    };

                    #[cfg(feature = "multithread")]
                    use rayon::iter::ParallelIterator;

                    // TODO: instead of using parallel iterator only here, would be better to be
                    //       able to run it over `process_information` directly!
                    let processes = into_iter(process_ids)
                        .filter_map(|pi| {
                            let pi = *pi.0;
                            let pid = Pid(pi.UniqueProcessId as _);
                            if let Some(proc_) = (*process_list.0.get()).get_mut(&pid) {
                                if proc_
                                    .get_start_time()
                                    .map(|start| start == proc_.start_time())
                                    .unwrap_or(true)
                                {
                                    proc_.memory = pi.WorkingSetSize as _;
                                    proc_.virtual_memory = pi.VirtualSize as _;
                                    proc_.update(refresh_kind, nb_cpus, now);
                                    return None;
                                }
                                // If the PID owner changed, we need to recompute the whole process.
                                sysinfo_debug!("owner changed for PID {}", proc_.pid());
                            }
                            let name = get_process_name(&pi, pid);
                            let mut p = Process::new_full(
                                pid,
                                if pi.InheritedFromUniqueProcessId as usize != 0 {
                                    Some(Pid(pi.InheritedFromUniqueProcessId as _))
                                } else {
                                    None
                                },
                                pi.WorkingSetSize as _,
                                pi.VirtualSize as _,
                                name,
                                now,
                                refresh_kind,
                            );
                            p.update(refresh_kind, nb_cpus, now);
                            Some(p)
                        })
                        .collect::<Vec<_>>();
                    for p in processes.into_iter() {
                        self.process_list.insert(p.pid(), p);
                    }
                    self.process_list.retain(|_, v| {
                        let x = v.updated;
                        v.updated = false;
                        x
                    });

                    break;
                }

                // GetNewBufferSize
                if cb_needed == 0 {
                    buffer_size *= 2;
                    continue;
                }
                // allocating a few more kilo bytes just in case there are some new process
                // kicked in since new call to NtQuerySystemInformation
                buffer_size = (cb_needed + (1024 * 10)) as usize;
            }
        }
    }

    fn refresh_disks_list(&mut self) {
        self.disks = unsafe { get_disks() };
    }

    fn refresh_users_list(&mut self) {
        self.users = unsafe { get_users() };
    }

    fn processes(&self) -> &HashMap<Pid, Process> {
        &self.process_list
    }

    fn process(&self, pid: Pid) -> Option<&Process> {
        self.process_list.get(&pid)
    }

    fn global_cpu_info(&self) -> &Cpu {
        self.cpus.global_cpu()
    }

    fn cpus(&self) -> &[Cpu] {
        self.cpus.cpus()
    }

    fn physical_core_count(&self) -> Option<usize> {
        get_physical_core_count()
    }

    fn total_memory(&self) -> u64 {
        self.mem_total
    }

    fn free_memory(&self) -> u64 {
        // MEMORYSTATUSEX doesn't report free memory
        self.mem_available
    }

    fn available_memory(&self) -> u64 {
        self.mem_available
    }

    fn used_memory(&self) -> u64 {
        self.mem_total - self.mem_available
    }

    fn total_swap(&self) -> u64 {
        self.swap_total
    }

    fn free_swap(&self) -> u64 {
        self.swap_total - self.swap_used
    }

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

    fn users(&self) -> &[User] {
        &self.users
    }

    fn networks(&self) -> &Networks {
        &self.networks
    }

    fn networks_mut(&mut self) -> &mut Networks {
        &mut self.networks
    }

    fn uptime(&self) -> u64 {
        unsafe { GetTickCount64() / 1_000 }
    }

    fn boot_time(&self) -> u64 {
        self.boot_time
    }

    fn load_average(&self) -> LoadAvg {
        get_load_average()
    }

    fn name(&self) -> Option<String> {
        Some("Windows".to_owned())
    }

    fn long_os_version(&self) -> Option<String> {
        if self.is_windows_eleven() {
            return get_reg_string_value(
                HKEY_LOCAL_MACHINE,
                "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
                "ProductName",
            )
            .map(|product_name| product_name.replace("Windows 10 ", "Windows 11 "));
        }
        get_reg_string_value(
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
            "ProductName",
        )
    }

    fn host_name(&self) -> Option<String> {
        get_dns_hostname()
    }

    fn kernel_version(&self) -> Option<String> {
        get_reg_string_value(
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
            "CurrentBuildNumber",
        )
    }

    fn os_version(&self) -> Option<String> {
        let build_number = get_reg_string_value(
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
            "CurrentBuildNumber",
        )
        .unwrap_or_default();
        let major = if self.is_windows_eleven() {
            11u32
        } else {
            u32::from_le_bytes(
                get_reg_value_u32(
                    HKEY_LOCAL_MACHINE,
                    "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
                    "CurrentMajorVersionNumber",
                )
                .unwrap_or_default(),
            )
        };
        Some(format!("{major} ({build_number})"))
    }

    fn distribution_id(&self) -> String {
        std::env::consts::OS.to_owned()
    }
}

impl Default for System {
    fn default() -> System {
        System::new()
    }
}

pub(crate) fn is_proc_running(handle: HANDLE) -> bool {
    let mut exit_code = 0;
    unsafe {
        let ret = GetExitCodeProcess(handle, &mut exit_code);
        !(ret == FALSE || exit_code != STILL_ACTIVE)
    }
}

/// If it returns `None`, it means that the PID owner changed and that the `Process` must be
/// completely recomputed.
fn refresh_existing_process(
    proc_: &mut Process,
    nb_cpus: u64,
    now: u64,
    refresh_kind: ProcessRefreshKind,
) -> Option<bool> {
    if let Some(handle) = proc_.get_handle() {
        if get_start_time(handle) != proc_.start_time() {
            sysinfo_debug!("owner changed for PID {}", proc_.pid());
            // PID owner changed!
            return None;
        }
        if !is_proc_running(handle) {
            return Some(false);
        }
    } else {
        return Some(false);
    }
    update_memory(proc_);
    proc_.update(refresh_kind, nb_cpus, now);
    proc_.updated = false;
    Some(true)
}

#[allow(clippy::size_of_in_element_count)]
//^ needed for "name.Length as usize / std::mem::size_of::<u16>()"
pub(crate) fn get_process_name(process: &SYSTEM_PROCESS_INFORMATION, process_id: Pid) -> String {
    let name = &process.ImageName;
    if name.Buffer.is_null() {
        match process_id.0 {
            0 => "Idle".to_owned(),
            4 => "System".to_owned(),
            _ => format!("<no name> Process {process_id}"),
        }
    } else {
        unsafe {
            let slice = std::slice::from_raw_parts(
                name.Buffer,
                // The length is in bytes, not the length of string
                name.Length as usize / std::mem::size_of::<u16>(),
            );

            String::from_utf16_lossy(slice)
        }
    }
}

fn get_dns_hostname() -> Option<String> {
    let mut buffer_size = 0;
    // Running this first to get the buffer size since the DNS name can be longer than MAX_COMPUTERNAME_LENGTH
    // setting the `lpBuffer` to null will return the buffer size
    // https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getcomputernameexw
    unsafe {
        GetComputerNameExW(
            ComputerNamePhysicalDnsHostname,
            std::ptr::null_mut(),
            &mut buffer_size,
        );

        // Setting the buffer with the new length
        let mut buffer = vec![0_u16; buffer_size as usize];

        // https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/ne-sysinfoapi-computer_name_format
        if GetComputerNameExW(
            ComputerNamePhysicalDnsHostname,
            buffer.as_mut_ptr() as *mut wchar_t,
            &mut buffer_size,
        ) == TRUE
        {
            if let Some(pos) = buffer.iter().position(|c| *c == 0) {
                buffer.resize(pos, 0);
            }

            return String::from_utf16(&buffer).ok();
        }
    }

    sysinfo_debug!("Failed to get computer hostname");
    None
}
