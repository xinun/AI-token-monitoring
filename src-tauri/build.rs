fn main() {
    tauri_build::build();
    #[cfg(windows)]
    {
        // Integration tests do not receive the application's embedded resources.
        // Tauri's menu imports need Common Controls v6 even in the mock runtime.
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/windows-test.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}", manifest.display());
    }
}
