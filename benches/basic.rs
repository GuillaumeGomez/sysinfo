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
fn bench_refresh(b: &mut test::Bencher) {
    let mut s = sysinfo::System::new();

    b.iter(move || {
        s.refresh_all();
    });
}
