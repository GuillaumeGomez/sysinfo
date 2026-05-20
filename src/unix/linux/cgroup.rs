// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::utils::get_all_utf8_data;

use std::cmp::min;
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

pub(crate) fn limits_for_system() -> Option<crate::CGroupLimits> {
    let v2_base = Path::new("/sys/fs/cgroup");
    let v1_base = Path::new("/sys/fs/cgroup/memory");

    limits_for_base(Some(v2_base), v2_base, Some(v1_base), v1_base)
}

pub(crate) fn limits_for_process(proc_path: &Path) -> Option<crate::CGroupLimits> {
    let cgroup_path = get_cgroup_path(&proc_path.join("cgroup"))?;
    let v2_root = Path::new("/sys/fs/cgroup");
    let v1_root = Path::new("/sys/fs/cgroup/memory");
    let v2_base = cgroup_path.v2.as_ref().map(|path| v2_root.join(path));
    let v1_base = cgroup_path
        .v1_memory
        .as_ref()
        .map(|path| v1_root.join(path));

    limits_for_base(v2_base.as_deref(), v2_root, v1_base.as_deref(), v1_root)
}

// A process may have only a cgroup v2 entry, only a v1 memory controller entry,
// or both. Keep the bases optional so hybrid cgroups don't reuse the v2 path for
// the v1 memory controller.
fn limits_for_base(
    v2_base: Option<&Path>,
    v2_root: &Path,
    v1_base: Option<&Path>,
    v1_root: &Path,
) -> Option<crate::CGroupLimits> {
    let context = read_cgroup_limits_context()?;
    limits_for_base_with_context(v2_base, v2_root, v1_base, v1_root, context)
}

