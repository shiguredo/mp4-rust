fn main() {
    let version = get_root_version();
    println!("cargo::rustc-env=SHIGUREDO_MP4_VERSION={version}");
}

fn get_root_version() -> String {
    let root_cargo_toml = include_str!("../../Cargo.toml");
    root_cargo_toml
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("version") {
                trimmed
                    .split_once('=')
                    .map(|(_, v)| v.trim().trim_matches('"'))
            } else {
                None
            }
        })
        .expect("version not found")
        .to_string()
}
