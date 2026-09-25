// Take a look at the license at the top of the repository in the LICENSE file.

use std::ffi::OsString;
use std::fs::{File, read_dir, read_link};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::FileExt;
use std::path::PathBuf;

use crate::{DiskUsage, Gid, Pid, ProcessRefreshKind, ProcessStatus, Uid};

const PSINFO_MIN_SIZE: usize = 315;
const MAX_VECTOR_ITEMS: usize = 65_536;
const MAX_STRING_SIZE: usize = 1024 * 1024;

pub(crate) struct ProcessInner {
    pub(crate) name: OsString,
    pub(crate) cmd: Vec<OsString>,
    pub(crate) exe: Option<PathBuf>,
    pub(crate) pid: Pid,
    pub(crate) parent: Option<Pid>,
    pub(crate) environ: Vec<OsString>,
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) root: Option<PathBuf>,
    pub(crate) memory: u64,
    pub(crate) virtual_memory: u64,
    pub(crate) updated: bool,
    pub(crate) cpu_usage: f32,
    pub(crate) start_time: u64,
    pub(crate) run_time: u64,
    pub(crate) status: ProcessStatus,
    pub(crate) user_id: Uid,
    pub(crate) effective_user_id: Uid,
    pub(crate) group_id: Gid,
    pub(crate) effective_group_id: Gid,
    pub(crate) read_bytes: u64,
    pub(crate) old_read_bytes: u64,
    pub(crate) written_bytes: u64,
    pub(crate) old_written_bytes: u64,
    pub(crate) accumulated_cpu_time: u64,
    pub(crate) exists: bool,
}

impl ProcessInner {
    pub(crate) fn disk_usage(&self) -> DiskUsage {
        DiskUsage {
            written_bytes: self.written_bytes.saturating_sub(self.old_written_bytes),
            total_written_bytes: self.written_bytes,
            read_bytes: self.read_bytes.saturating_sub(self.old_read_bytes),
            total_read_bytes: self.read_bytes,
        }
    }

    pub(crate) fn open_files(&self) -> Option<usize> {
        read_dir(format!("/proc/{}/fd", self.pid))
            .ok()
            .map(|entries| entries.count())
    }

    pub(crate) fn open_files_limit(&self) -> Option<usize> {
        crate::System::open_files_limit().ok()
    }
}

pub(crate) struct ProcessInfo {
    pub(crate) pid: Pid,
    parent: Option<Pid>,
    user_id: Uid,
    effective_user_id: Uid,
    group_id: Gid,
    effective_group_id: Gid,
    virtual_memory: u64,
    memory: u64,
    cpu_usage: f32,
    start_time: u64,
    accumulated_cpu_time: u64,
    name: OsString,
    fallback_cmd: Vec<OsString>,
    argc: usize,
    argv: u64,
    envp: u64,
    pointer_size: usize,
    status: ProcessStatus,
}

impl ProcessInfo {
    pub(crate) fn read(pid: Pid) -> Option<Self> {
        let data = std::fs::read(format!("/proc/{pid}/psinfo")).ok()?;
        if data.len() < PSINFO_MIN_SIZE || read_i32(&data, 8)? != pid.0 {
            return None;
        }

        let model = *data.get(256)?;
        let pointer_size = if model == 1 { 4 } else { 8 };
        let name = c_field(&data, 136, 16);
        let psargs = c_field(&data, 152, 80);
        let fallback_cmd = if psargs.is_empty() {
            Vec::new()
        } else {
            psargs
                .split(|byte| byte.is_ascii_whitespace())
                .filter(|part| !part.is_empty())
                .map(|part| OsString::from_vec(part.to_vec()))
                .collect()
        };
        let status = match *data.get(314)? as char {
            'O' | 'R' => ProcessStatus::Run,
            'S' => ProcessStatus::Sleep,
            'T' => ProcessStatus::Stop,
            'Z' => ProcessStatus::Zombie,
            'I' => ProcessStatus::Idle,
            value => ProcessStatus::Unknown(value as u32),
        };

        Some(Self {
            pid,
            parent: match read_i32(&data, 12)? {
                0 => None,
                parent => Some(Pid(parent)),
            },
            user_id: Uid(read_u32(&data, 24)?),
            effective_user_id: Uid(read_u32(&data, 28)?),
            group_id: Gid(read_u32(&data, 32)?),
            effective_group_id: Gid(read_u32(&data, 36)?),
            virtual_memory: read_u64(&data, 48)?.saturating_mul(1024),
            memory: read_u64(&data, 56)?.saturating_mul(1024),
            cpu_usage: read_u16(&data, 80)? as f32 * 100. / 32768.,
            start_time: read_i64(&data, 88)?.max(0) as u64,
            accumulated_cpu_time: timespec_millis(&data, 104)?,
            name: OsString::from_vec(name),
            fallback_cmd,
            argc: read_i32(&data, 236)?.max(0) as usize,
            argv: read_pointer(&data, 240, pointer_size)?,
            envp: read_pointer(&data, 248, pointer_size)?,
            pointer_size,
            status,
        })
    }

    pub(crate) fn start_time(&self) -> u64 {
        self.start_time
    }

    fn vector(&self, address: u64, count: Option<usize>) -> Vec<OsString> {
        if address == 0 {
            return Vec::new();
        }
        let Ok(address_space) = File::open(format!("/proc/{}/as", self.pid)) else {
            return Vec::new();
        };
        let limit = count.unwrap_or(MAX_VECTOR_ITEMS).min(MAX_VECTOR_ITEMS);
        let mut output = Vec::with_capacity(count.unwrap_or(0).min(256));
        for index in 0..limit {
            let offset = address.saturating_add((index * self.pointer_size) as u64);
            let Some(pointer) = read_pointer_at(&address_space, offset, self.pointer_size) else {
                break;
            };
            if pointer == 0 {
                break;
            }
            let Some(value) = read_c_string_at(&address_space, pointer) else {
                break;
            };
            output.push(OsString::from_vec(value));
        }
        output
    }
}

