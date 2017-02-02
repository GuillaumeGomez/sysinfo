#[cfg(target_os = "macos")]
fn main() {
    println!("cargo:rustc-link-lib=framework=IOKit");
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
}

#[cfg(not(target_os = "macos"))]
fn main() {}