fn limits_for_base_with_context(
    v2_base: Option<&Path>,
    v2_root: &Path,
    v1_base: Option<&Path>,
    v1_root: &Path,
    context: CGroupLimitsContext,
) -> Option<crate::CGroupLimits> {
    v1_base
        .and_then(|v1_base| v1_limits(v1_base, v1_root, context))
        .or_else(|| v2_base.and_then(|v2_base| v2_limits(v2_base, v2_root, context)))
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
    let mem_max = read_v2_memory_max(&base.join("memory.max"));
    let (total_memory, free_memory) = memory_limits(
        base,
        root,
        "memory.max",
        "memory.current",
        context.mem_total,
        mem_max,
        read_v2_memory_max,
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
        |path| read_u64(path).unwrap_or(u64::MAX),
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
    F: Fn(&Path) -> u64,
{
    let mem_cur = read_u64(&base.join(usage_file))?;
    let mut total_memory = None;
    let mut free_memory = None;

    for (pos, path) in base.ancestors().enumerate() {
        let is_base = pos == 0;
        let mem_max = if is_base {
            base_limit
        } else {
            read_limit(&path.join(limit_file))
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

fn read_v2_memory_max(filename: &Path) -> u64 {
    let content = match get_all_utf8_data(filename, 16_635) {
        Ok(content) => content,
        Err(_) => {
            sysinfo_debug!("Failed to read u64 in filename {filename:?}");
            return u64::MAX;
        }
    };
    let content = content.trim();
    if content == "max" {
        return u64::MAX;
    }

    match u64::from_str(content).ok() {
        Some(value) => value,
        None => {
            sysinfo_debug!("Failed to read u64 in filename {filename:?}");
            u64::MAX
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
    if cgroup_path.v2.is_none() && cgroup_path.v1_memory.is_none() {
        return None;
    }
    Some(cgroup_path)
}

fn parse_cgroup_path(content: &str) -> CGroupPath {
    let mut cgroup_path = CGroupPath::default();

    for line in content.lines() {
        let mut fields = line.splitn(3, ':');
        let Some(hierarchy_id) = fields.next() else {
            continue;
        };
        let Some(controllers) = fields.next() else {
            continue;
        };
        let Some(path) = fields.next() else {
            continue;
        };

        if hierarchy_id == "0" && controllers.is_empty() {
            cgroup_path.v2 = Some(normalize_cgroup_path(path));
            continue;
        }

        if controllers
            .split(',')
            .any(|controller| controller == "memory")
        {
            cgroup_path.v1_memory = Some(normalize_cgroup_path(path));
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

#[cfg(test)]
mod test {
    use super::CGroupLimitsContext;
    use super::CGroupPath;
    use super::limits_for_base_with_context;
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

    #[test]
    fn test_hybrid_cgroup_uses_v1_memory_path_when_v2_is_unlimited() {
        let root = tempdir().unwrap();
        let v2_root = root.path().join("unified");
        let v2_child = v2_root.join("system.slice/service.scope");
        let v1_root = root.path().join("memory");
        let v1_child = v1_root.join("memory.slice/service.scope");
        let host_memory = 32 * 1024 * 1024 * 1024;
        let cgroup_limit = 8 * 1024 * 1024 * 1024;
        let cgroup_usage = 1024 * 1024 * 1024;

        create_dir_all(&v2_child).unwrap();
        create_dir_all(&v1_child).unwrap();
        write(v2_root.join("memory.max"), "max").unwrap();
        write(v2_root.join("memory.current"), cgroup_usage.to_string()).unwrap();
        write(v2_child.join("memory.max"), "max").unwrap();
        write(v2_child.join("memory.current"), cgroup_usage.to_string()).unwrap();
        write(v2_child.join("memory.stat"), "anon 1073741824\n").unwrap();
        write(
            v1_child.join("memory.limit_in_bytes"),
            cgroup_limit.to_string(),
        )
        .unwrap();
        write(
            v1_child.join("memory.usage_in_bytes"),
            cgroup_usage.to_string(),
        )
        .unwrap();
        write(v1_child.join("memory.stat"), "total_rss 1073741824\n").unwrap();

        let cgroup_path = parse_cgroup_path(
            "0::/system.slice/service.scope\n\
             11:memory:/memory.slice/service.scope\n",
        );
        let v2_base = cgroup_path.v2.as_ref().map(|path| v2_root.join(path));
        let v1_base = cgroup_path
            .v1_memory
            .as_ref()
            .map(|path| v1_root.join(path));

        let limits = limits_for_base_with_context(
            v2_base.as_deref(),
            &v2_root,
            v1_base.as_deref(),
            &v1_root,
            CGroupLimitsContext {
                mem_total: host_memory,
                swap_total: 0,
                swap_free: 0,
            },
        )
        .unwrap();

        assert_eq!(limits.total_memory, cgroup_limit);
        assert_eq!(limits.free_memory, cgroup_limit - cgroup_usage);
        assert_eq!(limits.rss, cgroup_usage);
    }

    #[test]
    fn test_hybrid_cgroup_prefers_v1_memory_path_over_v2_limit() {
        let root = tempdir().unwrap();
        let v2_root = root.path().join("unified");
        let v2_child = v2_root.join("system.slice/service.scope");
        let v1_root = root.path().join("memory");
        let v1_child = v1_root.join("memory.slice/service.scope");
        let v2_limit = 32 * 1024 * 1024 * 1024;
        let v1_limit = 8 * 1024 * 1024 * 1024;
        let cgroup_usage = 1024 * 1024 * 1024;

        create_dir_all(&v2_child).unwrap();
        create_dir_all(&v1_child).unwrap();
        write(v2_root.join("memory.max"), v2_limit.to_string()).unwrap();
        write(v2_root.join("memory.current"), cgroup_usage.to_string()).unwrap();
        write(v2_child.join("memory.max"), v2_limit.to_string()).unwrap();
        write(v2_child.join("memory.current"), cgroup_usage.to_string()).unwrap();
        write(v2_child.join("memory.stat"), "anon 1073741824\n").unwrap();
        write(v1_child.join("memory.limit_in_bytes"), v1_limit.to_string()).unwrap();
        write(
            v1_child.join("memory.usage_in_bytes"),
            cgroup_usage.to_string(),
        )
        .unwrap();
        write(v1_child.join("memory.stat"), "total_rss 1073741824\n").unwrap();

        let limits = limits_for_base_with_context(
            Some(&v2_child),
            &v2_root,
            Some(&v1_child),
            &v1_root,
            CGroupLimitsContext {
                mem_total: v2_limit,
                swap_total: 0,
                swap_free: 0,
            },
        )
        .unwrap();

        assert_eq!(limits.total_memory, v1_limit);
        assert_eq!(limits.free_memory, v1_limit - cgroup_usage);
        assert_eq!(limits.rss, cgroup_usage);
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
