use std::fs;

#[test]
fn sandbox_image_installs_stable_rust_for_agent_shells() {
    let dockerfile = fs::read_to_string(".sandcastle/Dockerfile").unwrap();
    let package_json = fs::read_to_string("package.json").unwrap();

    assert!(package_json.contains(r#""typecheck": "cargo check""#));
    assert!(package_json.contains(r#""test": "cargo test""#));

    assert!(dockerfile.contains("rustup.rs"));
    assert!(dockerfile.contains("--default-toolchain stable"));
    assert!(dockerfile.contains("rustfmt"));
    assert!(dockerfile.contains("CARGO_HOME=/home/agent/.cargo"));
    assert!(dockerfile.contains("PATH=/home/agent/.cargo/bin:"));
    assert!(dockerfile.contains("cargo --version"));
    assert!(dockerfile.contains("rustc --version"));
    assert!(dockerfile.contains("rustup --version"));
    assert!(dockerfile.contains("rustfmt --version"));
    assert!(dockerfile.contains("package.json scripts run Cargo"));
}
