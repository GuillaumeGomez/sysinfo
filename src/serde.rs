// Take a look at the license at the top of the repository in the LICENSE file.

#[allow(unused_imports)]
use serde::{Serialize, Serializer, ser::SerializeStruct, ser::SerializeTupleVariant};

#[allow(dead_code)]
#[rustfmt::skip]
type SerializeField<'a, S> = dyn FnOnce(
    &mut <S as Serializer>::SerializeStruct
) -> Result<(), <S as Serializer>::Error> + 'a;

#[allow(dead_code)]
fn serialize_struct<'a, S: Serializer>(
    struct_name: &'static str,
    serializer: S,
    fields: Vec<Box<SerializeField<'a, S>>>,
) -> Result<S::Ok, S::Error> {
    let mut state = serializer.serialize_struct(struct_name, fields.len())?;

    for call in fields {
        call(&mut state)?;
    }

    state.end()
}

#[cfg(feature = "disk")]
impl Serialize for crate::Disk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("DiskKind", &self.kind())),
            Box::new(|s| s.serialize_field("file_system", &self.file_system().to_string_lossy())),
            Box::new(|s| s.serialize_field("mount_point", &self.mount_point())),
            Box::new(|s| s.serialize_field("total_space", &self.total_space())),
            Box::new(|s| s.serialize_field("available_space", &self.available_space())),
            Box::new(|s| s.serialize_field("is_removable", &self.is_removable())),
        ];
        if let Some(name) = self.name().to_str() {
            fields.push(Box::new(|s| s.serialize_field("name", name)));
        }
        serialize_struct::<S>("Disk", serializer, fields)
    }
}

#[cfg(feature = "disk")]
impl Serialize for crate::Disks {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.iter())
    }
}

#[cfg(feature = "disk")]
impl Serialize for crate::DiskKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (index, variant, maybe_value) = match *self {
            Self::HDD => (0, "HDD", None),
            Self::SSD => (1, "SSD", None),
            Self::Unknown(ref s) => (2, "Unknown", Some(s)),
        };

        if let Some(ref value) = maybe_value {
            serializer.serialize_newtype_variant("DiskKind", index, variant, value)
        } else {
            serializer.serialize_unit_variant("DiskKind", index, variant)
        }
    }
}

#[cfg(feature = "system")]
impl Serialize for crate::Pid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct("Pid", &self.to_string())
    }
}

#[cfg(feature = "system")]
impl Serialize for crate::Process {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("name", &self.name().to_string_lossy())),
            Box::new(|s| s.serialize_field("cmd", &self.cmd())),
            Box::new(|s| s.serialize_field("exe", &self.exe())),
            Box::new(|s| s.serialize_field("pid", &self.pid().as_u32())),
            Box::new(|s| s.serialize_field("environ", &self.environ())),
            Box::new(|s| s.serialize_field("cwd", &self.cwd())),
            Box::new(|s| s.serialize_field("root", &self.root())),
            Box::new(|s| s.serialize_field("memory", &self.memory())),
            Box::new(|s| s.serialize_field("virtual_memory", &self.virtual_memory())),
            Box::new(|s| s.serialize_field("parent", &self.parent())),
            Box::new(|s| s.serialize_field("status", &self.status())),
            Box::new(|s| s.serialize_field("start_time", &self.start_time())),
            Box::new(|s| s.serialize_field("run_time", &self.run_time())),
            Box::new(|s| s.serialize_field("cpu_usage", &self.cpu_usage())),
            Box::new(|s| s.serialize_field("accumulated_cpu_time", &self.accumulated_cpu_time())),
            Box::new(|s| s.serialize_field("disk_usage", &self.disk_usage())),
            Box::new(|s| s.serialize_field("user_id", &self.user_id())),
            Box::new(|s| s.serialize_field("group_id", &self.group_id())),
            Box::new(|s| s.serialize_field("session_id", &self.session_id())),
        ];
        serialize_struct::<S>("Process", serializer, fields)
    }
}

#[cfg(feature = "system")]
impl Serialize for crate::Cpu {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("usage", &self.usage())),
            Box::new(|s| s.serialize_field("name", &self.name())),
            Box::new(|s| s.serialize_field("vendor_id", &self.vendor_id())),
            Box::new(|s| s.serialize_field("brand", &self.brand())),
            Box::new(|s| s.serialize_field("frequency", &self.frequency())),
        ];
        serialize_struct::<S>("Cpu", serializer, fields)
    }
}

