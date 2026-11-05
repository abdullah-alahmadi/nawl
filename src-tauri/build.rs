fn main() {
    // Lets debug builds find the sidecar at binaries/tectonic-<triple>.
    println!(
        "cargo:rustc-env=TARGET_TRIPLE={}",
        std::env::var("TARGET").expect("TARGET is set by cargo")
    );
    tauri_build::build()
}
