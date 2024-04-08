// Take a look at the license at the top of the repository in the LICENSE file.

use sysinfo::{Pid, ProcessRefreshKind, System, UpdateKind};

#[test]
fn test_cwd() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }
    let mut p = if cfg!(target_os = "windows") {
        std::process::Command::new("waitfor")
            .arg("/t")
            .arg("3")
            .arg("CwdSignal")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    } else {
        std::process::Command::new("sleep")
            .arg("3")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };

    let pid = Pid::from_u32(p.id() as _);
    std::thread::sleep(std::time::Duration::from_secs(1));
    let mut s = System::new();
    s.refresh_processes_specifics(ProcessRefreshKind::new().with_cwd(UpdateKind::Always));
    p.kill().expect("Unable to kill process.");

    let processes = s.processes();
    let p = processes.get(&pid);

    if let Some(p) = p {
        assert_eq!(p.pid(), pid);
        assert_eq!(p.cwd().unwrap(), &std::env::current_dir().unwrap());
    } else {
        panic!("Process not found!");
    }
}

#[test]
fn test_cmd() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }
    let mut p = if cfg!(target_os = "windows") {
        std::process::Command::new("waitfor")
            .arg("/t")
            .arg("3")
            .arg("CmdSignal")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    } else {
        std::process::Command::new("sleep")
            .arg("3")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };
    std::thread::sleep(std::time::Duration::from_millis(500));
    let mut s = System::new();
    assert!(s.processes().is_empty());
    s.refresh_processes_specifics(ProcessRefreshKind::new().with_cmd(UpdateKind::Always));
    p.kill().expect("Unable to kill process");
    assert!(!s.processes().is_empty());
    if let Some(process) = s.process(Pid::from_u32(p.id() as _)) {
        if cfg!(target_os = "windows") {
            // Sometimes, we get the full path instead for some reasons... So just in case,
            // we check for the command independently that from the arguments.
            assert!(process.cmd()[0].contains("waitfor"));
            assert_eq!(&process.cmd()[1..], &["/t", "3", "CmdSignal"]);
        } else {
            assert_eq!(process.cmd(), &["sleep", "3"]);
        }
    } else {
        panic!("Process not found!");
    }
}

fn build_test_binary(file_name: &str) {
    std::process::Command::new("rustc")
        .arg("test_bin/main.rs")
        .arg("-o")
        .arg(file_name)
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

#[test]
fn test_environ() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }
    let file_name = "target/test_binary";
    build_test_binary(file_name);
    let mut p = std::process::Command::new(format!("./{file_name}"))
        .env("FOO", "BAR")
        .env("OTHER", "VALUE")
        .spawn()
        .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(1));
    let pid = Pid::from_u32(p.id() as _);
    let mut s = System::new();

    s.refresh_process_specifics(pid, sysinfo::ProcessRefreshKind::everything());
    p.kill().expect("Unable to kill process.");

    let processes = s.processes();
    let proc_ = processes.get(&pid);

    if let Some(proc_) = proc_ {
        assert_eq!(proc_.pid(), pid);
        assert!(proc_.environ().iter().any(|e| e == "FOO=BAR"));
        assert!(proc_.environ().iter().any(|e| e == "OTHER=VALUE"));
    } else {
        panic!("Process not found!");
    }

    // Test to ensure that a process with a lot of environment variables doesn't get truncated.
    // More information in <https://github.com/GuillaumeGomez/sysinfo/issues/886>.
    const SIZE: usize = 30_000;
    let mut big_env = String::with_capacity(SIZE);
    for _ in 0..SIZE {
        big_env.push('a');
    }
    let mut p = std::process::Command::new("./target/test_binary")
        .env("FOO", &big_env)
        .spawn()
        .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(1));
    let pid = Pid::from_u32(p.id() as _);
    let mut s = System::new();

    s.refresh_processes_specifics(ProcessRefreshKind::new().with_environ(UpdateKind::Always));

    let processes = s.processes();
    let proc_ = processes.get(&pid);

    if let Some(proc_) = proc_ {
        p.kill().expect("Unable to kill process.");
        assert_eq!(proc_.pid(), pid);
        let env = format!("FOO={big_env}");
        assert!(proc_.environ().iter().any(|e| *e == env));
    } else {
        panic!("Process not found!");
    }
}

