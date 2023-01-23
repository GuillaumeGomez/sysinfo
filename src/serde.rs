use crate::common::PidExt;
use crate::{ComponentExt, CpuExt, DiskExt, NetworkExt, ProcessExt, SystemExt, UserExt};
use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::ops::Deref;

impl Serialize for dyn DiskExt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Disk", 7)?;

        state.serialize_field("DiskType", &self.type_())?;
        if let Some(s) = self.name().to_str() {
            state.serialize_field("name", s)?;
        }
        state.serialize_field("file_system", &self.file_system())?;
        state.serialize_field("mount_point", &self.mount_point())?;
        state.serialize_field("total_space", &self.total_space())?;
        state.serialize_field("available_space", &self.available_space())?;
        state.serialize_field("is_removable", &self.is_removable())?;

        state.end()
    }
}

impl Serialize for dyn ProcessExt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Process", 18)?;

        state.serialize_field("name", &self.name())?;
        state.serialize_field("cmd", &self.cmd())?;
        state.serialize_field("exe", &self.exe())?;
        state.serialize_field("pid", &self.pid().as_u32())?;
        state.serialize_field("environ", &self.environ())?;
        state.serialize_field("cwd", &self.cwd())?;
        state.serialize_field("root", &self.root())?;
        state.serialize_field("memory", &self.memory())?;
        state.serialize_field("virtual_memory", &self.virtual_memory())?;
        if let Some(pid) = self.parent() {
            state.serialize_field("parent", &pid.as_u32())?;
        }
        state.serialize_field("status", &self.status())?;
        state.serialize_field("start_time", &self.start_time())?;
        state.serialize_field("run_time", &self.run_time())?;
        state.serialize_field("cpu_usage", &self.cpu_usage())?;
        state.serialize_field("disk_usage", &self.disk_usage())?;
        if let Some(uid) = self.user_id() {
            let uid = *uid.deref();
            state.serialize_field("user_id", &uid)?;
        }
        if let Some(gid) = self.group_id() {
            let gid = *gid.deref();
            state.serialize_field("group_id", &gid)?;
        }
        if let Some(pid) = self.session_id() {
            state.serialize_field("session_id", &pid.as_u32())?;
        }

        state.end()
    }
}

impl Serialize for dyn CpuExt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Cpu", 1)?;

        state.serialize_field("cpu_usage", &self.cpu_usage())?;
        state.serialize_field("name", &self.name())?;
        state.serialize_field("vendor_id", &self.vendor_id())?;
        state.serialize_field("brand", &self.brand())?;
        state.serialize_field("frequency", &self.frequency())?;

        state.end()
    }
}

/*
impl serde::Serialize for dyn SystemExt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("System", 27)?;

        // IS_SUPPORTED: bool
        // SUPPORTED_SIGNALS: &'static [Signal]
        // MINIMUM_CPU_UPDATE_INTERVAL: Duration;
        // &self.processes() -> &HashMap<Pid, Process>;
        // &self.global_cpu_info() -> &Cpu;
        // &self.cpus -> &[Cpu];

        state.serialize_field("physical_core_count", &self.physical_core_count())?; // Option<usize>
        state.serialize_field("total_memory", &self.total_memory())?; // u64
        state.serialize_field("free_memory", &self.free_memory())?; // u64
        state.serialize_field("available_memory", &self.available_memory())?; // u64
        state.serialize_field("used_memory", &self.used_memory())?; // u64
        state.serialize_field("total_swap", &self.total_swap())?; // u64
        state.serialize_field("free_swap", &self.free_swap())?; // u64
        state.serialize_field("used_swap", &self.used_swap())?; // u64

        // &self.components() -> &[Component]
        // &self.users() -> &[User]
        // &self.disks() -> &[Disk]
        // &self.networks() -> &Networks

        state.serialize_field("uptime", &self.uptime())?; // u64
        state.serialize_field("boot_time", &self.boot_time())?; // u64

        // &self.load_average() -> LoadAvg

        state.serialize_field("name", &self.name())?; // Option<String>
        state.serialize_field("kernel_version", &self.kernel_version())?; // Option<String>
        state.serialize_field("os_version", &self.os_version())?; // Option<String>
        state.serialize_field("long_os_version", &self.long_os_version())?; // Option<String>
        state.serialize_field("distribution_id", &self.distribution_id())?; // String
        state.serialize_field("host_name", &self.host_name())?; // Option<String>

        state.end()
    }
}
*/