#[cfg(feature = "system")]
impl Serialize for crate::System {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("global_cpu_usage", &self.global_cpu_usage())),
            Box::new(|s| s.serialize_field("cpus", &self.cpus())),
            Box::new(|s| s.serialize_field("physical_core_count", &Self::physical_core_count())),
            Box::new(|s| s.serialize_field("total_memory", &self.total_memory())),
            Box::new(|s| s.serialize_field("free_memory", &self.free_memory())),
            Box::new(|s| s.serialize_field("available_memory", &self.available_memory())),
            Box::new(|s| s.serialize_field("used_memory", &self.used_memory())),
            Box::new(|s| s.serialize_field("total_swap", &self.total_swap())),
            Box::new(|s| s.serialize_field("free_swap", &self.free_swap())),
            Box::new(|s| s.serialize_field("used_swap", &self.used_swap())),
            Box::new(|s| s.serialize_field("uptime", &Self::uptime().ok())),
            Box::new(|s| s.serialize_field("boot_time", &Self::boot_time().ok())),
            Box::new(|s| s.serialize_field("load_average", &Self::load_average().ok())),
            Box::new(|s| s.serialize_field("name", &Self::name())),
            Box::new(|s| s.serialize_field("kernel_version", &Self::kernel_version())),
            Box::new(|s| s.serialize_field("os_version", &Self::os_version())),
            Box::new(|s| s.serialize_field("long_os_version", &Self::long_os_version())),
            Box::new(|s| s.serialize_field("distribution_id", &Self::distribution_id())),
            Box::new(|s| s.serialize_field("host_name", &Self::host_name())),
        ];
        serialize_struct::<S>("System", serializer, fields)
    }
}
#[cfg(feature = "system")]
impl Serialize for crate::Motherboard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("name", &self.name())),
            Box::new(|s| s.serialize_field("vendor_name", &self.vendor_name())),
            Box::new(|s| s.serialize_field("version", &self.version())),
            Box::new(|s| s.serialize_field("serial_number", &self.serial_number())),
            Box::new(|s| s.serialize_field("asset_tag", &self.asset_tag())),
        ];
        serialize_struct::<S>("Motherboard", serializer, fields)
    }
}

#[cfg(feature = "system")]
impl Serialize for crate::Product {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("name", &Self::name())),
            Box::new(|s| s.serialize_field("family", &Self::family())),
            Box::new(|s| s.serialize_field("serial_number", &Self::serial_number())),
            Box::new(|s| s.serialize_field("stock_keeping_unit", &Self::stock_keeping_unit())),
            Box::new(|s| s.serialize_field("uuid", &Self::uuid())),
            Box::new(|s| s.serialize_field("version", &Self::version())),
            Box::new(|s| s.serialize_field("vendor_name", &Self::vendor_name())),
        ];
        serialize_struct::<S>("Product", serializer, fields)
    }
}

#[cfg(feature = "system")]
impl Serialize for crate::CGroupLimits {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("total_memory", &self.total_memory)),
            Box::new(|s| s.serialize_field("free_memory", &self.free_memory)),
            Box::new(|s| s.serialize_field("free_swap", &self.free_swap)),
        ];
        serialize_struct::<S>("CGroupLimits", serializer, fields)
    }
}

#[cfg(feature = "system")]
impl Serialize for crate::ThreadKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (index, variant) = match *self {
            Self::Kernel => (0, "Kernel"),
            Self::Userland => (1, "Userland"),
        };

        serializer.serialize_unit_variant("ThreadKind", index, variant)
    }
}

#[cfg(feature = "system")]
impl Serialize for crate::Signal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (index, variant) = match *self {
            Self::Hangup => (0, "Hangup"),
            Self::Interrupt => (1, "Interrupt"),
            Self::Quit => (2, "Quit"),
            Self::Illegal => (3, "Illegal"),
            Self::Trap => (4, "Trap"),
            Self::Abort => (5, "Abort"),
            Self::IOT => (6, "IOT"),
            Self::Bus => (7, "Bus"),
            Self::FloatingPointException => (8, "FloatingPointException"),
            Self::Kill => (9, "Kill"),
            Self::User1 => (10, "User1"),
            Self::Segv => (11, "Segv"),
            Self::User2 => (12, "User2"),
            Self::Pipe => (13, "Pipe"),
            Self::Alarm => (14, "Alarm"),
            Self::Term => (15, "Term"),
            Self::Child => (16, "Child"),
            Self::Continue => (17, "Continue"),
            Self::Stop => (18, "Stop"),
            Self::TSTP => (19, "TSTP"),
            Self::TTIN => (20, "TTIN"),
            Self::TTOU => (21, "TTOU"),
            Self::Urgent => (22, "Urgent"),
            Self::XCPU => (23, "XCPU"),
            Self::XFSZ => (24, "XFSZ"),
            Self::VirtualAlarm => (25, "VirtualAlarm"),
            Self::Profiling => (26, "Profiling"),
            Self::Winch => (27, "Winch"),
            Self::IO => (28, "IO"),
            Self::Poll => (29, "Poll"),
            Self::Power => (30, "Power"),
            Self::Sys => (31, "Sys"),
        };

        serializer.serialize_unit_variant("Signal", index, variant)
    }
}

