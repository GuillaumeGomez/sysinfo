#![feature(test)]

extern crate test;
extern crate sysinfo;

use sysinfo::SystemExt;

#[bench]
fn bench_new(b: &mut test::Bencher) {
    b.iter(|| {
        sysinfo::System::new();
    });
}

#[bench]
fn bench_refresh_all(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new();

    b.iter(move || {
        s.refresh_all();
    });
}

#[bench]
fn bench_refresh_system(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new();

    s.refresh_system();
    b.iter(move || {
        s.refresh_system();
    });
}

#[bench]
fn bench_refresh_processes(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new();

    b.iter(move || {
        s.refresh_processes();
    });
}

#[bench]
fn bench_refresh_process(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new();

    s.refresh_all();
    let pid = *s.get_process_list().iter().take(1).next().unwrap().0;
    b.iter(move || {
        s.refresh_process(pid);
    });
}

#[bench]
fn bench_refresh_disks(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new();

    b.iter(move || {
        s.refresh_disks();
    });
}

#[bench]
fn bench_refresh_disk_lists(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new();

    b.iter(move || {
        s.refresh_disk_list();
    });
}

#[bench]
fn bench_refresh_network(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new();

    b.iter(move || {
        s.refresh_network();
    });
}
