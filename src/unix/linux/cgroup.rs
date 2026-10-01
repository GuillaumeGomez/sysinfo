// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::utils::get_all_utf8_data;

use std::cmp::min;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Clone, Copy)]
struct CGroupLimitsContext {
    mem_total: u64,
    swap_total: u64,
    swap_free: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct CGroupPath {
    v2: Option<PathBuf>,
    v1_memory: Option<PathBuf>,
}

impl CGroupPath {
    fn is_empty(&self) -> bool {
        self.v2.is_none() && self.v1_memory.is_none()
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CGroupBase {
    base: PathBuf,
    root: PathBuf,
}

impl CGroupBase {
    fn new(base: PathBuf, root: PathBuf) -> Self {
        Self { base, root }
    }

    fn root(root: &Path) -> Self {
        Self::new(root.to_path_buf(), root.to_path_buf())
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CGroupMount {
    root: PathBuf,
    mount_point: PathBuf,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct CGroupMounts {
    v2: Vec<CGroupMount>,
    v1_memory: Vec<CGroupMount>,
}

pub(crate) fn limits_for_system() -> Option<crate::CGroupLimits> {
    let v2_root = Path::new("/sys/fs/cgroup");
    let v1_root = Path::new("/sys/fs/cgroup/memory");
    let v2_bases = [CGroupBase::root(v2_root)];
    let v1_bases = [CGroupBase::root(v1_root)];
    let v1_bases = if v1_root.join("memory.limit_in_bytes").exists() {
        &v1_bases[..]
    } else {
        &[]
    };

    limits_for_base(&v2_bases, v1_bases)
}

pub(crate) fn limits_for_process(proc_path: &Path) -> Option<crate::CGroupLimits> {
    let cgroup_path = get_cgroup_path(&proc_path.join("cgroup"))?;
    let cgroup_mounts = get_cgroup_mounts(&proc_path.join("mountinfo"))?;

    limits_for_process_with_context(&cgroup_path, &cgroup_mounts, read_cgroup_limits_context()?)
}

fn limits_for_process_with_context(
    cgroup_path: &CGroupPath,
    cgroup_mounts: &CGroupMounts,
    context: CGroupLimitsContext,
) -> Option<crate::CGroupLimits> {
    // Select the memory controller and mount before reading limits. A failed read must not
    // fall back to a different mount or controller that could report a larger limit.
    if let Some(path) = &cgroup_path.v1_memory {
        let base = cgroup_base_for_path(path, &cgroup_mounts.v1_memory)?;
        return v1_limits(&base.base, &base.root, context);
    }
    let base = cgroup_base_for_path(cgroup_path.v2.as_ref()?, &cgroup_mounts.v2)?;
    v2_limits(&base.base, &base.root, context)
}

/// Evaluate the conventional cgroup roots for system-wide limits.
fn limits_for_base(
    v2_bases: &[CGroupBase],
    v1_bases: &[CGroupBase],
) -> Option<crate::CGroupLimits> {
    let context = read_cgroup_limits_context()?;
    limits_for_base_with_context(v2_bases, v1_bases, context)
}

fn limits_for_base_with_context(
    v2_bases: &[CGroupBase],
    v1_bases: &[CGroupBase],
    context: CGroupLimitsContext,
) -> Option<crate::CGroupLimits> {
    v1_bases
        .iter()
        .find_map(|v1_base| v1_limits(&v1_base.base, &v1_base.root, context))
        .or_else(|| {
            v2_bases
                .iter()
                .find_map(|v2_base| v2_limits(&v2_base.base, &v2_base.root, context))
        })
}

fn read_cgroup_limits_context() -> Option<CGroupLimitsContext> {
    let mut mem_total = None;
    let mut swap_total = 0;
    let mut swap_free = 0;

    read_table("/proc/meminfo", ':', |key, value_kib| {
        let value = value_kib.saturating_mul(1_024);
        match key {
            "MemTotal" => mem_total = Some(value),
            "SwapTotal" => swap_total = value,
            "SwapFree" => swap_free = value,
            _ => (),
        }
    });

    Some(CGroupLimitsContext {
        mem_total: mem_total?,
        swap_total,
        swap_free,
    })
}

fn v2_limits(
    base: &Path,
    root: &Path,
    context: CGroupLimitsContext,
) -> Option<crate::CGroupLimits> {
    let mem_max = read_v2_memory_max(&base.join("memory.max"), root)?;
    let (total_memory, free_memory) = memory_limits(
        base,
        root,
        "memory.max",
        "memory.current",
        context.mem_total,
        mem_max,
        |path| read_v2_memory_max(path, root),
    )?;
    let mem_rss = read_table_key(&base.join("memory.stat"), "anon", ' ')?;

    let mut limits = crate::CGroupLimits {
        total_memory,
        free_memory,
        free_swap: context.swap_free,
        rss: mem_rss,
    };

    if let Some(swap_cur) = read_u64(&base.join("memory.swap.current")) {
        limits.free_swap = context.swap_total.saturating_sub(swap_cur);
    }

    Some(limits)
}

fn v1_limits(
    base: &Path,
    root: &Path,
    context: CGroupLimitsContext,
) -> Option<crate::CGroupLimits> {
    let mem_max = read_u64(&base.join("memory.limit_in_bytes"))?;
    let (total_memory, free_memory) = memory_limits(
        base,
        root,
        "memory.limit_in_bytes",
        "memory.usage_in_bytes",
        context.mem_total,
        mem_max,
        read_u64,
    )?;
    let mem_rss = read_table_key(&base.join("memory.stat"), "total_rss", ' ')?;

    Some(crate::CGroupLimits {
        total_memory,
        free_memory,
        free_swap: context.swap_free,
        rss: mem_rss,
    })
}

fn memory_limits<F>(
    base: &Path,
    root: &Path,
    limit_file: &str,
    usage_file: &str,
    mem_total: u64,
    base_limit: u64,
    read_limit: F,
) -> Option<(u64, u64)>
where
    F: Fn(&Path) -> Option<u64>,
{
    let mem_cur = read_u64(&base.join(usage_file))?;
    let mut total_memory = None;
    let mut free_memory = None;

    for (pos, path) in base.ancestors().enumerate() {
        let is_base = pos == 0;
        let mem_max = if is_base {
            base_limit
        } else {
            read_limit(&path.join(limit_file))?
        };
        if mem_max <= mem_total {
            let mem_cur = if is_base {
                mem_cur
            } else {
                read_u64(&path.join(usage_file))?
            };
            total_memory = Some(match total_memory {
                Some(total_memory) => min(total_memory, mem_max),
                None => mem_max,
            });
            free_memory = Some(match free_memory {
                Some(free_memory) => min(free_memory, mem_max.saturating_sub(mem_cur)),
                None => mem_max.saturating_sub(mem_cur),
            });
        }
        if path == root {
            return Some((total_memory?, free_memory?));
        }
    }

    None
}

fn read_v2_memory_max(filename: &Path, root: &Path) -> Option<u64> {
    let content = match get_all_utf8_data(filename, 16_635) {
        Ok(content) => content,
        // The actual v2 hierarchy root has neither memory.max nor cgroup.type. A mount
        // rooted at a non-root cgroup still has cgroup.type, so it must not use this exception.
        Err(err)
            if err.kind() == ErrorKind::NotFound
                && filename.parent() == Some(root)
                && get_all_utf8_data(root.join("cgroup.controllers"), 4096).is_ok()
                && matches!(
                    std::fs::metadata(root.join("cgroup.type")),
                    Err(err) if err.kind() == ErrorKind::NotFound
                ) =>
        {
            return Some(u64::MAX);
        }
        Err(_) => {
            sysinfo_debug!("Failed to read u64 in filename {filename:?}");
            return None;
        }
    };
    let content = content.trim();
    if content == "max" {
        return Some(u64::MAX);
    }

    match u64::from_str(content).ok() {
        Some(value) => Some(value),
        None => {
            sysinfo_debug!("Failed to read u64 in filename {filename:?}");
            None
        }
    }
}

fn read_u64(filename: &Path) -> Option<u64> {
    let result = get_all_utf8_data(filename, 16_635)
        .ok()
        .and_then(|d| u64::from_str(d.trim()).ok());

    if result.is_none() {
        sysinfo_debug!("Failed to read u64 in filename {filename:?}");
    }

    result
}

fn read_table<F>(filename: &str, colsep: char, mut f: F)
where
    F: FnMut(&str, u64),
{
    if let Ok(content) = get_all_utf8_data(filename, 16_635) {
        content
            .split('\n')
            .flat_map(|line| {
                let mut split = line.split(colsep);
                let key = split.next()?;
                let value = split.next()?;
                let value0 = value.trim_start().split(' ').next()?;
                let value0_u64 = u64::from_str(value0).ok()?;
                Some((key, value0_u64))
            })
            .for_each(|(k, v)| f(k, v));
    }
}

fn read_table_key(filename: &Path, target_key: &str, colsep: char) -> Option<u64> {
    if let Ok(content) = get_all_utf8_data(filename, 16_635) {
        return content.split('\n').find_map(|line| {
            let mut split = line.split(colsep);
            let key = split.next()?;
            if key != target_key {
                return None;
            }

            let value = split.next()?;
            let value0 = value.trim_start().split(' ').next()?;
            u64::from_str(value0).ok()
        });
    }

    None
}

fn get_cgroup_path(path: &Path) -> Option<CGroupPath> {
    let content = get_all_utf8_data(path, 4096).ok()?;
    let cgroup_path = parse_cgroup_path(&content);
    if cgroup_path.is_empty() {
        return None;
    }
    Some(cgroup_path)
}

fn get_cgroup_mounts(path: &Path) -> Option<CGroupMounts> {
    let content = get_all_utf8_data(path, 16_385).ok()?;
    Some(parse_cgroup_mounts(&content))
}

fn cgroup_base_for_path(cgroup_path: &Path, mounts: &[CGroupMount]) -> Option<CGroupBase> {
    mounts
        .iter()
        .find_map(|mount| resolve_cgroup_base(cgroup_path, mount))
}

fn resolve_cgroup_base(cgroup_path: &Path, mount: &CGroupMount) -> Option<CGroupBase> {
    let relative_path = if mount.root.as_os_str().is_empty() {
        cgroup_path
    } else {
        cgroup_path.strip_prefix(&mount.root).ok()?
    };

    Some(CGroupBase::new(
        join_cgroup_path(&mount.mount_point, relative_path),
        mount.mount_point.clone(),
    ))
}

fn join_cgroup_path(root: &Path, path: &Path) -> PathBuf {
    if path.as_os_str().is_empty() {
        return root.to_path_buf();
    }

    root.join(path)
}

fn parse_cgroup_mounts(content: &str) -> CGroupMounts {
    let mut mounts = CGroupMounts::default();

    for line in content.lines() {
        let mut fields = line.split(' ');
        // Skip the mount ID, parent ID and major:minor fields.
        if let Some(root) = fields.nth(3)
            && let Some(mount_point) = fields.next()
            // Skip mount options and optional fields up to the standalone "-" separator.
            && fields.by_ref().skip(1).any(|field| field == "-")
            && let Some(filesystem_type) = fields.next()
            // Skip the mount source field.
            && let Some(super_options) = fields.nth(1)
        {
            let mount = CGroupMount {
                root: normalize_mountinfo_path(root),
                mount_point: PathBuf::from(decode_mountinfo_path(mount_point)),
            };

            match filesystem_type {
                "cgroup2" => mounts.v2.push(mount),
                "cgroup" if super_options.split(',').any(|option| option == "memory") => {
                    mounts.v1_memory.push(mount)
                }
                _ => (),
            }
        }
    }

    mounts
}

fn parse_cgroup_path(content: &str) -> CGroupPath {
    let mut cgroup_path = CGroupPath::default();

    for line in content.lines() {
        let mut fields = line.splitn(3, ':');
        if let Some(hierarchy_id) = fields.next()
            && let Some(controllers) = fields.next()
            && let Some(path) = fields.next()
        {
            if hierarchy_id == "0" && controllers.is_empty() {
                cgroup_path.v2 = Some(normalize_cgroup_path(path));
            } else if controllers
                .split(',')
                .any(|controller| controller == "memory")
            {
                cgroup_path.v1_memory = Some(normalize_cgroup_path(path));
            }
        }
    }

    cgroup_path
}

fn normalize_cgroup_path(path: &str) -> PathBuf {
    if let Ok(path) = Path::new(path).strip_prefix("/") {
        return path.to_path_buf();
    }

    PathBuf::from(path)
}

fn normalize_mountinfo_path(path: &str) -> PathBuf {
    let path = decode_mountinfo_path(path);

    if let Ok(path) = Path::new(&path).strip_prefix("/") {
        return path.to_path_buf();
    }

    PathBuf::from(path)
}

// The kernel renders mountinfo root and mount point fields with octal
// escapes for space, tab, newline and backslash (seq_escape() via
// show_mountinfo() in fs/proc_namespace.c), e.g. `\040` for a space.
// Decode them so the paths can actually be looked up on the filesystem.
fn decode_mountinfo_path(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut pos = 0;

    while pos < bytes.len() {
        if bytes[pos] == b'\\'
            && pos + 3 < bytes.len()
            && let Some(value) = decode_octal_escape(&bytes[pos + 1..pos + 4])
        {
            decoded.push(value);
            pos += 4;
            continue;
        }

        decoded.push(bytes[pos]);
        pos += 1;
    }

    String::from_utf8(decoded).unwrap_or_else(|_| path.to_owned())
}

fn decode_octal_escape(digits: &[u8]) -> Option<u8> {
    let mut value = 0;

    for digit in digits {
        if !(b'0'..=b'7').contains(digit) {
            return None;
        }
        value = value * 8 + (digit - b'0');
    }

    Some(value)
}

#[cfg(test)]
mod test {
    use super::CGroupBase;
    use super::CGroupLimitsContext;
    use super::CGroupMount;
    use super::CGroupMounts;
    use super::CGroupPath;
    use super::cgroup_base_for_path;
    use super::decode_mountinfo_path;
    use super::limits_for_process;
    use super::limits_for_process_with_context;
    use super::parse_cgroup_mounts;
    use super::parse_cgroup_path;
    use super::read_table;
    use super::read_table_key;
    use super::v1_limits;
    use super::v2_limits;
    use std::collections::HashMap;
    use std::fs::{create_dir_all, write};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use tempfile::{NamedTempFile, tempdir};

    #[test]
    fn test_read_table() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "KEY1:100 kB").unwrap();
        writeln!(file, "KEY2:200 kB").unwrap();
        writeln!(file, "KEY3:300 kB").unwrap();
        writeln!(file, "KEY4:invalid").unwrap();

        let file_path = file.path().to_str().unwrap();

        let mut result = HashMap::new();
        read_table(file_path, ':', |key, value| {
            result.insert(key.to_string(), value);
        });

        assert_eq!(result.get("KEY1"), Some(&100));
        assert_eq!(result.get("KEY2"), Some(&200));
        assert_eq!(result.get("KEY3"), Some(&300));
        assert_eq!(result.get("KEY4"), None);

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "KEY1 400 MB").unwrap();
        writeln!(file, "KEY2 500 GB").unwrap();
        writeln!(file, "KEY3 600").unwrap();

        let file_path = file.path().to_str().unwrap();

        let mut result = HashMap::new();
        read_table(file_path, ' ', |key, value| {
            result.insert(key.to_string(), value);
        });

        assert_eq!(result.get("KEY1"), Some(&400));
        assert_eq!(result.get("KEY2"), Some(&500));
        assert_eq!(result.get("KEY3"), Some(&600));

        let file = NamedTempFile::new().unwrap();
        let file_path = file.path().to_str().unwrap();

        let mut result = HashMap::new();
        read_table(file_path, ':', |key, value| {
            result.insert(key.to_string(), value);
        });

        assert!(result.is_empty());

        let mut result = HashMap::new();
        read_table("/nonexistent/file", ':', |key, value| {
            result.insert(key.to_string(), value);
        });

        assert!(result.is_empty());
    }

    #[test]
    fn test_read_table_key() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "KEY1:100 kB").unwrap();
        writeln!(file, "KEY2:200 kB").unwrap();
        writeln!(file, "KEY3:300 kB").unwrap();

        let file_path = file.path();

        assert_eq!(read_table_key(file_path, "KEY1", ':'), Some(100));
        assert_eq!(read_table_key(file_path, "KEY2", ':'), Some(200));
        assert_eq!(read_table_key(file_path, "KEY3", ':'), Some(300));
        assert_eq!(read_table_key(file_path, "KEY4", ':'), None);

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "KEY1 400 kB").unwrap();
        writeln!(file, "KEY2 500 kB").unwrap();

