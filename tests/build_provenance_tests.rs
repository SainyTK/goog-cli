use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const BUILD_ENVIRONMENT: [&str; 4] = [
    "GOOG_BUILD_GIT_COMMIT",
    "GOOG_BUILD_GIT_DIRTY",
    "GOOG_BUILD_GIT_DISTANCE",
    "GOOG_BUILD_SOURCE_TAG",
];

#[test]
fn build_metadata_distinguishes_release_development_and_archive_states() {
    let temporary = tempfile::tempdir().unwrap();
    let build_script = temporary.path().join("build-script");
    let compilation = Command::new("rustc")
        .args(["--edition", "2021", "build.rs", "-o"])
        .arg(&build_script)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_success(&compilation, "compile build.rs");

    let checkout = temporary.path().join("checkout");
    fs::create_dir(&checkout).unwrap();
    git(&checkout, &["init"]);
    git(&checkout, &["config", "user.name", "Provenance Test"]);
    git(
        &checkout,
        &["config", "user.email", "provenance@example.invalid"],
    );
    fs::write(checkout.join("tracked.txt"), "tagged\n").unwrap();
    git(&checkout, &["add", "tracked.txt"]);
    git(&checkout, &["commit", "-m", "tagged release"]);
    git(&checkout, &["tag", "v0.2.5"]);

    let tagged = metadata(&build_script, &checkout, "0.2.5", &[]);
    assert_eq!(tagged["GOOG_DISPLAY_VERSION"], "0.2.5");
    assert_eq!(tagged["GOOG_RELEASE_CHANNEL"], "stable");
    assert_eq!(tagged["GOOG_GIT_DISTANCE"], "0");
    assert_eq!(tagged["GOOG_GIT_DIRTY"], "false");

    fs::write(checkout.join("tracked.txt"), "one commit ahead\n").unwrap();
    git(&checkout, &["add", "tracked.txt"]);
    git(&checkout, &["commit", "-m", "development commit"]);
    let ahead = metadata(&build_script, &checkout, "0.2.5", &[]);
    let short_commit = &ahead["GOOG_GIT_COMMIT"][..7];
    assert_eq!(
        ahead["GOOG_DISPLAY_VERSION"],
        format!("0.2.5-dev.1+{short_commit}")
    );
    assert_eq!(ahead["GOOG_RELEASE_CHANNEL"], "development");
    assert_eq!(ahead["GOOG_GIT_DISTANCE"], "1");

    fs::write(checkout.join("tracked.txt"), "dirty tracked change\n").unwrap();
    let dirty = metadata(&build_script, &checkout, "0.2.5", &[]);
    assert_eq!(dirty["GOOG_GIT_DIRTY"], "true");
    assert!(dirty["GOOG_DISPLAY_VERSION"].ends_with(".dirty"));

    git(&checkout, &["restore", "tracked.txt"]);
    fs::write(
        checkout.join("untracked-build-output"),
        "ignored by policy\n",
    )
    .unwrap();
    let untracked = metadata(&build_script, &checkout, "0.2.5", &[]);
    assert_eq!(untracked["GOOG_GIT_DIRTY"], "false");
    assert_eq!(
        untracked["GOOG_DISPLAY_VERSION"],
        ahead["GOOG_DISPLAY_VERSION"]
    );

    let archive = temporary.path().join("source-archive");
    fs::create_dir(&archive).unwrap();
    let supplied = [
        (
            "GOOG_BUILD_GIT_COMMIT",
            "0123456789abcdef0123456789abcdef01234567",
        ),
        ("GOOG_BUILD_GIT_DIRTY", "false"),
        ("GOOG_BUILD_GIT_DISTANCE", "0"),
        ("GOOG_BUILD_SOURCE_TAG", "v0.2.5-preview.2"),
    ];
    let archived_once = metadata(&build_script, &archive, "0.2.5-preview.2", &supplied);
    let archived_twice = metadata(&build_script, &archive, "0.2.5-preview.2", &supplied);
    assert_eq!(archived_once, archived_twice);
    assert_eq!(archived_once["GOOG_SEMANTIC_VERSION"], "0.2.5-preview.2");
    assert_eq!(archived_once["GOOG_DISPLAY_VERSION"], "0.2.5-preview.2");
    assert_eq!(archived_once["GOOG_RELEASE_CHANNEL"], "preview");
    assert_eq!(archived_once["GOOG_BUILD_TARGET"], "test-target");
}

fn metadata(
    build_script: &Path,
    working_directory: &Path,
    package_version: &str,
    supplied: &[(&str, &str)],
) -> HashMap<String, String> {
    let mut command = Command::new(build_script);
    command
        .current_dir(working_directory)
        .env("CARGO_PKG_VERSION", package_version)
        .env("TARGET", "test-target");
    for name in BUILD_ENVIRONMENT {
        command.env_remove(name);
    }
    for (name, value) in supplied {
        command.env(name, value);
    }

    let output = command.output().unwrap();
    assert_success(&output, "run build.rs");
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.strip_prefix("cargo:rustc-env="))
        .map(|line| {
            let (name, value) = line.split_once('=').unwrap();
            (name.to_owned(), value.to_owned())
        })
        .collect()
}

fn git(working_directory: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(working_directory)
        .output()
        .unwrap();
    assert_success(&output, &format!("git {}", args.join(" ")));
}

fn assert_success(output: &Output, operation: &str) {
    assert!(
        output.status.success(),
        "{operation} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