#[cfg(feature = "system")]
impl Serialize for crate::LoadAvg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("one", &self.one)),
            Box::new(|s| s.serialize_field("five", &self.five)),
            Box::new(|s| s.serialize_field("fifteen", &self.fifteen)),
        ];
        serialize_struct::<S>("LoadAvg", serializer, fields)
    }
}

#[cfg(feature = "system")]
impl Serialize for crate::ProcessStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (index, variant, maybe_value) = match *self {
            Self::Idle => (0, "Idle", None),
            Self::Run => (1, "Run", None),
            Self::Sleep => (2, "Sleep", None),
            Self::Stop => (3, "Stop", None),
            Self::Zombie => (4, "Zombie", None),
            Self::Tracing => (5, "Tracing", None),
            Self::Dead => (6, "Dead", None),
            Self::Wakekill => (7, "Wakekill", None),
            Self::Waking => (8, "Waking", None),
            Self::Parked => (9, "Parked", None),
            Self::LockBlocked => (10, "LockBlocked", None),
            Self::UninterruptibleDiskSleep => (11, "UninterruptibleDiskSleep", None),
            Self::Suspended => (12, "Suspended", None),
            Self::Unknown(n) => (13, "Unknown", Some(n)),
        };

        if let Some(ref value) = maybe_value {
            serializer.serialize_newtype_variant("ProcessStatus", index, variant, value)
        } else {
            serializer.serialize_unit_variant("ProcessStatus", index, variant)
        }
    }
}

#[cfg(feature = "system")]
impl Serialize for crate::DiskUsage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("total_written_bytes", &self.total_written_bytes)),
            Box::new(|s| s.serialize_field("written_bytes", &self.written_bytes)),
            Box::new(|s| s.serialize_field("total_read_bytes", &self.total_read_bytes)),
            Box::new(|s| s.serialize_field("read_bytes", &self.read_bytes)),
        ];
        serialize_struct::<S>("DiskUsage", serializer, fields)
    }
}

#[cfg(feature = "component")]
impl Serialize for crate::Components {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.iter())
    }
}

#[cfg(feature = "component")]
impl Serialize for crate::Component {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("temperature", &self.temperature())),
            Box::new(|s| s.serialize_field("max", &self.max())),
            Box::new(|s| s.serialize_field("critical", &self.critical())),
            Box::new(|s| s.serialize_field("label", &self.label())),
        ];
        serialize_struct::<S>("Component", serializer, fields)
    }
}

#[cfg(feature = "network")]
impl Serialize for crate::Networks {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.iter())
    }
}

#[cfg(feature = "network")]
impl Serialize for crate::NetworkData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("received", &self.received())),
            Box::new(|s| s.serialize_field("total_received", &self.total_received())),
            Box::new(|s| s.serialize_field("transmitted", &self.transmitted())),
            Box::new(|s| s.serialize_field("total_transmitted", &self.total_transmitted())),
            Box::new(|s| s.serialize_field("packets_received", &self.packets_received())),
            Box::new(|s| {
                s.serialize_field("total_packets_received", &self.total_packets_received())
            }),
            Box::new(|s| s.serialize_field("packets_transmitted", &self.packets_transmitted())),
            Box::new(|s| {
                s.serialize_field(
                    "total_packets_transmitted",
                    &self.total_packets_transmitted(),
                )
            }),
            Box::new(|s| s.serialize_field("errors_on_received", &self.errors_on_received())),
            Box::new(|s| {
                s.serialize_field("total_errors_on_received", &self.total_errors_on_received())
            }),
            Box::new(|s| s.serialize_field("errors_on_transmitted", &self.errors_on_transmitted())),
            Box::new(|s| {
                s.serialize_field(
                    "total_errors_on_transmitted",
                    &self.total_errors_on_transmitted(),
                )
            }),
            Box::new(|s| s.serialize_field("mac_address", &self.mac_address())),
            Box::new(|s| s.serialize_field("ip_networks", &self.ip_networks())),
            Box::new(|s| s.serialize_field("mtu", &self.mtu())),
        ];
        serialize_struct::<S>("NetworkData", serializer, fields)
    }
}

