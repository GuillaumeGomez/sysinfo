// Take a look at the license at the top of the repository in the LICENSE file.

#![cfg(feature = "system")]

use sysinfo::{CGroupLimits, Pid};

#[test]
fn invalid_pid() {
    assert!(CGroupLimits::for_pid(Pid::from_u32(0)).is_none());
}

#[cfg(any(
    feature = "unknown-ci",
    not(any(target_os = "linux", target_os = "android")),
))]
#[test]
fn unsupported_platform() {
    assert!(CGroupLimits::for_pid(Pid::from_u32(std::process::id())).is_none());
}

#[cfg(all(
    not(feature = "unknown-ci"),
    any(target_os = "linux", target_os = "android"),
))]
#[test]
fn query_preserves_file_descriptor_limits() {
    const CHILD: &str = "SYSINFO_CGROUP_LIMITS_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        // Use a fresh process so earlier tests cannot initialize the process-refresh state.
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "query_preserves_file_descriptor_limits"])
            .env(CHILD, "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }

    let mut before = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    assert_eq!(
        unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut before) },
        0
    );
    // Keep the soft limit below the hard limit so a process refresh would change it.
    before.rlim_cur = before.rlim_max.saturating_sub(1).min(1024);
    assert!(before.rlim_cur > 0 && before.rlim_cur < before.rlim_max);
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &before) }, 0);

    let pid = sysinfo::get_current_pid().unwrap();
    let direct = CGroupLimits::for_pid(pid);
    let mut after = before;
    assert_eq!(
        unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut after) },
        0
    );
    assert_eq!(after.rlim_cur, before.rlim_cur);
    assert_eq!(after.rlim_max, before.rlim_max);

    let mut system = sysinfo::System::new().unwrap();
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[pid]),
        false,
        sysinfo::ProcessRefreshKind::nothing(),
    );
    let process = system.process(pid).unwrap().cgroup_limits();
    // Usage can change between reads, so compare only the capacity.
    assert_eq!(
        direct.map(|limits| limits.total_memory),
        process.map(|limits| limits.total_memory)
    );
}
