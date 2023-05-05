// Take a look at the license at the top of the repository in the LICENSE file.

use sysinfo::{Pid, PidExt, ProcessExt, SystemExt};

#[test]
fn test_process() {
    let mut s = sysinfo::System::new();
    assert_eq!(s.processes().len(), 0);
    s.refresh_processes();
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }
    assert!(!s.processes().is_empty());
    assert!(s
        .processes()
        .values()
        .any(|p| !p.exe().to_str().unwrap_or("").is_empty()));
}

#[test]
fn test_cwd() {
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
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
    let mut s = sysinfo::System::new();
    s.refresh_processes();
    p.kill().expect("Unable to kill process.");

    let processes = s.processes();
    let p = processes.get(&pid);

    if let Some(p) = p {
        assert_eq!(p.pid(), pid);
        assert_eq!(p.cwd(), std::env::current_dir().unwrap());
    } else {
        panic!("Process not found!");
    }
}

#[test]
fn test_cmd() {
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
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
    let mut s = sysinfo::System::new();
    assert!(s.processes().is_empty());
    s.refresh_processes();
    p.kill().expect("Unable to kill process.");
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

#[test]
fn test_environ() {
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }
    let mut p = if cfg!(target_os = "windows") {
        std::process::Command::new("waitfor")
            .arg("/t")
            .arg("3")
            .arg("EnvironSignal")
            .stdout(std::process::Stdio::null())
            .env("FOO", "BAR")
            .env("OTHER", "VALUE")
            .spawn()
            .unwrap()
    } else {
        std::process::Command::new("sleep")
            .arg("3")
            .stdout(std::process::Stdio::null())
            .env("FOO", "BAR")
            .env("OTHER", "VALUE")
            .spawn()
            .unwrap()
    };

    let pid = Pid::from_u32(p.id() as _);
    std::thread::sleep(std::time::Duration::from_secs(1));
    let mut s = sysinfo::System::new();
    s.refresh_processes();
    p.kill().expect("Unable to kill process.");

    let processes = s.processes();
    let p = processes.get(&pid);

    if let Some(p) = p {
        assert_eq!(p.pid(), pid);
        // FIXME: instead of ignoring the test on CI, try to find out what's wrong...
        if std::env::var("APPLE_CI").is_err() {
            assert!(p.environ().iter().any(|e| e == "FOO=BAR"));
            assert!(p.environ().iter().any(|e| e == "OTHER=VALUE"));
        }
    } else {
        panic!("Process not found!");
    }
}

// Test to ensure that a process with a lot of environment variables doesn't get truncated.
// More information in <https://github.com/GuillaumeGomez/sysinfo/issues/886>.
#[test]
fn test_big_environ() {
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }
    const SIZE: usize = 30_000;
    let mut big_env = String::with_capacity(SIZE);
    for _ in 0..SIZE {
        big_env.push('a');
    }
    let mut p = if cfg!(target_os = "windows") {
        std::process::Command::new("waitfor")
            .arg("/t")
            .arg("3")
            .arg("EnvironSignal")
            .stdout(std::process::Stdio::null())
            .env("FOO", &big_env)
            .spawn()
            .unwrap()
    } else {
        std::process::Command::new("sleep")
            .arg("3")
            .stdout(std::process::Stdio::null())
            .env("FOO", &big_env)
            .spawn()
            .unwrap()
    };

    let pid = Pid::from_u32(p.id() as _);
    std::thread::sleep(std::time::Duration::from_secs(1));
    let mut s = sysinfo::System::new();
    s.refresh_processes();
    p.kill().expect("Unable to kill process.");

    let processes = s.processes();
    let p = processes.get(&pid);

    if let Some(p) = p {
        assert_eq!(p.pid(), pid);
        // FIXME: instead of ignoring the test on CI, try to find out what's wrong...
        if std::env::var("APPLE_CI").is_err() {
            let env = format!("FOO={big_env}");
            assert!(p.environ().iter().any(|e| *e == env));
        }
    } else {
        panic!("Process not found!");
    }
}

