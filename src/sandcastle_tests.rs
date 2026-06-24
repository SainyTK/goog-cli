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

#[test]
fn sandcastle_rust_workflow_verifier_runs_expected_commands() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let package_json = fs::read_to_string(repo_root.join("package.json")).unwrap();
    let verifier =
        fs::read_to_string(repo_root.join(".sandcastle/verify-rust-workflow.mts")).unwrap();
    let notes = fs::read_to_string(
        repo_root.join(".sandcastle/verify-rust-workflow-notes.md"),
    )
    .unwrap();

    assert_contains(
        &package_json,
        r#""sandcastle:verify-rust-workflow": "npx tsx .sandcastle/verify-rust-workflow.mts""#,
    );

    for expected in [
        "docker()",
        r#""npm install""#,
        r#"rustup --version"#,
        r#"cargo fmt"#,
        r#"npm run typecheck"#,
        r#"npm run test"#,
        "Runtime Rust installer commands are intentionally forbidden",
    ] {
        assert_contains(&verifier, expected);
    }

    for forbidden in ["rustup.rs", "curl --proto", "sh -s --"] {
        assert_not_contains(&verifier, forbidden);
    }

    assert_ordered(
        &verifier,
        &[
            r#""npm install""#,
            r#"rustup --version"#,
            r#"cargo fmt"#,
            r#"npm run typecheck"#,
            r#"npm run test"#,
        ],
    );

    assert_contains(&notes, "npm run sandcastle:verify-rust-workflow");
    assert_contains(&notes, "cargo fmt");
    assert_contains(&notes, "npm run typecheck");
    assert_contains(&notes, "npm run test");
}

fn assert_contains(contents: &str, expected: &str) {
    assert!(
        contents.contains(expected),
        "expected file contents to include {expected:?}"
    );
}

fn assert_not_contains(contents: &str, forbidden: &str) {
    assert!(
        !contents.contains(forbidden),
        "expected file contents not to include {forbidden:?}"
    );
}

fn assert_ordered(contents: &str, expected: &[&str]) {
    let mut cursor = 0;

    for item in expected {
        let Some(offset) = contents[cursor..].find(item) else {
            panic!("expected file contents after byte {cursor} to include {item:?}");
        };
        cursor += offset + item.len();
    }
}