        let file_path = file.path();

        assert_eq!(read_table_key(file_path, "KEY1", ' '), Some(400));
        assert_eq!(read_table_key(file_path, "KEY2", ' '), Some(500));
        assert_eq!(
            read_table_key(Path::new("/nonexistent/file"), "KEY1", ':'),
            None
        );
    }

    #[test]
    fn test_v2_parent_limit() {
        let root = tempdir().unwrap();
        let parent = root.path().join("parent");
        let child = parent.join("child");

        create_dir_all(&child).unwrap();
        write(root.path().join("memory.max"), "max").unwrap();
        write(parent.join("memory.max"), "500").unwrap();
        write(parent.join("memory.current"), "350").unwrap();
        write(child.join("memory.max"), "max").unwrap();
        write(child.join("memory.current"), "100").unwrap();
        write(child.join("memory.stat"), "anon 30\n").unwrap();

        let limits = v2_limits(
            &child,
            root.path(),
            CGroupLimitsContext {
                mem_total: 2000,
                swap_total: 1000,
                swap_free: 700,
            },
        )
        .unwrap();

        assert_eq!(limits.total_memory, 500);
        assert_eq!(limits.free_memory, 150);
        assert_eq!(limits.free_swap, 700);
        assert_eq!(limits.rss, 30);
    }