#[test]
fn test_process_refresh() {
    let mut s = sysinfo::System::new();
    assert_eq!(s.processes().len(), 0);

    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }
    s.refresh_process(sysinfo::get_current_pid().expect("failed to get current pid"));
    assert!(s
        .process(sysinfo::get_current_pid().expect("failed to get current pid"))
        .is_some(),);
}

#[test]
fn test_process_disk_usage() {
    use std::fs;
    use std::fs::File;
    use std::io::prelude::*;
    use sysinfo::{get_current_pid, ProcessExt, SystemExt};

    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }
    if std::env::var("FREEBSD_CI").is_ok() {
        // For an unknown reason, when running this test on Cirrus CI, it fails. It works perfectly
        // locally though... Dark magic...
        return;
    }

    fn inner() -> sysinfo::System {
        {
            let mut file = File::create("test.txt").expect("failed to create file");
            file.write_all(b"This is a test file\nwith test data.\n")
                .expect("failed to write to file");
        }
        fs::remove_file("test.txt").expect("failed to remove file");
        // Waiting a bit just in case...
        std::thread::sleep(std::time::Duration::from_millis(250));
        let mut system = sysinfo::System::new();
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
    let mut system = sysinfo::System::new();
    system.refresh_processes();

    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
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

    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }
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
    let mut s = sysinfo::System::new();
    s.refresh_processes();
    p.kill().expect("Unable to kill process.");

    if let Some(p) = s.process(pid) {
        assert_eq!(p.pid(), pid);
        assert!(p.run_time() >= 1);
        assert!(p.run_time() <= 2);
        assert!(p.start_time() > p.run_time());
        // On linux, for whatever reason, the uptime seems to be older than the boot time, leading
        // to this weird `+ 3` to ensure the test is passing as it should...
        assert!(
            p.start_time() + 3
                > SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
        );
        assert!(p.start_time() >= s.boot_time());
    } else {
        panic!("Process not found!");
    }
}

// Checks that `session_id` is working.
#[test]
fn test_process_session_id() {
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }
    let mut s = sysinfo::System::new();
    s.refresh_processes();
    assert!(s.processes().values().any(|p| p.session_id().is_some()));
}

// Checks that `refresh_processes` is removing dead processes.
#[test]
fn test_refresh_processes() {
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
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
    let mut s = sysinfo::System::new();
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
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
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
    let mut s = sysinfo::System::new();
    s.refresh_processes();

    assert!(s
        .process(pid)
        .unwrap()
        .tasks
        .values()
        .any(|t| t.name() == task_name));

    // Let's give some time to the system to clean up...
    std::thread::sleep(std::time::Duration::from_secs(2));

    s.refresh_processes();

    assert!(!s
        .process(pid)
        .unwrap()
        .tasks
        .values()
        .any(|t| t.name() == task_name));
}

// Checks that `refresh_process` is NOT removing dead processes.
#[test]
fn test_refresh_process() {
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
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
    let mut s = sysinfo::System::new();
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
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }
    let p = if cfg!(target_os = "windows") {
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

    let before = std::time::Instant::now();
    let pid = Pid::from_u32(p.id() as _);

    let mut s = sysinfo::System::new();
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
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
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

    let mut s = sysinfo::System::new();
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
    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }

    let s = sysinfo::System::new_with_specifics(
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
    use sysinfo::{ProcessExt, System, SystemExt};

    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
        return;
    }

    let mut sys = System::new_all();
    std::thread::sleep(System::MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_all();

    let max_usage = sys.cpus().len() as f32 * 100.;

    for process in sys.processes().values() {
        assert!(process.cpu_usage() <= max_usage);
    }
}

#[test]
fn test_process_creds() {
    use sysinfo::{ProcessExt, System, SystemExt};

    if !sysinfo::System::IS_SUPPORTED || cfg!(feature = "apple-sandbox") {
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
