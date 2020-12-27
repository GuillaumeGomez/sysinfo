#[cfg(any(target_os = "macos", target_os = "ios"))]
extern crate cc;

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn main() {
    cc::Build::new().file("src/mac/disk.m").compile("disk");
    println!("cargo:rustc-link-lib=framework=IOKit");
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=DiskArbitration");
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn main() {}