    #[test]
    fn test_v2_parent_free_memory() {
        let root = tempdir().unwrap();
        let parent = root.path().join("parent");
        let child = parent.join("child");

        create_dir_all(&child).unwrap();
        write(root.path().join("memory.max"), "max").unwrap();
        write(parent.join("memory.max"), "500").unwrap();
        write(parent.join("memory.current"), "450").unwrap();
        write(child.join("memory.max"), "200").unwrap();
        write(child.join("memory.current"), "100").unwrap();
        write(child.join("memory.stat"), "anon 30\n").unwrap();

        let limits = v2_limits(
            &child,
            root.path(),
            CGroupLimitsContext {
                mem_total: 2000,
                swap_total: 1000,
                swap_free: 700,
            },
        )
        .unwrap();

        assert_eq!(limits.total_memory, 200);
        assert_eq!(limits.free_memory, 50);
    }

    #[test]
    fn test_v2_unlimited_memory() {
        let root = tempdir().unwrap();
        let child = root.path().join("child");

        create_dir_all(&child).unwrap();
        write(root.path().join("memory.max"), "max").unwrap();
        write(root.path().join("memory.current"), "350").unwrap();
        write(child.join("memory.max"), "max").unwrap();
        write(child.join("memory.current"), "100").unwrap();
        write(child.join("memory.stat"), "anon 30\n").unwrap();

        let limits = v2_limits(
            &child,
            root.path(),
            CGroupLimitsContext {
                mem_total: 2000,
                swap_total: 1000,
                swap_free: 700,
            },
        );

        assert!(limits.is_none());
    }

