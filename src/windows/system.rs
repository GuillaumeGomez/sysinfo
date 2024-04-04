// Take a look at the license at the top of the repository in the LICENSE file.

use crate::{Cpu, CpuRefreshKind, LoadAvg, MemoryRefreshKind, Pid, ProcessRefreshKind};

use crate::sys::cpu::*;
use crate::sys::process::get_start_time;
use crate::sys::tools::*;
use crate::sys::utils::{get_now, get_reg_string_value, get_reg_value_u32};
use crate::{Process, ProcessInner};

use crate::utils::into_iter;

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::ffi::OsString;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStringExt;
use std::ptr;
use std::time::SystemTime;

use bstr::ByteSlice;
use ntapi::ntexapi::SYSTEM_PROCESS_INFORMATION;
use windows::core::PWSTR;
use windows::Wdk::System::SystemInformation::{NtQuerySystemInformation, SystemProcessInformation};
use windows::Win32::Foundation::{HANDLE, STATUS_INFO_LENGTH_MISMATCH, STILL_ACTIVE};
use windows::Win32::System::ProcessStatus::{K32GetPerformanceInfo, PERFORMANCE_INFORMATION};
use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;
use windows::Win32::System::SystemInformation;
use windows::Win32::System::SystemInformation::{
    ComputerNamePhysicalDnsHostname, GetComputerNameExW, GetTickCount64, GlobalMemoryStatusEx,
    MEMORYSTATUSEX, SYSTEM_INFO,
};
use windows::Win32::System::Threading::GetExitCodeProcess;

const WINDOWS_ELEVEN_BUILD_NUMBER: u32 = 22000;