#[cfg(feature = "network")]
impl Serialize for crate::MacAddr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct("MacAddr", &self.to_string())
    }
}

#[cfg(feature = "network")]
impl Serialize for crate::IpNetwork {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("addr", &self.addr)),
            Box::new(|s| s.serialize_field("prefix", &self.prefix)),
        ];
        serialize_struct::<S>("IpNetwork", serializer, fields)
    }
}

#[cfg(feature = "user")]
impl Serialize for crate::Users {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.iter())
    }
}

#[cfg(feature = "user")]
impl Serialize for crate::User {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("id", &self.id())),
            Box::new(|s| s.serialize_field("group_id", &self.group_id())),
            Box::new(|s| s.serialize_field("name", &self.name())),
            Box::new(|s| s.serialize_field("groups", &self.groups())),
        ];
        serialize_struct::<S>("User", serializer, fields)
    }
}

#[cfg(feature = "user")]
impl Serialize for crate::Group {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("id", &self.id())),
            Box::new(|s| s.serialize_field("name", &self.name())),
        ];
        serialize_struct::<S>("Group", serializer, fields)
    }
}

#[cfg(any(feature = "user", feature = "system"))]
impl Serialize for crate::Gid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct("Gid", &self.to_string())
    }
}

#[cfg(any(feature = "user", feature = "system"))]
impl Serialize for crate::Uid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct("Uid", &self.to_string())
    }
}

#[cfg(feature = "gpu")]
impl Serialize for crate::Gpus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.iter())
    }
}

#[cfg(feature = "gpu")]
impl Serialize for crate::Gpu {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields: Vec<Box<SerializeField<S>>> = vec![
            Box::new(|s| s.serialize_field("pci", &self.pci())),
            Box::new(|s| s.serialize_field("vendor", &self.vendor())),
            Box::new(|s| s.serialize_field("model", &self.model())),
            Box::new(|s| s.serialize_field("usage", &self.usage())),
            Box::new(|s| s.serialize_field("total_memory", &self.total_memory())),
            Box::new(|s| s.serialize_field("used_memory", &self.used_memory())),
        ];
        serialize_struct::<S>("Gpu", serializer, fields)
    }
}

impl Serialize for crate::Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Unsupported => serializer.serialize_unit_variant("Error", 0, "Unsupported"),
            Self::Io(io) => {
                let mut state = serializer.serialize_tuple_variant("Error", 1, "Io", 1)?;
                state.serialize_field(&io.to_string())?;
                state.end()
            }
            Self::Other(other) => {
                let mut state = serializer.serialize_tuple_variant("Error", 2, "Other", 1)?;
                state.serialize_field(&other.to_string())?;
                state.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_serde_process_name() {
        let Ok(mut s) = crate::System::new() else {
            return;
        };
        s.refresh_processes_specifics(
            crate::ProcessesToUpdate::All,
            false,
            crate::ProcessRefreshKind::nothing(),
        );

        if s.processes().is_empty() {
            panic!("no processes?");
        }

        for p in s.processes().values() {
            let values = match serde_json::to_value(p) {
                Ok(serde_json::Value::Object(values)) => values,
                other => panic!("expected object, found `{other:?}`"),
            };
            match values.get("name") {
                Some(serde_json::Value::String(_)) => {}
                value => panic!("expected a string, found `{value:?}`"),
            }
        }
    }

    #[test]
    #[cfg(feature = "network")]
    fn test_serde_mac_address() {
        let m = crate::MacAddr([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC]);

        let value = match serde_json::to_value(m) {
            Ok(serde_json::Value::String(value)) => value,
            other => panic!("expected string, found `{other:?}`"),
        };
        assert_eq!(value, "12:34:56:78:9a:bc");
    }

    #[test]
    #[cfg(feature = "disk")]
    fn test_serde_disk_file_system() {
        let mut disk = crate::Disk {
            inner: crate::DiskInner::default(),
        };
        disk.inner.file_system = "ZFS".into();

        let obj = match serde_json::to_value(disk) {
            Ok(serde_json::Value::Object(obj)) => obj,
            other => panic!("expected object, found `{other:?}`"),
        };
        assert_eq!(
            obj.get("file_system"),
            Some(&serde_json::Value::String("ZFS".to_string()))
        );
    }
}
