use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_c_examples_compile() {
    let project_root = get_project_root();
    let examples_dir = get_examples_dir();
    let lib_path = project_root.join("target/debug/libmp4.a");

    // ライブラリファイルが存在することを確認
    assert!(
        lib_path.exists(),
        "libmp4.a not found at {}. Run `cargo build` first.",
        lib_path.display()
    );

    // examples ディレクトリから全ての .c ファイルを検索
    let c_files: Vec<_> = std::fs::read_dir(&examples_dir)
        .expect("Failed to read examples directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "c") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    assert!(
        !c_files.is_empty(),
        "No .c files found in examples directory"
    );

    // 各 C ファイルをコンパイルする
    for c_file in c_files {
        let example_name = c_file
            .file_stem()
            .expect("Failed to get file stem")
            .to_string_lossy();
        let output_path = project_root
            .join("target/debug")
            .join(format!("{}", example_name));

        // C コンパイラでビルド
        let status = Command::new("cc")
            .arg(&c_file)
            .arg("-o")
            .arg(&output_path)
            .arg(&lib_path)
            .arg("-I")
            .arg(project_root.join("crates/c-api/include"))
            .status()
            .expect(&format!("Failed to compile example: {}", c_file.display()));

        assert!(
            status.success(),
            "Compilation failed for example: {}",
            example_name
        );
    }
}

fn get_project_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find project root")
        .to_path_buf()
}

fn get_examples_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("examples")
}
