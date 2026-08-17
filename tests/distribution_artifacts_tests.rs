use std::fs;
use std::process::Command;

#[test]
fn binary_reports_matching_human_and_structured_provenance() {
    let binary = env!("CARGO_BIN_EXE_goog");
    let human = Command::new(binary)
        .arg("--version")
        .output()
        .expect("--version should run");
    let structured = Command::new(binary)
        .args(["version", "--json"])
        .output()
        .expect("version --json should run");

    assert!(human.status.success());
    assert!(structured.status.success());

    let human = String::from_utf8(human.stdout).unwrap();
    let structured: serde_json::Value = serde_json::from_slice(&structured.stdout).unwrap();
    assert_eq!(
        human.trim(),
        format!("goog {}", structured["displayVersion"].as_str().unwrap())
    );
    assert_eq!(structured["semanticVersion"], env!("CARGO_PKG_VERSION"));
    assert!(structured["gitCommit"].as_str().unwrap().len() >= 7);
    assert!(!structured.to_string().contains(env!("CARGO_MANIFEST_DIR")));
}

#[test]
fn readme_covers_public_distribution_and_usage_contract() {
    let readme = fs::read_to_string("README.md").expect("README.md should exist");

    for expected in [
        "Early Open-Source CLI",
        "power users and AI agents",
        "JSON is also supported for programmatic use, but it is not the primary product surface.",
        "Install `goog` on macOS or Linux with:",
        "Install `goog` on Windows with:",
        "install.ps1 | iex",
        "[scriptblock]::Create",
        "$env:LOCALAPPDATA\\Programs\\goog\\bin",
        "Remove-Item -Recurse -Force \"$env:LOCALAPPDATA\\Programs\\goog\"",
        "Additional Installation Options",
        "latest Stable LTS Canonical Release by default",
        "--channel preview",
        "Check which release source produced the installed binary with:",
        "goog version --json",
        "compare `sourceTag` and `gitCommit` with the tag and commit",
        "`dirty` must be `false` for a published binary",
        "Uninstall",
        "cargo install --git https://github.com/SainyTK/goog-cli goog",
        "rm -f /usr/local/bin/goog \"$HOME/.local/bin/goog\"",
        "cargo uninstall goog",
        "delete `$HOME/.goog`",
        "auth state in `auth.json`",
        "goog auth setup",
        "goog auth login",
        "goog auth list",
        "goog auth switch",
        "goog drive ls",
        "goog docs map",
        "--fit-page --reserve-height 72",
        "docs/local-docs-image-staging.md",
        "reports each image's native point dimensions",
        "Pageless documents require explicit",
        "require `--allow-distortion`",
        "goog sheets values get",
        "goog mail list",
        "Release Flow",
        "Preview Release",
        "git push origin HEAD:preview",
        "Canonical Release",
        "Stable LTS Release",
        "git tag v0.2.4",
        "Contributor Workflow",
    ] {
        assert!(
            readme.contains(expected),
            "README.md should contain {expected:?}"
        );
    }
}

#[test]
fn installer_resolves_canonical_releases_and_supported_targets() {
    let installer = fs::read_to_string("install.sh").expect("install.sh should exist");

    for expected in [
        "https://api.github.com/repos/${REPO}/releases/latest",
        "https://api.github.com/repos/${REPO}/releases?per_page=30",
        "--channel",
        "stable|preview",
        "-preview\\.",
        "--version",
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        ".sha256",
        "checksum verification failed",
        "Windows uses the PowerShell installer",
        "install.ps1 | iex",
        "DEFAULT_INSTALL_DIR=\"/usr/local/bin\"",
        "INSTALL_DIR=\"${HOME}/.local/bin\"",
        "install directory is not writable",
        "is not on PATH",
        "--version must look like vX.Y.Z or vX.Y.Z-preview.N",
    ] {
        assert!(
            installer.contains(expected),
            "install.sh should contain {expected:?}"
        );
    }
}

#[test]
fn powershell_installer_resolves_canonical_releases_and_the_windows_target() {
    let installer = fs::read_to_string("install.ps1").expect("install.ps1 should exist");

    for expected in [
        "https://api.github.com/repos/$Repo/releases/latest",
        "https://api.github.com/repos/$Repo/releases?per_page=30",
        "-preview.",
        "-Channel",
        "-Version",
        "-InstallDir",
        "-NoPathUpdate",
        "GOOG_CHANNEL",
        "GOOG_VERSION",
        "GOOG_INSTALL_DIR",
        "GOOG_NO_MODIFY_PATH",
        "x86_64-pc-windows-msvc",
        "goog.exe",
        ".zip",
        ".sha256",
        "Tls12",
        "Get-FileHash",
        "-Algorithm SHA256",
        "checksum verification failed",
        "Expand-Archive",
        "LOCALAPPDATA",
        "Programs\\goog\\bin",
        "--channel must be stable or preview",
        "--version must look like vX.Y.Z or vX.Y.Z-preview.N",
        "open a new terminal",
    ] {
        assert!(
            installer.contains(expected),
            "install.ps1 should contain {expected:?}"
        );
    }
}

