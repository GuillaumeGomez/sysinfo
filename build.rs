fn main() {
    let is_apple = std::env::var("TARGET")
        .map(|t| t.contains("-apple"))
        .unwrap_or(false);
    let is_ios = std::env::var("CARGO_CFG_TARGET_OS")
        .map(|s| s == "ios")
        .unwrap_or(false);

    if is_apple {
        if !is_ios {
            // DiskArbitration is not available on iOS: https://developer.apple.com/documentation/diskarbitration
            println!("cargo:rustc-link-lib=framework=DiskArbitration");
            // IOKit is not available on iOS: https://developer.apple.com/documentation/iokit
            println!("cargo:rustc-link-lib=framework=IOKit");
        }

        println!("cargo:rustc-link-lib=framework=Foundation");
    }
}
