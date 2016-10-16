#[cfg(target_os = "macos")]
fn main() {
    println!("cargo:rustc-link-lib=framework=IOKit");
}

#[cfg(not(target_os = "macos"))]
fn main() {}