#[test]
fn test_process_refresh() {
    let mut s = System::new();
    assert_eq!(s.processes().len(), 0);

    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }
    s.refresh_process(sysinfo::get_current_pid().expect("failed to get current pid"));
    assert!(s
        .process(sysinfo::get_current_pid().expect("failed to get current pid"))
        .is_some());

    assert!(s
        .processes()
        .iter()
        .all(|(_, p)| p.environ().is_empty() && p.cwd().is_none() && p.cmd().is_empty()));
    assert!(s
        .processes()
        .iter()
        .any(|(_, p)| !p.name().is_empty() && p.memory() != 0));
}

#[test]
fn test_process_disk_usage() {
    use std::fs;
    use std::fs::File;
    use std::io::prelude::*;
    use sysinfo::get_current_pid;

    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }
    if std::env::var("FREEBSD_CI").is_ok() {
        // For an unknown reason, when running this test on Cirrus CI, it fails. It works perfectly
        // locally though... Dark magic...
        return;
    }

    fn inner() -> System {
        {
            let mut file = File::create("test.txt").expect("failed to create file");
            file.write_all(b"This is a test file\nwith test data.\n")
                .expect("failed to write to file");
        }
        fs::remove_file("test.txt").expect("failed to remove file");
        // Waiting a bit just in case...
        std::thread::sleep(std::time::Duration::from_millis(250));
        let mut system = System::new();
        assert!(system.processes().is_empty());
        system.refresh_processes();
        assert!(!system.processes().is_empty());
        system
    }

    let mut system = inner();
    let mut p = system
        .process(get_current_pid().expect("Failed retrieving current pid."))
        .expect("failed to get process");

    if cfg!(any(target_os = "macos", target_os = "ios")) && p.disk_usage().total_written_bytes == 0
    {
        // For whatever reason, sometimes, mac doesn't work on the first time when running
        // `cargo test`. Two solutions, either run with "cargo test -- --test-threads 1", or
        // check twice...
        system = inner();
        p = system
            .process(get_current_pid().expect("Failed retrieving current pid."))
            .expect("failed to get process");
    }

    assert!(
        p.disk_usage().total_written_bytes > 0,
        "found {} total written bytes...",
        p.disk_usage().total_written_bytes
    );
    assert!(
        p.disk_usage().written_bytes > 0,
        "found {} written bytes...",
        p.disk_usage().written_bytes
    );
}

#[test]
fn cpu_usage_is_not_nan() {
    let mut system = System::new();
    system.refresh_processes();

    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }

    // We need `collect` otherwise we can't have mutable access to `system`.
    #[allow(clippy::needless_collect)]
    let first_pids = system
        .processes()
        .iter()
        .take(10)
        .map(|(&pid, _)| pid)
        .collect::<Vec<_>>();
    let mut checked = 0;

    first_pids.into_iter().for_each(|pid| {
        system.refresh_process(pid);
        if let Some(p) = system.process(pid) {
            assert!(!p.cpu_usage().is_nan());
            checked += 1;
        }
    });
    assert!(checked > 0);
}

#[test]
fn test_process_times() {
    use std::time::{SystemTime, UNIX_EPOCH};

    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }
    let boot_time = System::boot_time();
    assert!(boot_time > 0);
    let mut p = if cfg!(target_os = "windows") {
        std::process::Command::new("waitfor")
            .arg("/t")
            .arg("3")
            .arg("ProcessTimes")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    } else {
        std::process::Command::new("sleep")
            .arg("3")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };

    let pid = Pid::from_u32(p.id() as _);
    std::thread::sleep(std::time::Duration::from_secs(1));
    let mut s = System::new();
    s.refresh_processes();
    p.kill().expect("Unable to kill process.");

    if let Some(p) = s.process(pid) {
        assert_eq!(p.pid(), pid);
        assert!(p.run_time() >= 1);
        assert!(p.run_time() <= 2);
        assert!(p.start_time() > p.run_time());
        // On linux, for whatever reason, the uptime seems to be older than the boot time, leading
        // to this weird `+ 5` to ensure the test is passing as it should...
        assert!(
            p.start_time() + 5
                > SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
        );
        assert!(p.start_time() >= boot_time);
    } else {
        panic!("Process not found!");
    }
}

// Checks that `session_id` is working.
#[test]
fn test_process_session_id() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }
    let mut s = System::new();
    s.refresh_processes();
    assert!(s.processes().values().any(|p| p.session_id().is_some()));
}

