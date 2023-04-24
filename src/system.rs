// Take a look at the license at the top of the repository in the LICENSE file.

// Once https://github.com/rust-lang/rfcs/blob/master/text/1422-pub-restricted.md
// feature gets stabilized, we can move common parts in here.

#[cfg(test)]
mod tests {
    use crate::{ProcessExt, System, SystemExt};

    #[test]
    fn test_refresh_system() {
        let mut sys = System::new();
        sys.refresh_system();
        // We don't want to test on unsupported systems.
        if System::IS_SUPPORTED {
            assert!(sys.total_memory() != 0);
            assert!(sys.free_memory() != 0);
        }
        assert!(sys.total_memory() >= sys.free_memory());
        assert!(sys.total_swap() >= sys.free_swap());
    }

    #[test]
    fn test_refresh_process() {
        let mut sys = System::new();
        assert!(sys.processes().is_empty(), "no process should be listed!");
        // We don't want to test on unsupported systems.

        #[cfg(not(feature = "apple-sandbox"))]
        if System::IS_SUPPORTED {
            assert!(
                sys.refresh_process(crate::get_current_pid().expect("failed to get current pid")),
                "process not listed",
            );
            // Ensure that the process was really added to the list!
            assert!(sys
                .process(crate::get_current_pid().expect("failed to get current pid"))
                .is_some());
        }
    }

    #[test]
    fn test_get_process() {
        let mut sys = System::new();
        sys.refresh_processes();
        let current_pid = match crate::get_current_pid() {
            Ok(pid) => pid,
            _ => {
                if !System::IS_SUPPORTED {
                    return;
                }
                panic!("get_current_pid should work!");
            }
        };
        if let Some(p) = sys.process(current_pid) {
            assert!(p.memory() > 0);
        } else {
            #[cfg(not(feature = "apple-sandbox"))]
            assert!(!System::IS_SUPPORTED);
        }
    }

    #[test]
    fn check_if_send_and_sync() {
        trait Foo {
            fn foo(&self) {}
        }
        impl<T> Foo for T where T: Send {}

        trait Bar {
            fn bar(&self) {}
        }

        impl<T> Bar for T where T: Sync {}

        let mut sys = System::new();
        sys.refresh_processes();
        let current_pid = match crate::get_current_pid() {
            Ok(pid) => pid,
            _ => {
                if !System::IS_SUPPORTED {
                    return;
                }
                panic!("get_current_pid should work!");
            }
        };
        if let Some(p) = sys.process(current_pid) {
            p.foo(); // If this doesn't compile, it'll simply mean that the Process type
                     // doesn't implement the Send trait.
            p.bar(); // If this doesn't compile, it'll simply mean that the Process type
                     // doesn't implement the Sync trait.
        } else {
            #[cfg(not(feature = "apple-sandbox"))]
            assert!(!System::IS_SUPPORTED);
        }
    }

    #[test]
    fn check_hostname_has_no_nuls() {
        let sys = System::new();

        if let Some(hostname) = sys.host_name() {
            assert!(!hostname.contains('\u{0}'))
        }
    }

    #[test]
    fn check_uptime() {
        let sys = System::new();
        let uptime = sys.uptime();
        if System::IS_SUPPORTED {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            let new_uptime = sys.uptime();
            assert!(uptime < new_uptime);
        }
    }

    // This test is used to ensure that the CPU usage computation isn't completely going off
    // when refreshing it too frequently (ie, multiple times in a row in a very small interval).
    #[test]
    #[ignore] // This test MUST be run on its own to prevent wrong CPU usage measurements.
    fn test_consecutive_cpu_usage_update() {
        use crate::{PidExt, ProcessExt, ProcessRefreshKind, System, SystemExt};
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        if !System::IS_SUPPORTED {
            return;
        }

        let mut sys = System::new_all();
        assert!(!sys.cpus().is_empty());
        sys.refresh_processes_specifics(ProcessRefreshKind::new().with_cpu());

        let stop = Arc::new(AtomicBool::new(false));
        // Spawning a few threads to ensure that it will actually have an impact on the CPU usage.
        for it in 0..sys.cpus().len() / 2 + 1 {
            let stop_c = Arc::clone(&stop);
            std::thread::spawn(move || {
                while !stop_c.load(Ordering::Relaxed) {
                    if it != 0 {
                        // The first thread runs at 100% to be sure it'll be noticeable.
                        std::thread::sleep(Duration::from_millis(1));
                    }
                }
            });
        }

        let mut pids = sys
            .processes()
            .iter()
            .map(|(pid, _)| *pid)
            .take(2)
            .collect::<Vec<_>>();
        let pid = std::process::id();
        pids.push(PidExt::from_u32(pid));
        assert_eq!(pids.len(), 3);

        for it in 0..3 {
            std::thread::sleep(
                crate::System::MINIMUM_CPU_UPDATE_INTERVAL + Duration::from_millis(1),
            );
            for pid in &pids {
                sys.refresh_process_specifics(*pid, ProcessRefreshKind::new().with_cpu());
            }
            // To ensure that Linux doesn't give too high numbers.
            assert!(
                sys.process(pids[2]).unwrap().cpu_usage() < sys.cpus().len() as f32 * 100.,
                "using ALL CPU: failed at iteration {}",
                it
            );
            // To ensure it's not 0 either.
            assert!(
                sys.process(pids[2]).unwrap().cpu_usage() > 0.,
                "using NO CPU: failed at iteration {}",
                it
            );
        }
        stop.store(false, Ordering::Relaxed);
    }
}