impl SystemInner {
    fn is_windows_eleven() -> bool {
        WINDOWS_ELEVEN_BUILD_NUMBER
            <= Self::kernel_version()
                .unwrap_or_default()
                .to_str()
                .and_then(|s| s.parse().ok())
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

pub(crate) struct SystemInner {
    process_list: HashMap<Pid, Process>,
    mem_total: u64,
    mem_available: u64,
    swap_total: u64,
    swap_used: u64,
    cpus: CpusWrapper,
    query: Option<Query>,
}

impl SystemInner {
    pub(crate) fn new() -> Self {
        Self {
            process_list: HashMap::with_capacity(500),
            mem_total: 0,
            mem_available: 0,
            swap_total: 0,
            swap_used: 0,
            cpus: CpusWrapper::new(),
            query: None,
        }
    }

    pub(crate) fn refresh_cpu_specifics(&mut self, refresh_kind: CpuRefreshKind) {
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
                    .inner
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
                    p.inner.set_cpu_usage(100.0 - idle_time);
                }
            }
            if refresh_kind.frequency() {
                self.cpus.get_frequencies();
            }
        }
    }

    pub(crate) fn refresh_memory_specifics(&mut self, refresh_kind: MemoryRefreshKind) {
        unsafe {
            if refresh_kind.ram() {
                let mut mem_info: MEMORYSTATUSEX = zeroed();
                mem_info.dwLength = size_of::<MEMORYSTATUSEX>() as _;
                let _err = GlobalMemoryStatusEx(&mut mem_info);
                self.mem_total = mem_info.ullTotalPhys as _;
                self.mem_available = mem_info.ullAvailPhys as _;
            }
            if refresh_kind.swap() {
                let mut perf_info: PERFORMANCE_INFORMATION = zeroed();
                if K32GetPerformanceInfo(&mut perf_info, size_of::<PERFORMANCE_INFORMATION>() as _)
                    .as_bool()
                {
                    let page_size = perf_info.PageSize as u64;
                    let physical_total = perf_info.PhysicalTotal as u64;
                    let commit_limit = perf_info.CommitLimit as u64;
                    let commit_total = perf_info.CommitTotal as u64;
                    self.swap_total =
                        page_size.saturating_mul(commit_limit.saturating_sub(physical_total));
                    self.swap_used =
                        page_size.saturating_mul(commit_total.saturating_sub(physical_total));
                }
            }
        }
    }

    pub(crate) fn cgroup_limits(&self) -> Option<crate::CGroupLimits> {
        None
    }

    #[allow(clippy::map_entry)]
    pub(crate) fn refresh_process_specifics(
        &mut self,
        pid: Pid,
        refresh_kind: ProcessRefreshKind,
    ) -> bool {
        let now = get_now();
        let nb_cpus = self.cpus.len() as u64;

        if let Some(proc_) = self.process_list.get_mut(&pid) {
            if let Some(ret) = refresh_existing_process(proc_, nb_cpus, now, refresh_kind) {
                return ret;
            }
            // We need to re-make the process because the PID owner changed.
        }
        if let Some(mut p) = ProcessInner::new_from_pid(pid, now, refresh_kind) {
            p.update(refresh_kind, nb_cpus, now);
            p.updated = false;
            self.process_list.insert(pid, Process { inner: p });
            true
        } else {
            false
        }
    }

    #[allow(clippy::cast_ptr_alignment)]
    pub(crate) fn refresh_processes_specifics(
        &mut self,
        filter: Option<&[Pid]>,
        refresh_kind: ProcessRefreshKind,
    ) {
        // Windows 10 notebook requires at least 512KiB of memory to make it in one go
        let mut buffer_size = 512 * 1024;
        let mut process_information: Vec<u8> = Vec::with_capacity(buffer_size);

        unsafe {
            loop {
                let mut cb_needed = 0;
                // reserve(n) ensures the Vec has capacity for n elements on top of len
                // so we should reserve buffer_size - len. len will always be zero at this point
                // this is a no-op on the first call as buffer_size == capacity
                process_information.reserve(buffer_size);

                match NtQuerySystemInformation(
                    SystemProcessInformation,
                    process_information.as_mut_ptr() as *mut _,
                    buffer_size as _,
                    &mut cb_needed,
                )
                .ok()
                {
                    Ok(()) => break,
                    Err(err) if err.code() == STATUS_INFO_LENGTH_MISMATCH.to_hresult() => {
                        // GetNewBufferSize
                        if cb_needed == 0 {
                            buffer_size *= 2;
                            continue;
                        }
                        // allocating a few more kilo bytes just in case there are some new process
                        // kicked in since new call to NtQuerySystemInformation
                        buffer_size = (cb_needed + (1024 * 10)) as usize;
                        continue;
                    }
                    Err(_err) => {
                        sysinfo_debug!(
                            "Couldn't get process infos: NtQuerySystemInformation returned {}",
                            _err,
                        );
                        return;
                    }
                }
            }

            #[inline(always)]
            fn real_filter(e: Pid, filter: &[Pid]) -> bool {
                filter.contains(&e)
            }

            #[inline(always)]
            fn empty_filter(_e: Pid, _filter: &[Pid]) -> bool {
                true
            }

            #[allow(clippy::type_complexity)]
            let (filter, filter_callback): (
                &[Pid],
                &(dyn Fn(Pid, &[Pid]) -> bool + Sync + Send),
            ) = if let Some(filter) = filter {
                (filter, &real_filter)
            } else {
                (&[], &empty_filter)
            };

            // If we reach this point NtQuerySystemInformation succeeded
            // and the buffer contents are initialized
            process_information.set_len(buffer_size);

            // Parse the data block to get process information
            let mut process_ids = Vec::with_capacity(500);
            let mut process_information_offset = 0;
            loop {
                let p = process_information
                    .as_ptr()
                    .offset(process_information_offset)
                    as *const SYSTEM_PROCESS_INFORMATION;

                // read_unaligned is necessary to avoid
                // misaligned pointer dereference: address must be a multiple of 0x8 but is 0x...
                // under x86_64 wine (and possibly other systems)
                let pi = ptr::read_unaligned(p);

                if filter_callback(Pid(pi.UniqueProcessId as _), filter) {
                    process_ids.push(Wrap(p));
                }

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

            let now = get_now();

            #[cfg(feature = "multithread")]
            use rayon::iter::ParallelIterator;

            // TODO: instead of using parallel iterator only here, would be better to be
            //       able to run it over `process_information` directly!
            let processes = into_iter(process_ids)
                .filter_map(|pi| {
                    // as above, read_unaligned is necessary
                    let pi = ptr::read_unaligned(pi.0);
                    let pid = Pid(pi.UniqueProcessId as _);
                    if let Some(proc_) = (*process_list.0.get()).get_mut(&pid) {
                        let proc_ = &mut proc_.inner;
                        if proc_
                            .get_start_time()
                            .map(|start| start == proc_.start_time())
                            .unwrap_or(true)
                        {
                            if refresh_kind.memory() {
                                proc_.memory = pi.WorkingSetSize as _;
                                proc_.virtual_memory = pi.VirtualSize as _;
                            }
                            proc_.update(refresh_kind, nb_cpus, now);
                            return None;
                        }
                        // If the PID owner changed, we need to recompute the whole process.
                        sysinfo_debug!("owner changed for PID {}", proc_.pid());
                    }
                    let name = get_process_name(&pi, pid);
                    let (memory, virtual_memory) = if refresh_kind.memory() {
                        (pi.WorkingSetSize as _, pi.VirtualSize as _)
                    } else {
                        (0, 0)
                    };
                    let mut p = ProcessInner::new_full(
                        pid,
                        if pi.InheritedFromUniqueProcessId as usize != 0 {
                            Some(Pid(pi.InheritedFromUniqueProcessId as _))
                        } else {
                            None
                        },
                        memory,
                        virtual_memory,
                        name,
                        now,
                        refresh_kind,
                    );
                    p.update(refresh_kind.without_memory(), nb_cpus, now);
                    Some(Process { inner: p })
                })
                .collect::<Vec<_>>();
            for p in processes.into_iter() {
                self.process_list.insert(p.pid(), p);
            }
            self.process_list.retain(|_, v| {
                let x = v.inner.updated;
                v.inner.updated = false;
                x
            });
        }
    }

    pub(crate) fn processes(&self) -> &HashMap<Pid, Process> {
        &self.process_list
    }

    pub(crate) fn process(&self, pid: Pid) -> Option<&Process> {
        self.process_list.get(&pid)
    }

    pub(crate) fn global_cpu_info(&self) -> &Cpu {
        self.cpus.global_cpu()
    }

    pub(crate) fn cpus(&self) -> &[Cpu] {
        self.cpus.cpus()
    }

    pub(crate) fn physical_core_count(&self) -> Option<usize> {
        get_physical_core_count()
    }

    pub(crate) fn total_memory(&self) -> u64 {
        self.mem_total
    }

    pub(crate) fn free_memory(&self) -> u64 {
        // MEMORYSTATUSEX doesn't report free memory
        self.mem_available
    }

    pub(crate) fn available_memory(&self) -> u64 {
        self.mem_available
    }

    pub(crate) fn used_memory(&self) -> u64 {
        self.mem_total - self.mem_available
    }

    pub(crate) fn total_swap(&self) -> u64 {
        self.swap_total
    }

    pub(crate) fn free_swap(&self) -> u64 {
        self.swap_total - self.swap_used
    }

    pub(crate) fn used_swap(&self) -> u64 {
        self.swap_used
    }

    pub(crate) fn uptime() -> u64 {
        unsafe { GetTickCount64() / 1_000 }
    }

    pub(crate) fn boot_time() -> u64 {
        unsafe { boot_time() }
    }

    pub(crate) fn load_average() -> LoadAvg {
        get_load_average()
    }

    pub(crate) fn name() -> Option<OsString> {
        Some("Windows".into())
    }

    pub(crate) fn long_os_version() -> Option<OsString> {
        if Self::is_windows_eleven() {
            return get_reg_string_value(
                HKEY_LOCAL_MACHINE,
                r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
                "ProductName",
            )
            .map(|product_name| {
                // SAFETY: We only replace the Windows version, which is valid
                // UTF-8. Since OsString is a superset of UTF-8 and we only
                // modify the valid UTF-8 bytes, the OsString stays correctly
                // encoded.
                unsafe {
                    OsString::from_encoded_bytes_unchecked(
                        product_name
                            .into_encoded_bytes()
                            .replace("Windows 10 ", "Windows 11 "),
                    )
                }
            });
        }
        get_reg_string_value(
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
            "ProductName",
        )
    }

    pub(crate) fn host_name() -> Option<OsString> {
        get_dns_hostname()
    }

    pub(crate) fn kernel_version() -> Option<OsString> {
        get_reg_string_value(
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
            "CurrentBuildNumber",
        )
    }

    pub(crate) fn os_version() -> Option<OsString> {
        let build_number = get_reg_string_value(
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
            "CurrentBuildNumber",
        )
        .unwrap_or_default();
        let major = if Self::is_windows_eleven() {
            11u32
        } else {
            u32::from_le_bytes(
                get_reg_value_u32(
                    HKEY_LOCAL_MACHINE,
                    r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
                    "CurrentMajorVersionNumber",
                )
                .unwrap_or_default(),
            )
        };

        unsafe {
            let mut buf = format!("{major} (").into_bytes();
            buf.extend(build_number.into_encoded_bytes());
            buf.push(b')');

            Some(OsString::from_encoded_bytes_unchecked(buf))
        }
    }

    pub(crate) fn distribution_id() -> OsString {
        std::env::consts::OS.into()
    }
    pub(crate) fn cpu_arch() -> Option<OsString> {
        unsafe {
            // https://docs.microsoft.com/fr-fr/windows/win32/api/sysinfoapi/ns-sysinfoapi-system_info
            let info = SYSTEM_INFO::default();
            match info.Anonymous.Anonymous.wProcessorArchitecture {
                SystemInformation::PROCESSOR_ARCHITECTURE_ALPHA => Some("alpha".into()),
                SystemInformation::PROCESSOR_ARCHITECTURE_ALPHA64 => Some("alpha64".into()),
                SystemInformation::PROCESSOR_ARCHITECTURE_AMD64 => Some("x86_64".into()),
                SystemInformation::PROCESSOR_ARCHITECTURE_ARM => Some("arm".into()),
                SystemInformation::PROCESSOR_ARCHITECTURE_ARM32_ON_WIN64 => Some("arm".into()),
                SystemInformation::PROCESSOR_ARCHITECTURE_ARM64 => Some("arm64".into()),
                SystemInformation::PROCESSOR_ARCHITECTURE_IA32_ON_ARM64
                | SystemInformation::PROCESSOR_ARCHITECTURE_IA32_ON_WIN64 => Some("ia32".into()),
                SystemInformation::PROCESSOR_ARCHITECTURE_IA64 => Some("ia64".into()),
                SystemInformation::PROCESSOR_ARCHITECTURE_INTEL => Some("x86".into()),
                SystemInformation::PROCESSOR_ARCHITECTURE_MIPS => Some("mips".into()),
                SystemInformation::PROCESSOR_ARCHITECTURE_PPC => Some("powerpc".into()),
                _ => None,
            }
        }
    }
}