// Checks that `refresh_processes` is removing dead processes.
#[test]
fn test_refresh_processes() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }
    let mut p = if cfg!(target_os = "windows") {
        std::process::Command::new("waitfor")
            .arg("/t")
            .arg("300")
            .arg("RefreshProcesses")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    } else {
        std::process::Command::new("sleep")
            .arg("300")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };

    let pid = Pid::from_u32(p.id() as _);
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Checks that the process is listed as it should.
    let mut s = System::new();
    s.refresh_processes();
    assert!(s.process(pid).is_some());

    // Check that the process name is not empty.
    assert!(!s.process(pid).unwrap().name().is_empty());

    p.kill().expect("Unable to kill process.");
    // We need this, otherwise the process will still be around as a zombie on linux.
    let _ = p.wait();
    // Let's give some time to the system to clean up...
    std::thread::sleep(std::time::Duration::from_secs(1));

    s.refresh_processes();
    // Checks that the process isn't listed anymore.
    assert!(s.process(pid).is_none());
}

// Checks that `refresh_processes` is adding and removing task.
#[test]
#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    not(feature = "unknown-ci")
))]
fn test_refresh_tasks() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }
    let task_name = "task_1_second";
    std::thread::Builder::new()
        .name(task_name.into())
        .spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(1));
        })
        .unwrap();

    let pid = Pid::from_u32(std::process::id() as _);

    // Checks that the task is listed as it should.
    let mut s = System::new();
    s.refresh_processes();

    assert!(s
        .process(pid)
        .unwrap()
        .tasks()
        .map(|tasks| tasks.iter().any(|task_pid| s
            .process(*task_pid)
            .map(|task| task.name() == task_name)
            .unwrap_or(false)))
        .unwrap_or(false));
    assert!(s.processes_by_exact_name(task_name).next().is_some());

    // Let's give some time to the system to clean up...
    std::thread::sleep(std::time::Duration::from_secs(2));

    s.refresh_processes();

    assert!(!s
        .process(pid)
        .unwrap()
        .tasks()
        .map(|tasks| tasks.iter().any(|task_pid| s
            .process(*task_pid)
            .map(|task| task.name() == task_name)
            .unwrap_or(false)))
        .unwrap_or(false));
    assert!(s.processes_by_exact_name(task_name).next().is_none());
}

// Checks that `refresh_process` is NOT removing dead processes.
#[test]
fn test_refresh_process() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }
    let mut p = if cfg!(target_os = "windows") {
        std::process::Command::new("waitfor")
            .arg("/t")
            .arg("300")
            .arg("RefreshProcess")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    } else {
        std::process::Command::new("sleep")
            .arg("300")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };

    let pid = Pid::from_u32(p.id() as _);
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Checks that the process is listed as it should.
    let mut s = System::new();
    s.refresh_process(pid);
    assert!(s.process(pid).is_some());

    // Check that the process name is not empty.
    assert!(!s.process(pid).unwrap().name().is_empty());

    p.kill().expect("Unable to kill process.");
    // We need this, otherwise the process will still be around as a zombie on linux.
    let _ = p.wait();
    // Let's give some time to the system to clean up...
    std::thread::sleep(std::time::Duration::from_secs(1));

    assert!(!s.refresh_process(pid));
    // Checks that the process is still listed.
    assert!(s.process(pid).is_some());
}

#[test]
fn test_wait_child() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }
    let p = if cfg!(target_os = "windows") {
        std::process::Command::new("waitfor")
            .arg("/t")
            .arg("300")
            .arg("WaitChild")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    } else {
        std::process::Command::new("sleep")
            .arg("300")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };

    let before = std::time::Instant::now();
    let pid = Pid::from_u32(p.id() as _);

    let mut s = System::new();
    s.refresh_process(pid);
    let process = s.process(pid).unwrap();

    // Kill the child process.
    process.kill();
    // Wait for child process should work.
    process.wait();

    // Child process should not be present.
    assert!(!s.refresh_process(pid));
    assert!(before.elapsed() < std::time::Duration::from_millis(1000));
}