    #[test]
    fn test_v1_parent_limit() {
        let root = tempdir().unwrap();
        let parent = root.path().join("parent");
        let child = parent.join("child");

        create_dir_all(&child).unwrap();
        write(
            root.path().join("memory.limit_in_bytes"),
            u64::MAX.to_string(),
        )
        .unwrap();
        write(parent.join("memory.limit_in_bytes"), "500").unwrap();
        write(parent.join("memory.usage_in_bytes"), "350").unwrap();
        write(child.join("memory.limit_in_bytes"), u64::MAX.to_string()).unwrap();
        write(child.join("memory.usage_in_bytes"), "100").unwrap();
        write(child.join("memory.stat"), "total_rss 30\n").unwrap();

        let limits = v1_limits(
            &child,
            root.path(),
            CGroupLimitsContext {
                mem_total: 2000,
                swap_total: 1000,
                swap_free: 700,
            },
        )
        .unwrap();

        assert_eq!(limits.total_memory, 500);
        assert_eq!(limits.free_memory, 150);
        assert_eq!(limits.rss, 30);
    }

    #[test]
    fn test_v1_unlimited_memory() {
        let root = tempdir().unwrap();
        let child = root.path().join("child");

        create_dir_all(&child).unwrap();
        write(
            root.path().join("memory.limit_in_bytes"),
            u64::MAX.to_string(),
        )
        .unwrap();
        write(root.path().join("memory.usage_in_bytes"), "350").unwrap();
        write(child.join("memory.limit_in_bytes"), u64::MAX.to_string()).unwrap();
        write(child.join("memory.usage_in_bytes"), "100").unwrap();
        write(child.join("memory.stat"), "total_rss 30\n").unwrap();

        let limits = v1_limits(
            &child,
            root.path(),
            CGroupLimitsContext {
                mem_total: 2000,
                swap_total: 1000,
                swap_free: 700,
            },
        );

        assert!(limits.is_none());
    }

