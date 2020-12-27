#[cfg(any(target_os = "macos", target_os = "ios"))]
extern crate cc;

fn main() {
    if std::env::var("TARGET").unwrap().contains("-apple") {
        cc::Build::new().file("src/mac/disk.m").compile("disk");
        println!("cargo:rustc-link-lib=framework=IOKit");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=DiskArbitration");
    }
}
