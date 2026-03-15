// Take a look at the license at the top of the repository in the LICENSE file.

use crate::sys::utils::get_all_utf8_data;

use std::cmp::min;
use std::path::Path;
use std::str::FromStr;

#[derive(Clone, Copy)]
struct CGroupLimitsContext {
    mem_total: u64,
    swap_total: u64,
    swap_free: u64,
}

pub(crate) fn limits_for_system() -> Option<crate::CGroupLimits> {
    limits_for_base("/sys/fs/cgroup", "/sys/fs/cgroup/memory")
}

pub(crate) fn limits_for_process(proc_path: &Path) -> Option<crate::CGroupLimits> {
    let cgroup_path = get_cgroup_path(proc_path.join("cgroup"))?;
    let v2_base = format!("/sys/fs/cgroup{cgroup_path}");
    let v1_base = format!("/sys/fs/cgroup/memory{cgroup_path}");

    limits_for_base(&v2_base, &v1_base)
}

fn limits_for_base(v2_base: &str, v1_base: &str) -> Option<crate::CGroupLimits> {
    let context = read_cgroup_limits_context()?;
    new_v2(v2_base, context).or_else(|| new_v1(v1_base, context))
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

fn new_v2(base: &str, context: CGroupLimitsContext) -> Option<crate::CGroupLimits> {
    let mem_cur = read_u64(format!("{base}/memory.current"))?;
    // `memory.max` contains `max` when no limit is set.
    let mem_max = read_u64(format!("{base}/memory.max")).or(Some(u64::MAX))?;
    let mem_rss = read_table_key(format!("{base}/memory.stat"), "anon", ' ')?;

    let mut limits = crate::CGroupLimits {
        total_memory: min(mem_max, context.mem_total),
        free_memory: 0,
        free_swap: context.swap_free,
        rss: mem_rss,
    };
    limits.free_memory = limits.total_memory.saturating_sub(mem_cur);

    if let Some(swap_cur) = read_u64(format!("{base}/memory.swap.current")) {
        limits.free_swap = context.swap_total.saturating_sub(swap_cur);
    }

    Some(limits)
}

fn new_v1(base: &str, context: CGroupLimitsContext) -> Option<crate::CGroupLimits> {
    let mem_cur = read_u64(format!("{base}/memory.usage_in_bytes"))?;
    let mem_max = read_u64(format!("{base}/memory.limit_in_bytes"))?;
    let mem_rss = read_table_key(format!("{base}/memory.stat"), "total_rss", ' ')?;

    let mut limits = crate::CGroupLimits {
        total_memory: min(mem_max, context.mem_total),
        free_memory: 0,
        free_swap: context.swap_free,
        rss: mem_rss,
    };

    limits.free_memory = limits.total_memory.saturating_sub(mem_cur);
    Some(limits)
}

fn read_u64<P: AsRef<Path>>(filename: P) -> Option<u64> {
    let path = filename.as_ref();
    let result = get_all_utf8_data(path, 16_635)
        .ok()
        .and_then(|d| u64::from_str(d.trim()).ok());

    if result.is_none() {
        sysinfo_debug!("Failed to read u64 in filename {path:?}");
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

fn read_table_key<P: AsRef<Path>>(filename: P, target_key: &str, colsep: char) -> Option<u64> {
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

fn get_cgroup_path<P: AsRef<Path>>(path: P) -> Option<String> {
    let content = get_all_utf8_data(path, 4096).ok()?;
    parse_cgroup_path(&content)
}

fn parse_cgroup_path(content: &str) -> Option<String> {
    for line in content.lines() {
        let mut fields = line.splitn(3, ':');
        let hierarchy_id = fields.next()?;
        let controllers = fields.next()?;
        let path = fields.next()?;

        if (hierarchy_id == "0" && controllers.is_empty())
            || controllers
                .split(',')
                .any(|controller| controller == "memory")
        {
            return Some(path.to_owned());
        }
    }
    None
}

#[cfg(test)]
mod test {
    use super::parse_cgroup_path;
    use super::read_table;
    use super::read_table_key;
    use std::collections::HashMap;
    use std::io::Write;
    use tempfile::NamedTempFile;

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

        let file_path = file.path().to_str().unwrap();

        assert_eq!(read_table_key(file_path, "KEY1", ':'), Some(100));
        assert_eq!(read_table_key(file_path, "KEY2", ':'), Some(200));
        assert_eq!(read_table_key(file_path, "KEY3", ':'), Some(300));
        assert_eq!(read_table_key(file_path, "KEY4", ':'), None);

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "KEY1 400 kB").unwrap();
        writeln!(file, "KEY2 500 kB").unwrap();

        let file_path = file.path().to_str().unwrap();

        assert_eq!(read_table_key(file_path, "KEY1", ' '), Some(400));
        assert_eq!(read_table_key(file_path, "KEY2", ' '), Some(500));
        assert_eq!(read_table_key("/nonexistent/file", "KEY1", ':'), None);
    }

    #[test]
    fn test_parse_cgroup_path_v2() {
        assert_eq!(
            parse_cgroup_path("0::/user.slice/session.scope"),
            Some("/user.slice/session.scope".to_owned())
        );
    }

    #[test]
    fn test_parse_cgroup_path_v1_memory() {
        assert_eq!(
            parse_cgroup_path("12:cpuset:/\n11:memory:/system.slice/service.scope"),
            Some("/system.slice/service.scope".to_owned()),
        );
    }

    #[test]
    fn test_parse_cgroup_path_missing_memory_controller() {
        assert_eq!(parse_cgroup_path("12:cpuset:/\n10:cpu:/"), None);
    }
}