#[test]
fn test_wait_non_child() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }

    let before = std::time::Instant::now();

    // spawn non child process.
    let p = if !cfg!(target_os = "linux") {
        return;
    } else {
        std::process::Command::new("setsid")
            .arg("-w")
            .arg("sleep")
            .arg("2")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };
    let pid = Pid::from_u32(p.id());

    let mut s = System::new();
    s.refresh_process(pid);
    let process = s.process(pid).expect("Process not found!");

    // Wait for a non child process.
    process.wait();

    // Child process should not be present.
    assert!(!s.refresh_process(pid));

    // should wait for 2s.
    assert!(
        before.elapsed() > std::time::Duration::from_millis(1900),
        "Elapsed time {:?} is not greater than 1900ms",
        before.elapsed()
    );
    assert!(
        before.elapsed() < std::time::Duration::from_millis(3000),
        "Elapsed time {:?} is not less than 3000ms",
        before.elapsed()
    );
}

#[test]
fn test_process_iterator_lifetimes() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }

    let s = System::new_with_specifics(
        sysinfo::RefreshKind::new().with_processes(sysinfo::ProcessRefreshKind::new()),
    );

    let process: Option<&sysinfo::Process>;
    {
        let name = String::from("");
        // errors before PR #904: name does not live long enough
        process = s.processes_by_name(&name).next();
    }
    process.unwrap();

    let process: Option<&sysinfo::Process>;
    {
        // worked fine before and after: &'static str lives longer than System, error couldn't appear
        process = s.processes_by_name("").next();
    }
    process.unwrap();
}

// Regression test for <https://github.com/GuillaumeGomez/sysinfo/issues/918>.
#[test]
fn test_process_cpu_usage() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }

    let mut sys = System::new_all();
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_all();

    let max_usage = sys.cpus().len() as f32 * 100.;

    for process in sys.processes().values() {
        assert!(process.cpu_usage() <= max_usage);
    }
}

#[test]
fn test_process_creds() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }

    let mut sys = System::new_all();
    sys.refresh_all();

    // Just ensure there is at least one process on the system whose credentials can be retrieved.
    assert!(sys.processes().values().any(|process| {
        if process.user_id().is_none() {
            return false;
        }

        #[cfg(not(windows))]
        {
            if process.group_id().is_none()
                || process.effective_user_id().is_none()
                || process.effective_group_id().is_none()
            {
                return false;
            }
        }

        true
    }));

    // On Windows, make sure no process has real group ID and no effective IDs.
    #[cfg(windows)]
    assert!(sys.processes().values().all(|process| {
        if process.group_id().is_some()
            || process.effective_user_id().is_some()
            || process.effective_group_id().is_some()
        {
            return false;
        }

        true
    }));
}

// This test ensures that only the requested information is retrieved.
#[test]
fn test_process_specific_refresh() {
    use sysinfo::{DiskUsage, ProcessRefreshKind};

    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }

    fn check_empty(s: &System, pid: Pid) {
        let p = s.process(pid).unwrap();

        // Name should never be empty.
        assert!(!p.name().is_empty());
        if cfg!(target_os = "windows") {
            assert_eq!(p.user_id(), None);
        }
        assert_eq!(p.environ().len(), 0);
        assert_eq!(p.cmd().len(), 0);
        assert_eq!(p.exe(), None);
        assert_eq!(p.cwd(), None);
        assert_eq!(p.root(), None);
        assert_eq!(p.memory(), 0);
        assert_eq!(p.virtual_memory(), 0);
        // These two won't be checked, too much lazyness in testing them...
        assert_eq!(p.disk_usage(), DiskUsage::default());
        assert_eq!(p.cpu_usage(), 0.);
    }

    let mut s = System::new();
    let pid = Pid::from_u32(std::process::id());

    macro_rules! update_specific_and_check {
        (memory) => {
            s.refresh_process_specifics(pid, ProcessRefreshKind::new());
            {
                let p = s.process(pid).unwrap();
                assert_eq!(p.memory(), 0, "failed 0 check for memory");
                assert_eq!(p.virtual_memory(), 0, "failed 0 check for virtual memory");
            }
            s.refresh_process_specifics(pid, ProcessRefreshKind::new().with_memory());
            {
                let p = s.process(pid).unwrap();
                assert_ne!(p.memory(), 0, "failed non-0 check for memory");
                assert_ne!(p.virtual_memory(), 0, "failed non-0 check for virtual memory");
            }
            // And now we check that re-refreshing nothing won't remove the
            // information.
            s.refresh_process_specifics(pid, ProcessRefreshKind::new());
            {
                let p = s.process(pid).unwrap();
                assert_ne!(p.memory(), 0, "failed non-0 check (number 2) for memory");
                assert_ne!(p.virtual_memory(), 0, "failed non-0 check(number 2) for virtual memory");
            }
        };
        ($name:ident, $method:ident, $($extra:tt)+) => {
            s.refresh_process_specifics(pid, ProcessRefreshKind::new());
            {
                let p = s.process(pid).unwrap();
                assert_eq!(
                    p.$name()$($extra)+,
                    concat!("failed 0 check check for ", stringify!($name)),
                );
            }
            s.refresh_process_specifics(pid, ProcessRefreshKind::new().$method(UpdateKind::Always));
            {
                let p = s.process(pid).unwrap();
                assert_ne!(
                    p.$name()$($extra)+,
                    concat!("failed non-0 check check for ", stringify!($name)),);
            }
            // And now we check that re-refreshing nothing won't remove the
            // information.
            s.refresh_process_specifics(pid, ProcessRefreshKind::new());
            {
                let p = s.process(pid).unwrap();
                assert_ne!(
                    p.$name()$($extra)+,
                    concat!("failed non-0 check (number 2) check for ", stringify!($name)),);
            }
        }
    }

    s.refresh_process_specifics(pid, ProcessRefreshKind::new());
    check_empty(&s, pid);

    s.refresh_process_specifics(pid, ProcessRefreshKind::new());
    check_empty(&s, pid);

    update_specific_and_check!(memory);
    update_specific_and_check!(environ, with_environ, .len(), 0);
    update_specific_and_check!(cmd, with_cmd, .len(), 0);
    if !cfg!(any(
        target_os = "macos",
        target_os = "ios",
        feature = "apple-sandbox",
    )) {
        update_specific_and_check!(root, with_root, , None);
    }
    update_specific_and_check!(exe, with_exe, , None);
    update_specific_and_check!(cwd, with_cwd, , None);
}

