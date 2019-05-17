fn main() {
    if std::env::var("TARGET").unwrap().contains("-apple") {
        println!("cargo:rustc-link-lib=framework=IOKit");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }
}
