#![feature(test)]

extern crate test;

use sysinfo::get_current_pid;
use sysinfo::{DiskExt, SystemExt};

#[bench]
fn bench_new(b: &mut test::Bencher) {
    b.iter(|| {
        sysinfo::System::new();
    });
}

#[bench]
fn bench_new_all(b: &mut test::Bencher) {
    b.iter(|| {
        sysinfo::System::new_all();
    });
}

#[bench]
fn bench_refresh_all(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new_all();

    b.iter(move || {
        s.refresh_all();
    });
}

#[bench]
fn bench_refresh_system(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new_all();

    s.refresh_system();
    b.iter(move || {
        s.refresh_system();
    });
}

#[bench]
fn bench_refresh_processes(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new();

    s.refresh_processes(); // to load the whole processes list a first time.
    b.iter(move || {
        s.refresh_processes();
    });
}

#[bench]
fn bench_first_refresh_processes(b: &mut test::Bencher) {
    b.iter(move || {
        let mut s = sysinfo::System::new();
        s.refresh_processes();
    });
}

#[bench]
fn bench_refresh_process(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new();

    s.refresh_all();
    // to be sure it'll exist for at least as long as we run
    let pid = get_current_pid().expect("failed to get current pid");
    b.iter(move || {
        s.refresh_process(pid);
    });
}

#[bench]
fn bench_refresh_disk(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new_all();

    let disks = s.disks_mut();
    let disk = &mut disks[0];
    b.iter(move || {
        disk.refresh();
    });
}

#[bench]
fn bench_refresh_disks(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new_all();

    b.iter(move || {
        s.refresh_disks();
    });
}

#[bench]
fn bench_refresh_disks_list(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new();

    b.iter(move || {
        s.refresh_disks_list();
    });
}

#[bench]
fn bench_refresh_networks(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new_all();

    b.iter(move || {
        s.refresh_networks();
    });
}

#[bench]
fn bench_refresh_networks_list(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new();

    b.iter(move || {
        s.refresh_networks_list();
    });
}

#[bench]
fn bench_refresh_memory(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new();

    b.iter(move || {
        s.refresh_memory();
    });
}

#[bench]
fn bench_refresh_cpu(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new();

    b.iter(move || {
        s.refresh_cpu();
    });
}

#[bench]
fn bench_refresh_components(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new_all();

    b.iter(move || {
        s.refresh_components();
    });
}

#[bench]
fn bench_refresh_components_list(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new_all();

    b.iter(move || {
        s.refresh_components_list();
    });
}

#[bench]
fn bench_refresh_users_list(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new_all();

    b.iter(move || {
        s.refresh_users_list();
    });
}
