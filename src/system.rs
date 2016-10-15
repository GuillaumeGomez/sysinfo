// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

use ffi;
use Component;
use processor::*;
use process::*;
use std::fs::{File, read_link};
use std::io::Read;
use std::str::FromStr;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use libc::{self, c_void, c_int, size_t, stat, lstat, c_char, sysconf, _SC_CLK_TCK, _SC_PAGESIZE, S_IFLNK, S_IFMT};
#[cfg(target_os = "macos")]
use std::rc::Rc;
#[cfg(target_os = "macos")]
use processor;

pub struct System {
    process_list: HashMap<usize, Process>,
    mem_total: u64,
    mem_free: u64,
    swap_total: u64,
    swap_free: u64,
    processors: Vec<Processor>,
    page_size_kb: u64,
    temperatures: Vec<Component>,
}

fn get_string_from_array(ar: &[u8]) -> String {
    let mut it = 0;

    while it < ar.len() && ar[it] != 0 {
        it += 1;
    }
    let mut tmp = ar.to_vec();
    unsafe { tmp.set_len(it); }
    unsafe { String::from_utf8_unchecked(tmp) }
}

unsafe fn get_unchecked_str(cp: *mut u8, start: *mut u8) -> String {
    let len = cp as usize - start as usize;
    let part = Vec::from_raw_parts(start, len, len);
    let tmp = String::from_utf8_unchecked(part.clone());
    ::std::mem::forget(part);
    tmp
}

impl System {
    pub fn new() -> System {
        let mut s = System {
            process_list: HashMap::new(),
            mem_total: 0,
            mem_free: 0,
            swap_total: 0,
            swap_free: 0,
            processors: Vec::new(),
            page_size_kb: unsafe { sysconf(_SC_PAGESIZE) as u64 / 1024 },
            temperatures: Component::get_components(),
        };
        s.refresh_all();
        s
    }

