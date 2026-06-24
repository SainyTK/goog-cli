use std::fs;
use std::path::Path;

#[test]
fn sandbox_image_installs_stable_rust_for_agent_shells() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dockerfile = fs::read_to_string(repo_root.join(".sandcastle/Dockerfile")).unwrap();
    let package_json = fs::read_to_string(repo_root.join("package.json")).unwrap();

    for expected in [r#""typecheck": "cargo check""#, r#""test": "cargo test""#] {
        assert_contains(&package_json, expected);
    }

    for expected in [
        "rustup.rs",
        "--default-toolchain stable",
        "rustfmt",
        "CARGO_HOME=/home/agent/.cargo",
        "PATH=/home/agent/.cargo/bin:",
        "cargo --version",
        "rustc --version",
        "rustup --version",
        "rustfmt --version",
        "package.json scripts run Cargo",
    ] {
        assert_contains(&dockerfile, expected);
    }
}

fn assert_contains(contents: &str, expected: &str) {
    assert!(
        contents.contains(expected),
        "expected file contents to include {expected:?}"
    );
}
