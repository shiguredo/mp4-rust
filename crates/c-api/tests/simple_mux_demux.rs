use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_simple_mux_demux() {
    let project_root = get_project_root();
    let lib_path = project_root.join("target/debug/libmp4.a");

    // ライブラリファイルが存在することを確認
    assert!(
        lib_path.exists(),
        "libmp4.a not found at {}. Run `cargo build` first.",
        lib_path.display()
    );

    let c_file = project_root.join("crates/c-api/tests/simple_mux_demux.c");
    assert!(
        c_file.exists(),
        "simple_mux_demux.c not found at {}",
        c_file.display()
    );

    let output_path = project_root.join("target/debug").join("simple_mux_demux");

    // C ファイルをコンパイル
    let status = Command::new("cc")
        .arg(&c_file)
        .arg("-o")
        .arg(&output_path)
        .arg(&lib_path)
        .arg("-I")
        .arg(project_root.join("crates/c-api/include"))
        .status()
        .expect("Failed to compile simple_mux_demux.c");

    assert!(
        status.success(),
        "Compilation failed for simple_mux_demux.c"
    );

    // コンパイルされた実行ファイルを実行
    let status = Command::new(&output_path)
        .status()
        .expect("Failed to execute simple_mux_demux");

    assert!(status.success(), "simple_mux_demux execution failed");
}

fn get_project_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find project root")
        .to_path_buf()
}