impl Serialize for dyn NetworkExt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Network", 1)?;

        state.serialize_field("received", &self.received())?;
        state.serialize_field("total_received", &self.total_received())?;
        state.serialize_field("transmitted", &self.transmitted())?;
        state.serialize_field("total_transmitted", &self.total_transmitted())?;
        state.serialize_field("packets_received", &self.packets_received())?;
        state.serialize_field("total_packets_received", &self.total_packets_received())?;
        state.serialize_field("packets_transmitted", &self.packets_transmitted())?;
        state.serialize_field(
            "total_packets_transmitted",
            &self.total_packets_transmitted(),
        )?;
        state.serialize_field("errors_on_received", &self.errors_on_received())?;
        state.serialize_field("total_errors_on_received", &self.total_errors_on_received())?;
        state.serialize_field("errors_on_transmitted", &self.errors_on_transmitted())?;
        state.serialize_field(
            "total_errors_on_transmitted",
            &self.total_errors_on_transmitted(),
        )?;
        state.serialize_field("mac_address", &self.mac_address())?;

        state.end()
    }
}

impl Serialize for dyn ComponentExt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Component", 4)?;

        state.serialize_field("temperature", &self.temperature())?;
        state.serialize_field("max", &self.max())?;
        state.serialize_field("critical", &self.critical())?;
        state.serialize_field("label", &self.label())?;

        state.end()
    }
}

impl Serialize for dyn UserExt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("User", 1)?;

        let uid = *self.id().deref();
        state.serialize_field("id", &uid)?;

        let gid = *self.group_id().deref();
        state.serialize_field("group_id", &gid)?;

        state.serialize_field("name", &self.name())?;
        state.serialize_field("groups", &self.groups())?;

        state.end()
    }
}

use crate::{DiskType, DiskUsage, MacAddr, ProcessStatus};

impl Serialize for DiskType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (index, variant, maybe_value) = match *self {
            DiskType::HDD => (0, "HDD", None),
            DiskType::SSD => (1, "SSD", None),
            DiskType::Unknown(ref s) => (2, "Unknown", Some(s)),
        };

        if let Some(ref value) = maybe_value {
            serializer.serialize_newtype_variant("DiskType", index, variant, value)
        } else {
            serializer.serialize_unit_variant("DiskType", index, variant)
        }
    }
}

impl Serialize for ProcessStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (index, variant, maybe_value) = match *self {
            ProcessStatus::Idle => (0, "Idle", None),
            ProcessStatus::Run => (1, "Run", None),
            ProcessStatus::Sleep => (2, "Sleep", None),
            ProcessStatus::Stop => (3, "Stop", None),
            ProcessStatus::Zombie => (4, "Zombie", None),
            ProcessStatus::Tracing => (5, "Tracing", None),
            ProcessStatus::Dead => (6, "Dead", None),
            ProcessStatus::Wakekill => (7, "Wakekill", None),
            ProcessStatus::Waking => (8, "Waking", None),
            ProcessStatus::Parked => (9, "Parked", None),
            ProcessStatus::LockBlocked => (10, "LockBlocked", None),
            ProcessStatus::Unknown(n) => (11, "Unknown", Some(n)),
        };

        if let Some(ref value) = maybe_value {
            serializer.serialize_newtype_variant("ProcessStatus", index, variant, value)
        } else {
            serializer.serialize_unit_variant("ProcessStatus", index, variant)
        }
    }
}

impl Serialize for DiskUsage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("DiskUsage", 4)?;

        state.serialize_field("total_written_bytes", &self.total_written_bytes)?;
        state.serialize_field("written_bytes", &self.written_bytes)?;
        state.serialize_field("total_read_bytes", &self.total_read_bytes)?;
        state.serialize_field("read_bytes", &self.read_bytes)?;

        state.end()
    }
}

impl Serialize for MacAddr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct("MacAddr", &self.0)
    }
}
