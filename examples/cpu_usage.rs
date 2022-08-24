use std::thread;
use std::time::Duration;

use sysinfo::{CpuExt, CpuRefreshKind, RefreshKind, System, SystemExt};

fn main() {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_cpu(CpuRefreshKind::new().with_cpu_usage()),
    );
    loop {
        thread::sleep(Duration::from_secs(1));
        system.refresh_cpu();
        println!("{:>7.3} %", system.global_cpu_info().cpu_usage());
    }
}