pub(crate) fn update_process(
    info: &ProcessInfo,
    now: u64,
    refresh_kind: ProcessRefreshKind,
    process: &mut ProcessInner,
) {
    process.parent = info.parent;
    process.run_time = now.saturating_sub(info.start_time);
    process.exists = true;

    if refresh_kind.memory() {
        process.virtual_memory = info.virtual_memory;
        process.memory = info.memory;
    }
    if refresh_kind.cpu() {
        process.cpu_usage = info.cpu_usage;
        process.accumulated_cpu_time = info.accumulated_cpu_time;
    }
    if refresh_kind.disk_usage() {
        // illumos procfs does not expose per-process byte counters. Keep the
        // previous totals so the public deltas remain zero rather than bogus.
        process.old_read_bytes = process.read_bytes;
        process.old_written_bytes = process.written_bytes;
    }
    if refresh_kind.exe().needs_update(|| process.exe.is_none()) {
        process.exe = read_link(format!("/proc/{}/path/a.out", info.pid)).ok();
    }
    if refresh_kind.cwd().needs_update(|| process.cwd.is_none()) {
        process.cwd = read_link(format!("/proc/{}/path/cwd", info.pid)).ok();
    }
    if refresh_kind.root().needs_update(|| process.root.is_none()) {
        process.root = read_link(format!("/proc/{}/path/root", info.pid)).ok();
    }
    if refresh_kind.cmd().needs_update(|| process.cmd.is_empty()) {
        process.cmd = info.vector(info.argv, Some(info.argc));
        if process.cmd.is_empty() {
            process.cmd.clone_from(&info.fallback_cmd);
        }
    }
    if refresh_kind
        .environ()
        .needs_update(|| process.environ.is_empty())
    {
        process.environ = info.vector(info.envp, None);
    }
    if process.name.is_empty() {
        process.name.clone_from(&info.name);
        if process.name.is_empty()
            && let Some(command) = process.cmd.first()
        {
            process.name = PathBuf::from(command)
                .file_name()
                .map_or_else(OsString::new, OsString::from);
        }
    }
    process.status = info.status;
    process.updated = true;
}

pub(crate) fn new_process(
    info: &ProcessInfo,
    now: u64,
    refresh_kind: ProcessRefreshKind,
) -> ProcessInner {
    let mut process = ProcessInner {
        name: OsString::new(),
        cmd: Vec::new(),
        exe: None,
        pid: info.pid,
        parent: info.parent,
        environ: Vec::new(),
        cwd: None,
        root: None,
        memory: 0,
        virtual_memory: 0,
        updated: true,
        cpu_usage: 0.,
        start_time: info.start_time,
        run_time: 0,
        status: info.status,
        user_id: info.user_id.clone(),
        effective_user_id: info.effective_user_id.clone(),
        group_id: info.group_id,
        effective_group_id: info.effective_group_id,
        read_bytes: 0,
        old_read_bytes: 0,
        written_bytes: 0,
        old_written_bytes: 0,
        accumulated_cpu_time: 0,
        exists: true,
    };
    update_process(info, now, refresh_kind, &mut process);
    process
}

fn c_field(data: &[u8], offset: usize, length: usize) -> Vec<u8> {
    data.get(offset..offset.saturating_add(length))
        .unwrap_or_default()
        .split(|byte| *byte == 0)
        .next()
        .unwrap_or_default()
        .to_vec()
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_ne_bytes(
        data.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_ne_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_i32(data: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_ne_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_u64(data: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_ne_bytes(
        data.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

fn read_i64(data: &[u8], offset: usize) -> Option<i64> {
    Some(i64::from_ne_bytes(
        data.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

fn read_pointer(data: &[u8], offset: usize, size: usize) -> Option<u64> {
    match size {
        4 => read_u32(data, offset).map(u64::from),
        8 => read_u64(data, offset),
        _ => None,
    }
}

fn timespec_millis(data: &[u8], offset: usize) -> Option<u64> {
    let seconds = read_i64(data, offset)?.max(0) as u64;
    let nanoseconds = read_i64(data, offset + 8)?.max(0) as u64;
    Some(
        seconds
            .saturating_mul(1000)
            .saturating_add(nanoseconds / 1_000_000),
    )
}

fn read_pointer_at(file: &File, offset: u64, size: usize) -> Option<u64> {
    let mut bytes = [0u8; 8];
    (file.read_at(&mut bytes[..size], offset).ok()? == size).then(|| match size {
        4 => u32::from_ne_bytes(bytes[..4].try_into().unwrap()) as u64,
        8 => u64::from_ne_bytes(bytes),
        _ => 0,
    })
}

fn read_c_string_at(file: &File, address: u64) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    let mut offset = address;
    let mut buffer = [0u8; 256];
    while output.len() < MAX_STRING_SIZE {
        let read = file.read_at(&mut buffer, offset).ok()?;
        if read == 0 {
            return None;
        }
        if let Some(end) = buffer[..read].iter().position(|byte| *byte == 0) {
            output.extend_from_slice(&buffer[..end]);
            return Some(output);
        }
        output.extend_from_slice(&buffer[..read]);
        offset = offset.saturating_add(read as u64);
    }
    None
}
