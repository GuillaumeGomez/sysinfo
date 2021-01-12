#[cfg(target_os = "macos")]
extern crate cc;

#[cfg(target_os = "macos")]
fn main() {
    let is_ios = std::env::var("CARGO_CFG_TARGET_OS")
        .map(|s| s == "ios")
        .unwrap_or(false);

    if !is_ios {
        cc::Build::new()
            .file("src/apple/macos/disk.m")
            .compile("disk");
        // DiskArbitration is not available on iOS: https://developer.apple.com/documentation/diskarbitration
        println!("cargo:rustc-link-lib=framework=DiskArbitration");
        // IOKit is not available on iOS: https://developer.apple.com/documentation/iokit
        println!("cargo:rustc-link-lib=framework=IOKit");
    }

    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
}

#[cfg(not(target_os = "macos"))]
fn main() {}