pub(crate) fn is_proc_running(handle: HANDLE) -> bool {
    let mut exit_code = 0;
    unsafe { GetExitCodeProcess(handle, &mut exit_code) }.is_ok()
        && exit_code == STILL_ACTIVE.0 as u32
}

/// If it returns `None`, it means that the PID owner changed and that the `Process` must be
/// completely recomputed.
fn refresh_existing_process(
    proc_: &mut Process,
    nb_cpus: u64,
    now: u64,
    refresh_kind: ProcessRefreshKind,
) -> Option<bool> {
    let proc_ = &mut proc_.inner;
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
    proc_.update(refresh_kind, nb_cpus, now);
    proc_.updated = false;
    Some(true)
}

#[allow(clippy::size_of_in_element_count)]
//^ needed for "name.Length as usize / std::mem::size_of::<u16>()"
pub(crate) fn get_process_name(process: &SYSTEM_PROCESS_INFORMATION, process_id: Pid) -> OsString {
    let name = &process.ImageName;
    if name.Buffer.is_null() {
        match process_id.0 {
            0 => "Idle".to_owned(),
            4 => "System".to_owned(),
            _ => format!("<no name> Process {process_id}"),
        }
        .into()
    } else {
        unsafe {
            let slice = std::slice::from_raw_parts(
                name.Buffer,
                // The length is in bytes, not the length of string
                name.Length as usize / std::mem::size_of::<u16>(),
            );

            OsString::from_wide(slice)
        }
    }
}

fn get_dns_hostname() -> Option<OsString> {
    let mut buffer_size = 0;
    // Running this first to get the buffer size since the DNS name can be longer than MAX_COMPUTERNAME_LENGTH
    // setting the `lpBuffer` to null will return the buffer size
    // https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getcomputernameexw
    unsafe {
        let _err = GetComputerNameExW(
            ComputerNamePhysicalDnsHostname,
            PWSTR::null(),
            &mut buffer_size,
        );

        // Setting the buffer with the new length
        let mut buffer = vec![0_u16; buffer_size as usize];

        // https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/ne-sysinfoapi-computer_name_format
        if GetComputerNameExW(
            ComputerNamePhysicalDnsHostname,
            PWSTR::from_raw(buffer.as_mut_ptr()),
            &mut buffer_size,
        )
        .is_ok()
        {
            if let Some(pos) = buffer.iter().position(|c| *c == 0) {
                buffer.resize(pos, 0);
            }

            return Some(OsString::from_wide(&buffer));
        }
    }

    sysinfo_debug!("Failed to get computer hostname");
    None
}
