fn main() {
    let crate_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR env var not set");
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(cbindgen::Language::C)
        .with_include_version(true)
        .with_include_guard("SHIGUREDO_MP4")
        .with_no_includes()
        .with_sys_include("stdbool")
        .with_sys_include("stdint")
        .generate()
        .expect("Failed to generate C bindings")
        .write_to_file("include/mp4.h");
}
