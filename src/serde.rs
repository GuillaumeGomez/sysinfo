// Take a look at the license at the top of the repository in the LICENSE file.

use crate::common::PidExt;
use crate::{
    ComponentExt, CpuExt, DiskExt, DiskKind, DiskUsage, MacAddr, NetworkExt, NetworksExt,
    ProcessExt, ProcessStatus, Signal, SystemExt, UserExt,
};
use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::ops::Deref;

impl Serialize for crate::Disk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Disk", 7)?;

        state.serialize_field("DiskKind", &self.kind())?;
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

impl Serialize for crate::Process {
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
            state.serialize_field("user_id", &uid.to_string())?;
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

impl Serialize for crate::Cpu {
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

impl serde::Serialize for crate::System {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("System", 27)?;

        state.serialize_field("IS_SUPPORTED", &<Self as SystemExt>::IS_SUPPORTED)?;
        state.serialize_field("SUPPORTED_SIGNALS", <Self as SystemExt>::SUPPORTED_SIGNALS)?;
        state.serialize_field(
            "MINIMUM_CPU_UPDATE_INTERVAL",
            &<Self as SystemExt>::MINIMUM_CPU_UPDATE_INTERVAL,
        )?;

        state.serialize_field("global_cpu_info", &self.global_cpu_info())?;
        state.serialize_field("cpus", &self.cpus())?;

        state.serialize_field("physical_core_count", &self.physical_core_count())?;
        state.serialize_field("total_memory", &self.total_memory())?;
        state.serialize_field("free_memory", &self.free_memory())?;
        state.serialize_field("available_memory", &self.available_memory())?;
        state.serialize_field("used_memory", &self.used_memory())?;
        state.serialize_field("total_swap", &self.total_swap())?;
        state.serialize_field("free_swap", &self.free_swap())?;
        state.serialize_field("used_swap", &self.used_swap())?;

        state.serialize_field("components", &self.components())?;
        state.serialize_field("users", &self.users())?;
        state.serialize_field("disks", &self.disks())?;
        state.serialize_field("networks", &self.networks())?;

        state.serialize_field("uptime", &self.uptime())?;
        state.serialize_field("boot_time", &self.boot_time())?;
        state.serialize_field("load_average", &self.load_average())?;
        state.serialize_field("name", &self.name())?;
        state.serialize_field("kernel_version", &self.kernel_version())?;
        state.serialize_field("os_version", &self.os_version())?;
        state.serialize_field("long_os_version", &self.long_os_version())?;
        state.serialize_field("distribution_id", &self.distribution_id())?;
        state.serialize_field("host_name", &self.host_name())?;

        state.end()
    }
}

impl Serialize for crate::Networks {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.iter())
    }
}

impl Serialize for Signal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (index, variant) = match *self {
            Signal::Hangup => (0, "Hangup"),
            Signal::Interrupt => (1, "Interrupt"),
            Signal::Quit => (2, "Quit"),
            Signal::Illegal => (3, "Illegal"),
            Signal::Trap => (4, "Trap"),
            Signal::Abort => (5, "Abort"),
            Signal::IOT => (6, "IOT"),
            Signal::Bus => (7, "Bus"),
            Signal::FloatingPointException => (8, "FloatingPointException"),
            Signal::Kill => (9, "Kill"),
            Signal::User1 => (10, "User1"),
            Signal::Segv => (11, "Segv"),
            Signal::User2 => (12, "User2"),
            Signal::Pipe => (13, "Pipe"),
            Signal::Alarm => (14, "Alarm"),
            Signal::Term => (15, "Term"),
            Signal::Child => (16, "Child"),
            Signal::Continue => (17, "Continue"),
            Signal::Stop => (18, "Stop"),
            Signal::TSTP => (19, "TSTP"),
            Signal::TTIN => (20, "TTIN"),
            Signal::TTOU => (21, "TTOU"),
            Signal::Urgent => (22, "Urgent"),
            Signal::XCPU => (23, "XCPU"),
            Signal::XFSZ => (24, "XFSZ"),
            Signal::VirtualAlarm => (25, "VirtualAlarm"),
            Signal::Profiling => (26, "Profiling"),
            Signal::Winch => (27, "Winch"),
            Signal::IO => (28, "IO"),
            Signal::Poll => (29, "Poll"),
            Signal::Power => (30, "Power"),
            Signal::Sys => (31, "Sys"),
        };

        serializer.serialize_unit_variant("Signal", index, variant)
    }
}

impl Serialize for crate::LoadAvg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("LoadAvg", 1)?;

        state.serialize_field("one", &self.one)?;
        state.serialize_field("five", &self.five)?;
        state.serialize_field("fifteen", &self.fifteen)?;
        state.end()
    }
}

// impl Serialize for crate::NetworkData {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let mut state = serializer.serialize_struct("NetworkData", 1)?;
//     }
// }

impl Serialize for crate::NetworkData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("NetworkData", 1)?;

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

impl Serialize for crate::Component {
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

impl Serialize for crate::User {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("User", 1)?;

        state.serialize_field("id", &self.id().to_string())?;

        let gid = *self.group_id().deref();
        state.serialize_field("group_id", &gid)?;

        state.serialize_field("name", &self.name())?;
        state.serialize_field("groups", &self.groups())?;

        state.end()
    }
}

impl Serialize for DiskKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (index, variant, maybe_value) = match *self {
            DiskKind::HDD => (0, "HDD", None),
            DiskKind::SSD => (1, "SSD", None),
            DiskKind::Unknown(ref s) => (2, "Unknown", Some(s)),
        };

        if let Some(ref value) = maybe_value {
            serializer.serialize_newtype_variant("DiskKind", index, variant, value)
        } else {
            serializer.serialize_unit_variant("DiskKind", index, variant)
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
            ProcessStatus::UninterruptibleDiskSleep => (11, "UninterruptibleDiskSleep", None),
            ProcessStatus::Unknown(n) => (12, "Unknown", Some(n)),
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