#[test]
fn powershell_installer_stays_pipeable_into_invoke_expression() {
    let installer = fs::read("install.ps1").expect("install.ps1 should exist");

    // A BOM survives Invoke-RestMethod and breaks `irm ... | iex`, and a
    // #Requires statement is rejected outright by Invoke-Expression.
    assert!(
        !installer.starts_with(&[0xEF, 0xBB, 0xBF]),
        "install.ps1 should not start with a UTF-8 BOM"
    );
    assert!(
        installer.is_ascii(),
        "install.ps1 should stay ASCII-only so it survives being piped into iex"
    );
    let text = String::from_utf8(installer).expect("install.ps1 should be valid UTF-8");
    assert!(
        !text
            .lines()
            .any(|line| line.trim_start().to_lowercase().starts_with("#requires")),
        "install.ps1 should not use a #Requires statement"
    );
}

#[test]
fn continuous_integration_workflow_checks_linux_and_windows() {
    let workflow =
        fs::read_to_string(".github/workflows/ci.yml").expect("CI workflow should exist");

    for expected in [
        "pull_request",
        "ubuntu-latest",
        "windows-latest",
        "cargo fmt --check",
        "cargo check --locked --all-targets",
        "cargo test --locked",
    ] {
        assert!(
            workflow.contains(expected),
            "CI workflow should contain {expected:?}"
        );
    }
}

#[test]
fn release_workflow_builds_assets_from_version_tags_only() {
    let workflow =
        fs::read_to_string(".github/workflows/release.yml").expect("release workflow should exist");

    for expected in [
        "tags:",
        "\"v*.*.*\"",
        "\"v*.*.*-preview.*\"",
        "release_channel=\"preview\"",
        "base_branch=\"preview\"",
        "release_channel=\"stable\"",
        "base_branch=\"main\"",
        "Tag must look like vX.Y.Z or vX.Y.Z-preview.N",
        "git merge-base --is-ancestor",
        "GOOG_BUILD_GIT_COMMIT: ${{ github.sha }}",
        "GOOG_BUILD_GIT_DIRTY: \"false\"",
        "GOOG_BUILD_GIT_DISTANCE: \"0\"",
        "GOOG_BUILD_SOURCE_TAG: ${{ github.ref_name }}",
        "Smoke test packaged binary",
        "scripts/verify-release-asset.sh",
        "Generate packaged asset checksum",
        "--prerelease",
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
        "windows-latest",
        "binary: goog.exe",
        "archive: zip",
        "7z a -tzip",
        "gh release create",
    ] {
        assert!(
            workflow.contains(expected),
            "release workflow should contain {expected:?}"
        );
    }
}

#[test]
fn release_asset_smoke_test_checks_provenance_and_command_surface() {
    let smoke_test = fs::read_to_string("scripts/verify-release-asset.sh")
        .expect("release asset smoke test should exist");

    for expected in [
        "tar -C \"$staging\" -xzf \"$asset\"",
        "*.zip)",
        "7z x",
        "unzip -q",
        "binary=\"$staging/goog.exe\"",
        "python_bin",
        "actual_version=\"$(\"$binary\" --version)\"",
        "\"$binary\" version --json",
        "\"semanticVersion\"",
        "\"gitCommit\"",
        "\"dirty\"",
        "\"sourceTag\"",
        "\"releaseChannel\"",
        "\"target\"",
        "\"$binary\" docs image insert --help",
        "\"$binary\" drive ls --help",
    ] {
        assert!(
            smoke_test.contains(expected),
            "release asset smoke test should contain {expected:?}"
        );
    }
}

#[test]
fn release_operator_docs_cover_channel_verification_and_recovery() {
    let docs = fs::read_to_string("docs/distribution/release-operator.md")
        .expect("release operator docs should exist");

    for expected in [
        "GitHub Releases are the only release authority for `goog`.",
        "Stable LTS releases are Canonical Releases from `main`.",
        "Preview releases are GitHub pre-releases from `preview`",
        "Cut A Preview Release",
        "git checkout -B preview",
        "git push origin v0.2.4-preview.1",
        "--channel preview",
        "Promote Preview To Stable LTS",
        "git push origin v0.1.0",
        "Verify Installer Script",
        "--channel preview",
        "On macOS",
        "On Linux",
        "On Windows",
        "goog-vX.Y.Z-x86_64-pc-windows-msvc.zip",
        "Verify Release Automation Changes",
        "cargo test --test distribution_artifacts_tests",
        "Rust-Native Fallback",
        "Recovery",
        "Never point users to branch-head binaries",
    ] {
        assert!(
            docs.contains(expected),
            "release docs should contain {expected:?}"
        );
    }
}