    fn write_memory_cgroup(path: &Path, v1: bool, limit: &str) {
        create_dir_all(path).unwrap();
        let (limit_file, usage_file, stat) = if v1 {
            (
                "memory.limit_in_bytes",
                "memory.usage_in_bytes",
                "total_rss 30\n",
            )
        } else {
            ("memory.max", "memory.current", "anon 30\n")
        };
        write(path.join(limit_file), limit).unwrap();
        write(path.join(usage_file), "100").unwrap();
        write(path.join("memory.stat"), stat).unwrap();
    }

    const CONTEXT: CGroupLimitsContext = CGroupLimitsContext {
        mem_total: 2000,
        swap_total: 1000,
        swap_free: 700,
    };

    #[test]
    fn test_memory_limit_read_errors() {
        for v1 in [false, true] {
            for level in ["", "parent", "parent/child"] {
                for failure in ["missing", "malformed", "unreadable"] {
                    let root = tempdir().unwrap();
                    let child = root.path().join("parent/child");
                    write_memory_cgroup(root.path(), v1, "1000");
                    write_memory_cgroup(&root.path().join("parent"), v1, "500");
                    write_memory_cgroup(&child, v1, "200");
                    let limits = if v1 { v1_limits } else { v2_limits };
                    assert_eq!(
                        limits(&child, root.path(), CONTEXT).unwrap().total_memory,
                        200
                    );

                    let filename = if v1 {
                        "memory.limit_in_bytes"
                    } else {
                        "memory.max"
                    };
                    let path = root.path().join(level).join(filename);
                    std::fs::remove_file(&path).unwrap();
                    match failure {
                        "malformed" => write(&path, "invalid").unwrap(),
                        // A directory gives a read error even when tests run as root.
                        "unreadable" => create_dir_all(&path).unwrap(),
                        _ => {}
                    }
                    assert!(
                        limits(&child, root.path(), CONTEXT).is_none(),
                        "v1={v1}, level={level}, failure={failure}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_v2_root_without_memory_max() {
        let root = tempdir().unwrap();
        let child = root.path().join("child");
        write_memory_cgroup(&child, false, "200");
        assert!(v2_limits(&child, root.path(), CONTEXT).is_none());

        write(root.path().join("cgroup.controllers"), "memory\n").unwrap();
        assert_eq!(
            v2_limits(&child, root.path(), CONTEXT)
                .unwrap()
                .total_memory,
            200
        );

        // A mount rooted at a non-root cgroup must have its own limit read.
        write(root.path().join("cgroup.type"), "domain\n").unwrap();
        assert!(v2_limits(&child, root.path(), CONTEXT).is_none());
        std::fs::remove_file(root.path().join("cgroup.type")).unwrap();

        write(root.path().join("memory.max"), "invalid").unwrap();
        assert!(v2_limits(&child, root.path(), CONTEXT).is_none());
        std::fs::remove_file(root.path().join("memory.max")).unwrap();
        create_dir_all(root.path().join("memory.max")).unwrap();
        assert!(v2_limits(&child, root.path(), CONTEXT).is_none());
        std::fs::remove_dir(root.path().join("memory.max")).unwrap();

        // Missing limits below the root remain errors, even with the same marker files.
        write(child.join("cgroup.controllers"), "memory\n").unwrap();
        std::fs::remove_file(child.join("memory.max")).unwrap();
        assert!(v2_limits(&child, root.path(), CONTEXT).is_none());
    }

    #[test]
    fn test_mountinfo_resolves_memory_path() {
        for v1 in [false, true] {
            let root = tempdir().unwrap();
            let mount = root.path().join("remapped-memory");
            let child = mount.join("pod/container");
            write_memory_cgroup(&mount, v1, "1500");
            write_memory_cgroup(&mount.join("pod"), v1, "500");
            write_memory_cgroup(&child, v1, "1000");
            let (membership, filesystem) = if v1 {
                (
                    "11:memory:/kubepods/pod/container\n",
                    "cgroup cgroup rw,memory",
                )
            } else {
                ("0::/kubepods/pod/container\n", "cgroup2 cgroup rw")
            };
            let cgroup_path = parse_cgroup_path(membership);
            let mounts = parse_cgroup_mounts(&format!(
                "30 23 0:25 /kubepods {} rw - {filesystem}\n",
                mount.display(),
            ));
            let matching_mounts = if v1 { &mounts.v1_memory } else { &mounts.v2 };
            assert_eq!(
                cgroup_base_for_path(Path::new("kubepods/pod/container"), matching_mounts),
                Some(CGroupBase::new(child, mount)),
            );
            let limits = limits_for_process_with_context(&cgroup_path, &mounts, CONTEXT).unwrap();
            assert_eq!(limits.total_memory, 500);
            assert_eq!(limits.free_memory, 400);
            assert_eq!(limits.rss, 30);
        }
    }

    #[test]
    fn test_hybrid_cgroup_prefers_v1_memory_path() {
        for v2_limit in ["max", "1500"] {
            let root = tempdir().unwrap();
            let v1_root = root.path().join("memory");
            let v2_root = root.path().join("unified");
            write_memory_cgroup(&v1_root, true, "1000");
            write_memory_cgroup(&v1_root.join("child"), true, "500");
            write_memory_cgroup(&v2_root, false, v2_limit);
            write_memory_cgroup(&v2_root.join("child"), false, v2_limit);
            let membership = parse_cgroup_path("0::/child\n11:memory:/child\n");
            let mounts = parse_cgroup_mounts(&format!(
                "30 23 0:25 / {} rw - cgroup cgroup rw,memory\n\
                 31 23 0:26 / {} rw - cgroup2 cgroup rw\n",
                v1_root.display(),
                v2_root.display(),
            ));
            let limits = limits_for_process_with_context(&membership, &mounts, CONTEXT).unwrap();
            assert_eq!(limits.total_memory, 500);
        }
    }

    #[test]
    fn test_process_limits_do_not_fall_back() {
        for v1 in [false, true] {
            let root = tempdir().unwrap();
            let selected = root.path().join("selected");
            let alternative = root.path().join("alternative");
            let v2_root = root.path().join("unified");
            write_memory_cgroup(&selected, v1, "1000");
            write_memory_cgroup(&alternative, v1, "1000");
            write_memory_cgroup(&alternative.join("child"), v1, "1000");
            write_memory_cgroup(&v2_root, false, "1500");
            write_memory_cgroup(&v2_root.join("child"), false, "1500");
            let (membership, filesystem) = if v1 {
                ("0::/child\n11:memory:/child\n", "cgroup cgroup rw,memory")
            } else {
                ("0::/child\n", "cgroup2 cgroup rw")
            };
            let membership = parse_cgroup_path(membership);
            let mut mounts = parse_cgroup_mounts(&format!(
                "30 23 0:25 / {} rw - {filesystem}\n\
                 31 23 0:25 / {} rw - {filesystem}\n\
                 32 23 0:26 / {} rw - cgroup2 cgroup rw\n",
                selected.display(),
                alternative.display(),
                v2_root.display(),
            ));
            // The selected child is missing; neither its root nor another mount is a substitute.
            assert!(limits_for_process_with_context(&membership, &mounts, CONTEXT).is_none());
            write_memory_cgroup(&selected.join("child"), v1, "invalid");
            assert!(limits_for_process_with_context(&membership, &mounts, CONTEXT).is_none());
            if v1 {
                mounts.v1_memory.clear();
            } else {
                mounts.v2.clear();
            }
            assert!(limits_for_process_with_context(&membership, &mounts, CONTEXT).is_none());
        }

        let proc_path = tempdir().unwrap();
        write(proc_path.path().join("cgroup"), "0::/child\n").unwrap();
        assert!(limits_for_process(proc_path.path()).is_none());
        write(proc_path.path().join("mountinfo"), "invalid").unwrap();
        assert!(limits_for_process(proc_path.path()).is_none());
    }

    #[test]
    fn test_parse_cgroup_mounts() {
        assert_eq!(
            parse_cgroup_mounts(
                "29 23 0:28 /kubepods\\040burstable /sys/fs/cgroup/memory\\040controller rw,nosuid,nodev,noexec - cgroup cgroup rw,memory\n\
                 30 23 0:29 / /sys/fs/cgroup rw,nosuid,nodev,noexec shared:1 master:2 propagate_from:3 unbindable x-extra:y- - cgroup2 cgroup rw\n\
                 31 23 0:30 / /sys/fs/cgroup/cpu rw,nosuid,nodev,noexec - cgroup cgroup rw,cpu\n",
            ),
            CGroupMounts {
                v2: vec![CGroupMount {
                    root: PathBuf::new(),
                    mount_point: PathBuf::from("/sys/fs/cgroup"),
                }],
                v1_memory: vec![CGroupMount {
                    root: PathBuf::from("kubepods burstable"),
                    mount_point: PathBuf::from("/sys/fs/cgroup/memory controller"),
                }],
            }
        );
    }

    #[test]
    fn test_decode_mountinfo_path() {
        assert_eq!(
            decode_mountinfo_path("/sys/fs/cgroup/memory\\040controller"),
            "/sys/fs/cgroup/memory controller"
        );
        assert_eq!(decode_mountinfo_path("tab\\011path"), "tab\tpath");
        assert_eq!(decode_mountinfo_path("newline\\012path"), "newline\npath");
        assert_eq!(
            decode_mountinfo_path("backslash\\134path"),
            "backslash\\path"
        );
        assert_eq!(decode_mountinfo_path("plain/path"), "plain/path");
        // Incomplete or non-octal escapes are kept as-is.
        assert_eq!(decode_mountinfo_path("trailing\\04"), "trailing\\04");
        assert_eq!(decode_mountinfo_path("invalid\\099"), "invalid\\099");
    }

    #[test]
    fn test_parse_cgroup_path_keeps_literal_path_bytes() {
        assert_eq!(
            parse_cgroup_path("11:memory:/kubepods\\040literal/a:b c\n0::/unified\\011path\n"),
            CGroupPath {
                v2: Some(PathBuf::from("unified\\011path")),
                v1_memory: Some(PathBuf::from("kubepods\\040literal/a:b c")),
            }
        );
    }

    #[test]
    fn test_parse_cgroup_path_v2() {
        assert_eq!(
            parse_cgroup_path("0::/user.slice/session.scope"),
            CGroupPath {
                v2: Some(PathBuf::from("user.slice/session.scope")),
                v1_memory: None,
            }
        );
    }

    #[test]
    fn test_parse_cgroup_path_v1_memory() {
        assert_eq!(
            parse_cgroup_path("12:cpuset:/\n11:memory:/system.slice/service.scope"),
            CGroupPath {
                v2: None,
                v1_memory: Some(PathBuf::from("system.slice/service.scope")),
            }
        );
    }

    #[test]
    fn test_parse_cgroup_path_hybrid() {
        assert_eq!(
            parse_cgroup_path(
                "0::/system.slice/service.scope\n11:memory:/kubepods/pod/container\n"
            ),
            CGroupPath {
                v2: Some(PathBuf::from("system.slice/service.scope")),
                v1_memory: Some(PathBuf::from("kubepods/pod/container")),
            }
        );
    }

    #[test]
    fn test_parse_cgroup_path_missing_memory_controller() {
        assert_eq!(
            parse_cgroup_path("12:cpuset:/\n10:cpu:/"),
            CGroupPath::default()
        );
    }
}