#[test]
fn test_refresh_pids() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }
    let self_pid = sysinfo::get_current_pid().expect("failed to get current pid");
    let mut s = System::new();

    let mut p = if cfg!(target_os = "windows") {
        std::process::Command::new("waitfor")
            .arg("/t")
            .arg("3")
            .arg("RefreshPids")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    } else {
        std::process::Command::new("sleep")
            .arg("3")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };

    let child_pid = Pid::from_u32(p.id() as _);
    let pids = &[child_pid, self_pid];
    std::thread::sleep(std::time::Duration::from_millis(500));
    s.refresh_pids(pids);
    p.kill().expect("Unable to kill process.");

    assert_eq!(s.processes().len(), 2);
    for pid in s.processes().keys() {
        assert!(pids.contains(pid));
    }
}

#[test]
fn test_process_run_time() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") {
        return;
    }
    let mut s = System::new();
    let current_pid = sysinfo::get_current_pid().expect("failed to get current pid");
    s.refresh_process(current_pid);
    let run_time = s.process(current_pid).expect("no process found").run_time();
    std::thread::sleep(std::time::Duration::from_secs(2));
    s.refresh_process(current_pid);
    let new_run_time = s.process(current_pid).expect("no process found").run_time();
    assert!(
        new_run_time > run_time,
        "{} not superior to {}",
        new_run_time,
        run_time
    );
}

// Test that if the parent of a process is removed, then the child PID will be
// updated as well.
#[test]
fn test_parent_change() {
    if !sysinfo::IS_SUPPORTED_SYSTEM || cfg!(feature = "apple-sandbox") || cfg!(windows) {
        // Windows never updates its parent PID so no need to check anything.
        return;
    }

    let file_name = "target/test_binary2";
    build_test_binary(file_name);
    let mut p = std::process::Command::new(format!("./{file_name}"))
        .arg("1")
        .spawn()
        .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(1));

    let pid = Pid::from_u32(p.id() as _);
    let mut s = System::new();
    s.refresh_processes();

    assert_eq!(
        s.process(pid).expect("process was not created").parent(),
        sysinfo::get_current_pid().ok(),
    );

    let child_pid = s
        .processes()
        .iter()
        .find(|(_, proc_)| proc_.parent() == Some(pid))
        .map(|(pid, _)| *pid)
        .expect("failed to get child process");

    // Waiting for the parent process to stop.
    p.wait().expect("wait failed");

    s.refresh_processes();
    // Parent should not be around anymore.
    assert!(s.process(pid).is_none());

    let child = s.process(child_pid).expect("child is dead");
    // Child should have a different parent now.
    assert_ne!(child.parent(), Some(pid));

    // We kill the child to clean up.
    child.kill();
}