    #[cfg(target_os = "macos")]
    pub fn refresh_system(&mut self) {
        unsafe fn get_sys_value(high: u32, low: u32, mut len: usize, value: *mut c_void) -> bool {
            let mut mib = [high as i32, low as i32];
            ffi::sysctl(mib.as_mut_ptr(), 2, value, &mut len as *mut usize,
                        ::std::ptr::null_mut(), 0) == 0
        }

        unsafe {
            // get system values
            if self.mem_total < 1 {
                get_sys_value(ffi::CTL_HW, ffi::HW_MEMSIZE, ::std::mem::size_of::<u64>(),
                              &mut self.mem_total as *mut u64 as *mut c_void);
                self.mem_total /= 1024;
            }
            let count: u32 = ffi::HOST_VM_INFO64_COUNT;
            let mut stat = ::std::mem::zeroed::<ffi::vm_statistics64>();
            if ffi::host_statistics64(ffi::mach_host_self(), ffi::HOST_VM_INFO64,
                                      &mut stat as *mut ffi::vm_statistics64 as *mut c_void,
                                      &count as *const u32) == ffi::KERN_SUCCESS {
                self.mem_free = (stat.free_count + stat.inactive_count
                    + stat.speculative_count) as u64 * self.page_size_kb;
            }

            // get processor values
            let mut numCPUsU = 0u32;
            let mut cpuInfo: *mut i32 = ::std::ptr::null_mut();
            let mut numCpuInfo = 0u32;

            if self.processors.len() == 0 {
                let mut numCPUs = 0;

                let mut mib = [ffi::CTL_HW, ffi::HW_NCPU];
                let mut sizeOfNumCPUs = ::std::mem::size_of::<u32>();
                if get_sys_value(ffi::CTL_HW, ffi::HW_NCPU, ::std::mem::size_of::<u32>(),
                                 &mut numCPUs as *mut usize as *mut c_void) == false {
                    numCPUs = 1;
                }

                self.processors.push(
                    processor::create_proc("0".to_owned(),
                                           Rc::new(ProcessorData::new(::std::ptr::null_mut(), 0))));
                if ffi::host_processor_info(ffi::mach_host_self(), ffi::PROCESSOR_CPU_LOAD_INFO,
                                       &mut numCPUsU as *mut u32,
                                       &mut cpuInfo as *mut *mut i32,
                                       &mut numCpuInfo as *mut u32) == ffi::KERN_SUCCESS {
                    let mut proc_data = Rc::new(ProcessorData::new(cpuInfo, numCpuInfo));
                    for i in 0..numCPUs {
                        let mut p = processor::create_proc(format!("{}", i + 1), proc_data.clone());
                        let inUse = *cpuInfo.offset((ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_USER as isize)
                            + *cpuInfo.offset((ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_SYSTEM as isize)
                            + *cpuInfo.offset((ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_NICE as isize);
                        let total = inUse + *cpuInfo.offset((ffi::CPU_STATE_MAX * i) as isize + ffi::CPU_STATE_IDLE as isize);
                        processor::set_cpu_proc(&mut p, inUse as f32 / total as f32);
                        self.processors.push(p);
                    }
                }
            } else if ffi::host_processor_info(ffi::mach_host_self(), ffi::PROCESSOR_CPU_LOAD_INFO,
                                               &mut numCPUsU as *mut u32,
                                               &mut cpuInfo as *mut *mut i32,
                                               &mut numCpuInfo as *mut u32) == ffi::KERN_SUCCESS {
                let mut pourcent = 0f32;
                let mut proc_data = Rc::new(ProcessorData::new(cpuInfo, numCpuInfo));
                for (i, proc_) in self.processors.iter_mut().skip(1).enumerate() {
                    let old_proc_data = &*processor::get_processor_data(proc_);
                    let inUse = (*cpuInfo.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_USER as isize)
                            - *old_proc_data.cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_USER as isize))
                        + (*cpuInfo.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_SYSTEM as isize)
                            - *old_proc_data.cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_SYSTEM as isize))
                        + (*cpuInfo.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_NICE as isize)
                            - *old_proc_data.cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_NICE as isize));
                    let total = inUse + (*cpuInfo.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_IDLE as isize)
                        - *old_proc_data.cpu_info.offset((ffi::CPU_STATE_MAX * i) as isize
                            + ffi::CPU_STATE_IDLE as isize));
                    processor::update_proc(proc_, inUse as f32 / total as f32, proc_data.clone());
                    pourcent += proc_.get_cpu_usage();
                }
                if self.processors.len() > 1 {
                    let len = self.processors.len() - 1;
                    if let Some(p) = self.processors.get_mut(0) {
                        processor::set_cpu_usage(p, pourcent / len as f32);
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn refresh_system(&mut self) {
        let data = get_all_data("/proc/meminfo");
        let lines: Vec<&str> = data.split('\n').collect();

        for component in self.temperatures.iter_mut() {
            component.update();
        }
        for line in lines.iter() {
            match *line {
                l if l.starts_with("MemTotal:") => {
                    let parts: Vec<&str> = line.split(' ').collect();

                    self.mem_total = u64::from_str(parts[parts.len() - 2]).unwrap();
                },
                l if l.starts_with("MemAvailable:") => {
                    let parts: Vec<&str> = line.split(' ').collect();

                    self.mem_free = u64::from_str(parts[parts.len() - 2]).unwrap();
                },
                l if l.starts_with("SwapTotal:") => {
                    let parts: Vec<&str> = line.split(' ').collect();

                    self.swap_total = u64::from_str(parts[parts.len() - 2]).unwrap();
                },
                l if l.starts_with("SwapFree:") => {
                    let parts: Vec<&str> = line.split(' ').collect();

                    self.swap_free = u64::from_str(parts[parts.len() - 2]).unwrap();
                },
                _ => continue,
            }
        }
        let data = get_all_data("/proc/stat");
        let lines: Vec<&str> = data.split('\n').collect();
        let mut i = 0;
        let first = self.processors.len() == 0;
        for line in lines.iter() {
            if !line.starts_with("cpu") {
                break;
            }

            let (parts, _): (Vec<&str>, Vec<&str>) = line.split(' ').partition(|s| s.len() > 0);
            if first {
                self.processors.push(new_processor(parts[0], u64::from_str(parts[1]).unwrap(),
                    u64::from_str(parts[2]).unwrap(),
                    u64::from_str(parts[3]).unwrap(),
                    u64::from_str(parts[4]).unwrap(),
                    u64::from_str(parts[5]).unwrap(),
                    u64::from_str(parts[6]).unwrap(),
                    u64::from_str(parts[7]).unwrap(),
                    u64::from_str(parts[8]).unwrap(),
                    u64::from_str(parts[9]).unwrap(),
                    u64::from_str(parts[10]).unwrap()));
            } else {
                set_processor(self.processors.get_mut(i).unwrap(),
                    u64::from_str(parts[1]).unwrap(),
                    u64::from_str(parts[2]).unwrap(),
                    u64::from_str(parts[3]).unwrap(),
                    u64::from_str(parts[4]).unwrap(),
                    u64::from_str(parts[5]).unwrap(),
                    u64::from_str(parts[6]).unwrap(),
                    u64::from_str(parts[7]).unwrap(),
                    u64::from_str(parts[8]).unwrap(),
                    u64::from_str(parts[9]).unwrap(),
                    u64::from_str(parts[10]).unwrap());
                i += 1;
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub fn refresh_process(&mut self) {
        let count = unsafe { ffi::proc_listallpids(::std::ptr::null_mut(), 0) };
        if count < 1 {
            return;
        }
        let mut pids: Vec<libc::pid_t> = Vec::with_capacity(count as usize);
        unsafe { pids.set_len(count as usize); }
        let count = count * ::std::mem::size_of::<libc::pid_t>() as i32;
        let x = unsafe { ffi::proc_listallpids(pids.as_mut_ptr() as *mut c_void, count) };

        if x < 1 || x as usize > pids.len() {
            return;
        } else if pids.len() > x as usize {
            unsafe { pids.set_len(x as usize); }
        }

        let taskallinfo_size = ::std::mem::size_of::<ffi::proc_taskallinfo>() as i32;
        let taskinfo_size = ::std::mem::size_of::<ffi::proc_taskinfo>() as i32;
        let threadinfo_size = ::std::mem::size_of::<ffi::proc_threadinfo>() as i32;
        let name_len = 2 * ffi::MAXCOMLEN;
        let mut name: Vec<u8> = Vec::with_capacity(name_len as usize);
        let mut cmd_path: Vec<u8> = Vec::with_capacity(ffi::MAXPATHLEN);

        let mut mib: [c_int; 3] = [ffi::CTL_KERN, ffi::KERN_ARGMAX, 0];
        let mut argmax = 0;
        let mut size = ::std::mem::size_of::<c_int>();
        unsafe {
            while ffi::sysctl(mib.as_mut_ptr(), 2, (&mut argmax) as *mut i32 as *mut c_void,
                              &mut size, ::std::ptr::null_mut(), 0) == -1 {}
        }
        let mut proc_args = Vec::with_capacity(argmax as usize);

        for pid in pids {
            unsafe {
                let mut thread_info = ::std::mem::zeroed::<ffi::proc_threadinfo>();
                let mut user_time = 0;
                let mut system_time = 0;
                if ffi::proc_pidinfo(pid,
                                     ffi::PROC_PIDTHREADINFO,
                                     0,
                                     &mut thread_info as *mut ffi::proc_threadinfo as *mut c_void,
                                     threadinfo_size) != 0 {
                    user_time = thread_info.pth_user_time;
                    system_time = thread_info.pth_system_time;
                }
                if let Some(ref mut p) = self.process_list.get_mut(&(pid as usize)) {
                    let mut task_info = ::std::mem::zeroed::<ffi::proc_taskinfo>();
                    if ffi::proc_pidinfo(pid,
                                         ffi::PROC_PIDTASKINFO,
                                         0,
                                         &mut task_info as *mut ffi::proc_taskinfo as *mut c_void,
                                         taskinfo_size) != taskinfo_size {
                        continue
                    }
                    let task_time = user_time + system_time
                        + task_info.pti_total_user + task_info.pti_total_system;
                    let time = ffi::mach_absolute_time();
                    compute_cpu_usage(p, time, task_time);

                    p.memory = task_info.pti_resident_size / 1024;
                    continue
                }

                let mut task_info = ::std::mem::zeroed::<ffi::proc_taskallinfo>();
                if ffi::proc_pidinfo(pid,
                                     ffi::PROC_PIDTASKALLINFO,
                                     0,
                                     &mut task_info as *mut ffi::proc_taskallinfo as *mut c_void,
                                     taskallinfo_size as i32) != taskallinfo_size as i32 {
                    continue
                }

                let mut p = Process::new(pid as i64, task_info.pbsd.pbi_start_tvsec);
                p.memory = task_info.ptinfo.pti_resident_size / 1024;

                let mut ptr = proc_args.as_mut_slice().as_mut_ptr();
                mib[0] = ffi::CTL_KERN;
                mib[1] = ffi::KERN_PROCARGS2;
                mib[2] = pid as c_int;
                size = argmax as size_t;
                /*
                * /---------------\ 0x00000000
                * | ::::::::::::: |
                * |---------------| <-- Beginning of data returned by sysctl() is here.
                * | argc          |
                * |---------------|
                * | exec_path     |
                * |---------------|
                * | 0             |
                * |---------------|
                * | arg[0]        |
                * |---------------|
                * | 0             |
                * |---------------|
                * | arg[n]        |
                * |---------------|
                * | 0             |
                * |---------------|
                * | env[0]        |
                * |---------------|
                * | 0             |
                * |---------------|
                * | env[n]        |
                * |---------------|
                * | ::::::::::::: |
                * |---------------| <-- Top of stack.
                * :               :
                * :               :
                * \---------------/ 0xffffffff
                */
                if ffi::sysctl(mib.as_mut_ptr(), 3, ptr as *mut c_void,
                               &mut size, ::std::ptr::null_mut(), 0) != -1 {
                    let mut n_args: c_int = 0;
                    ffi::memcpy((&mut n_args) as *mut c_int as *mut c_void, ptr as *const c_void, ::std::mem::size_of::<c_int>());
                    let mut cp = ptr.offset(::std::mem::size_of::<c_int>() as isize);
                    let mut start = cp;
                    if cp < ptr.offset(size as isize) {
                        while cp < ptr.offset(size as isize) && *cp != 0 {
                            cp = cp.offset(1);
                        }
                        p.exe = get_unchecked_str(cp, start);
                        if let Some(l) = p.exe.split("/").last() {
                            p.name = l.to_owned();
                        }
                        while cp < ptr.offset(size as isize) && *cp == 0 {
                            cp = cp.offset(1);
                        }
                        start = cp;
                        let mut c = 0;
                        while c < n_args && cp < ptr.offset(size as isize) {
                            if *cp == 0 {
                                c += 1;
                                if c < n_args {
                                    *cp = ' ' as u8;
                                }
                            }
                            cp = cp.offset(1);
                        }
                        p.cmd = get_unchecked_str(cp.offset(-1), start);
                        start = cp;
                        while cp < ptr.offset(size as isize) {
                            if *cp == 0 {
                                if cp == start {
                                    break;
                                }
                                p.environ.push(get_unchecked_str(cp, start));
                                start = cp.offset(1);
                            }
                            cp = cp.offset(1);
                        }
                    }
                } else {
                    // we don't have enough priviledges to get access to these info
                    continue;
                }
                self.process_list.insert(pid as usize, p);
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn refresh_process(&mut self) {
        match fs::read_dir(&Path::new("/proc")) {
            Ok(d) => {
                for entry in d {
                    if !entry.is_ok() {
                        continue;
                    }
                    let entry = entry.unwrap();
                    let entry = entry.path();

                    if entry.is_dir() {
                        _get_process_data(entry.as_path(), &mut self.process_list, self.page_size_kb);
                    }
                }
                self.clear_procs();
            }
            Err(_) => {}
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn clear_procs(&mut self) {
        if self.processors.len() > 0 {
            let (new, old) = get_raw_times(&self.processors[0]);
            let total_time = (new - old) as f32;
            let mut to_delete = Vec::new();
            let nb_processors = self.processors.len() as u64 - 1;

            for (pid, proc_) in self.process_list.iter_mut() {
                if has_been_updated(&proc_) == false {
                    to_delete.push(*pid);
                } else {
                    compute_cpu_usage(proc_, nb_processors, total_time);
                }
            }
            for pid in to_delete {
                self.process_list.remove(&pid);
            }
        }
    }

    pub fn refresh_all(&mut self) {
        self.refresh_system();
        self.refresh_process();
    }

    pub fn get_process_list<'a>(&'a self) -> &'a HashMap<usize, Process> {
        &self.process_list
    }

    /// Return the process corresponding to the given pid or None if no such process exists.
    pub fn get_process(&self, pid: i64) -> Option<&Process> {
        self.process_list.get(&(pid as usize))
    }

    /// Return a list of process starting with the given name.
    pub fn get_process_by_name(&self, name: &str) -> Vec<&Process> {
        let mut ret = vec!();
        for val in self.process_list.values() {
            if val.name.starts_with(name) {
                ret.push(val);
            }
        }
        ret
    }

    /// The first process in the array is the "main" process
    pub fn get_processor_list<'a>(&'a self) -> &'a [Processor] {
        &self.processors[..]
    }

    pub fn get_total_memory(&self) -> u64 {
        self.mem_total
    }

    pub fn get_free_memory(&self) -> u64 {
        self.mem_free
    }

    pub fn get_used_memory(&self) -> u64 {
        self.mem_total - self.mem_free
    }

    pub fn get_total_swap(&self) -> u64 {
        self.swap_total
    }

    pub fn get_free_swap(&self) -> u64 {
        self.swap_free
    }

    // need to be checked
    pub fn get_used_swap(&self) -> u64 {
        self.swap_total - self.swap_free
    }

    pub fn get_components_list<'a>(&'a self) -> &'a [Component] {
        &self.temperatures[..]
    }
}

fn get_all_data(file_path: &str) -> String {
    let mut file = File::open(file_path).unwrap();
    let mut data = String::new();

    file.read_to_string(&mut data).unwrap();
    data
}

fn update_time_and_memory(entry: &mut Process, parts: &[&str], page_size_kb: u64) {
    //entry.name = parts[1][1..].to_owned();
    //entry.name.pop();
    // we get the rss then we add the vsize
    entry.memory = u64::from_str(parts[23]).unwrap() * page_size_kb +
                   u64::from_str(parts[22]).unwrap() / 1024;
    set_time(entry,
             u64::from_str(parts[13]).unwrap(),
             u64::from_str(parts[14]).unwrap());
}

fn _get_process_data(path: &Path, proc_list: &mut HashMap<usize, Process>, page_size_kb: u64) {
    if !path.exists() || !path.is_dir() {
        return;
    }
    let paths : Vec<&str> = path.as_os_str().to_str().unwrap().split("/").collect();
    let last = paths[paths.len() - 1];
    match i64::from_str(last) {
        Ok(nb) => {
            let mut tmp = PathBuf::from(path);

            tmp.push("stat");
            let data = get_all_data(tmp.to_str().unwrap());
            let (parts, _): (Vec<&str>, Vec<&str>) = data.split(' ').partition(|s| s.len() > 0);
            if let Some(ref mut entry) = proc_list.get_mut(&(nb as usize)) {
                update_time_and_memory(entry, &parts, page_size_kb);
                return;
            }
            let mut p = Process::new(nb,
                                     u64::from_str(parts[21]).unwrap() /
                                     unsafe { sysconf(_SC_CLK_TCK) } as u64);

            tmp = PathBuf::from(path);
            tmp.push("cmdline");
            p.cmd = if let Some(t) = copy_from_file(&tmp).get(0) {
                t.clone()
            } else {
                String::new()
            };
            let x = p.cmd.split(":").collect::<Vec<&str>>()[0]
                         .split(" ").collect::<Vec<&str>>()[0].to_owned();
            p.name = if x.contains("/") {
                x.split("/").last().unwrap().to_owned()
            } else {
                x
            };
            tmp = PathBuf::from(path);
            tmp.push("environ");
            p.environ = copy_from_file(&tmp);
            tmp = PathBuf::from(path);
            tmp.push("exe");

            let s = read_link(tmp.to_str().unwrap());

            if s.is_ok() {
                p.exe = s.unwrap().to_str().unwrap().to_owned();
            }
            tmp = PathBuf::from(path);
            tmp.push("cwd");
            p.cwd = realpath(Path::new(tmp.to_str().unwrap())).to_str().unwrap().to_owned();
            tmp = PathBuf::from(path);
            tmp.push("root");
            p.root = realpath(Path::new(tmp.to_str().unwrap())).to_str().unwrap().to_owned();

            update_time_and_memory(&mut p, &parts, page_size_kb);
            proc_list.insert(nb as usize, p);
        }
        _ => {}
    }
}

#[allow(unused_must_use)] 
fn copy_from_file(entry: &Path) -> Vec<String> {
    match File::open(entry.to_str().unwrap()) {
        Ok(mut f) => {
            let mut d = String::new();

            f.read_to_string(&mut d);
            let v : Vec<&str> = d.split('\0').collect();
            let mut ret : Vec<String> = Vec::new();

            for tmp in v.iter() {
            if tmp.len() < 1 {
                    continue;
                }
                ret.push((*tmp).to_owned());
            }
            ret
        },
        Err(_) => Vec::new()
    }
}

fn realpath(original: &Path) -> PathBuf {
    let ori = Path::new(original.to_str().unwrap());

    // Right now lstat on windows doesn't work quite well
    if cfg!(windows) {
        return PathBuf::from(ori);
    }
    let result = PathBuf::from(original);
    let mut buf: stat = unsafe { ::std::mem::uninitialized() };
    let res = unsafe { lstat(result.to_str().unwrap().as_ptr() as *const c_char, &mut buf as *mut stat) };
    if res < 0 || (buf.st_mode & S_IFMT) != S_IFLNK {
        PathBuf::new()
    } else {
        match fs::read_link(&result) {
            Ok(f) => f,
        Err(_) => PathBuf::new(),
        }
    }
}
