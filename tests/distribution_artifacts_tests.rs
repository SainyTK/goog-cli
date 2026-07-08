use std::fs;
use std::path::Path;

#[test]
fn readme_covers_public_distribution_and_usage_contract() {
    let readme = fs::read_to_string("README.md").expect("README.md should exist");

    for expected in [
        "Early Open-Source CLI",
        "power users and AI agents",
        "JSON is also supported for programmatic use, but it is not the primary product surface.",
        "Install `goog` on macOS or Linux with:",
        "Additional Installation Options",
        "latest Stable LTS Canonical Release by default",
        "--channel preview",
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
        "goog sheets values get",
        "goog mail list",
        "Contributor Workflow",
    ] {
        assert!(
            readme.contains(expected),
            "README.md should contain {expected:?}"
        );
    }

    assert!(
        !readme.contains("brew install SainyTK/tap/goog"),
        "README.md should not advertise Homebrew installation until the tap is actually public"
    );
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
        "Windows binary releases are not supported yet",
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
        "--prerelease",
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "gh release create",
        "render-homebrew-formula.rb",
    ] {
        assert!(
            workflow.contains(expected),
            "release workflow should contain {expected:?}"
        );
    }
}

#[test]
fn homebrew_formula_renderer_contains_tap_install_contract() {
    let renderer = fs::read_to_string("scripts/render-homebrew-formula.rb")
        .expect("formula renderer should exist");

    for expected in [
        "class Goog < Formula",
        "SainyTK/goog-cli",
        "on_macos",
        "on_linux",
        "sha256",
        "bin.install \"goog\"",
        "goog --help",
    ] {
        assert!(
            renderer.contains(expected),
            "formula renderer should contain {expected:?}"
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
        "Homebrew tap updates are stable-only",
        "Promote Preview To Stable LTS",
        "git push origin v0.1.0",
        "Verify Installer Script",
        "On macOS",
        "On Linux",
        "brew install SainyTK/tap/goog",
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

#[test]
fn documented_homebrew_tap_setup_is_available_until_tap_exists() {
    let docs = fs::read_to_string("docs/distribution/homebrew-tap.md")
        .expect("Homebrew tap setup docs should exist");

    for expected in [
        "gh repo create SainyTK/homebrew-tap",
        "Formula/goog.rb",
        "GOOG_HOMEBREW_TAP_REPO",
        "GOOG_HOMEBREW_TAP_TOKEN",
        "brew install SainyTK/tap/goog",
        "goog-vX.Y.Z-aarch64-apple-darwin.tar.gz",
        "goog --help",
    ] {
        assert!(
            docs.contains(expected),
            "Homebrew tap setup docs should contain {expected:?}"
        );
    }

    assert!(
        Path::new("scripts/render-homebrew-formula.rb").exists(),
        "formula renderer should exist"
    );
}
