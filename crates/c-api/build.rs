fn main() {
    cbindgen::Builder::new()
        .with_crate(env!("CARGO_MANIFEST_DIR"))
        .with_language(cbindgen::Language::C)
        .with_include_version(true)
        .with_include_guard("SHIGUREDO_MP4_H")
        .with_no_includes()
        .with_sys_include("stdbool.h")
        .with_sys_include("stdint.h")
        .exclude_item("Option_CString")
        .exclude_item("Option_Mp4FileMuxer")
        .exclude_item("Vec_Output")
        .generate()
        .expect("Failed to generate C bindings")
        .write_to_file("include/mp4.h");
}
