// 
// Sysinfo
// 
// Copyright (c) 2015 Guillaume Gomez
//

// Once https://github.com/rust-lang/rfcs/blob/master/text/1422-pub-restricted.md
// feature gets stabilized, we can move common parts in here.

#[cfg(test)]
mod tests {
    use ::{ProcessExt, System, SystemExt};
    use ::utils;

    #[test]
    fn test_refresh_system() {
        let mut sys = System::new();
        sys.refresh_system();
        assert!(sys.get_total_memory() != 0);
        assert!(sys.get_free_memory() != 0);
        assert!(sys.get_total_memory() >= sys.get_free_memory());
        assert!(sys.get_total_swap() >= sys.get_free_swap());
    }

    #[test]
    fn test_refresh_process() {
        let mut sys = System::new();
        assert!(sys.refresh_process(utils::get_current_pid()));
    }

    #[test]
    fn test_get_process() {
        let mut sys = System::new();
        sys.refresh_processes();
        let p = sys.get_process(utils::get_current_pid()).expect("didn't find process");
        assert!(p.memory() > 0);
    }
}
