//
// Sysinfo
//
// Copyright (c) 2015 Guillaume Gomez
//

// Once https://github.com/rust-lang/rfcs/blob/master/text/1422-pub-restricted.md
// feature gets stabilized, we can move common parts in here.

#[cfg(test)]
mod tests {
    use crate::{utils, ProcessExt, System, SystemExt};

    #[test]
    fn test_refresh_system() {
        let mut sys = System::new();
        sys.refresh_system();
        // We don't want to test on unsupported systems.
        if System::IS_SUPPORTED {
            assert!(sys.get_total_memory() != 0);
            assert!(sys.get_free_memory() != 0);
        }
        assert!(sys.get_total_memory() >= sys.get_free_memory());
        assert!(sys.get_total_swap() >= sys.get_free_swap());
    }

    #[test]
    fn test_refresh_process() {
        let mut sys = System::new();
        assert!(
            sys.get_processes().is_empty(),
            "no process should be listed!"
        );
        sys.refresh_processes();
        // We don't want to test on unsupported systems.
        if System::IS_SUPPORTED {
            assert!(
                sys.refresh_process(utils::get_current_pid().expect("failed to get current pid")),
                "process not listed",
            );
        }
    }

    #[test]
    fn test_get_process() {
        let mut sys = System::new();
        sys.refresh_processes();
        if let Some(p) =
            sys.get_process(utils::get_current_pid().expect("failed to get current pid"))
        {
            assert!(p.memory() > 0);
        } else {
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
        if let Some(p) =
            sys.get_process(utils::get_current_pid().expect("failed to get current pid"))
        {
            p.foo(); // If this doesn't compile, it'll simply mean that the Process type
                     // doesn't implement the Send trait.
            p.bar(); // If this doesn't compile, it'll simply mean that the Process type
                     // doesn't implement the Sync trait.
        } else {
            assert!(!System::IS_SUPPORTED);
        }
    }

    #[test]
    fn check_hostname_has_no_nuls() {
        let sys = System::new();

        if let Some(hostname) = sys.get_host_name() {
            assert!(!hostname.contains('\u{0}'))
        }
    }

    #[test]
    fn check_uptime() {
        let sys = System::new();
        let uptime = sys.get_uptime();
        if System::IS_SUPPORTED {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            let new_uptime = sys.get_uptime();
            assert!(uptime < new_uptime);
        }
    }
}
