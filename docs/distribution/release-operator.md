# Release Operator Workflow

GitHub Releases are the only release authority for `goog`.
Stable LTS releases are Canonical Releases from `main`.
Preview releases are GitHub pre-releases from `preview` for opt-in validation before stable promotion.
Installer Script and Rust-native fallback documentation point users toward a tagged release or source installation, never a branch-head binary.

## Cut A Preview Release

1. Start from `develop` or the current release-prep branch and confirm it is current:

   ```sh
   git checkout develop
   git pull --ff-only origin develop
   cargo test
   ```

2. Create or update the `preview` branch to the tested commit:

   ```sh
   git checkout -B preview
   git push origin preview
   ```

3. Confirm `Cargo.toml` contains the intended preview version, such as `0.2.4-preview.1`.

4. Create and push a preview version tag:

   ```sh
   git tag v0.2.4-preview.1
   git push origin v0.2.4-preview.1
   ```

5. Watch the `Canonical Release` workflow.
   It verifies the tag commit is reachable from `origin/preview`, builds macOS arm64, macOS x64, Linux x64, and Linux arm64 Release Assets, uploads checksums, and creates a GitHub pre-release.

6. Verify the installer can consume the preview channel:

   ```sh
   tmp="$(mktemp -d)"
   curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh -s -- --channel preview --install-dir "$tmp/bin"
   "$tmp/bin/goog" --help
   ```

## Promote Preview To Stable LTS

1. Merge or fast-forward the tested preview commit into `main`.

2. Replace the preview package version with the stable version in `Cargo.toml`.

3. Run the stable release verification gates, then cut the stable tag from `main`.

4. After the stable release is published and installed-binary checks pass, fast-forward `develop` to the same commit if no divergence remains.

## Cut A Stable LTS Release

1. Start from `main` and confirm it is current:

   ```sh
   git checkout main
   git pull --ff-only origin main
   cargo test
   ```

2. Confirm `Cargo.toml` contains the intended version.

3. Create and push a version tag:

   ```sh
   git tag v0.1.0
   git push origin v0.1.0
   ```

4. Watch the `Canonical Release` workflow.
   It verifies the tag commit is reachable from `origin/main`, builds macOS arm64, macOS x64, Linux x64, and Linux arm64 Release Assets, uploads checksums, and creates the GitHub Release.

5. Confirm the GitHub Release contains:

   - `goog-vX.Y.Z-aarch64-apple-darwin.tar.gz`
   - `goog-vX.Y.Z-x86_64-apple-darwin.tar.gz`
   - `goog-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
   - `goog-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz`
   - One `.sha256` file for each archive.

## Verify Installer Script

On macOS:

```sh
tmp="$(mktemp -d)"
curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh -s -- --version v0.1.0 --install-dir "$tmp/bin"
"$tmp/bin/goog" --help
```

On Linux:

```sh
tmp="$(mktemp -d)"
curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh -s -- --version v0.1.0 --install-dir "$tmp/bin"
"$tmp/bin/goog" --help
```

The installer must download from the GitHub Release, verify the `.sha256` checksum, and install a runnable `goog` binary.

For preview:

```sh
tmp="$(mktemp -d)"
curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh -s -- --channel preview --install-dir "$tmp/bin"
"$tmp/bin/goog" --help
```

## Verify Release Automation Changes

Before changing `.github/workflows/release.yml` or `install.sh`, run:

```sh
sh -n install.sh
cargo test --test distribution_artifacts_tests
cargo test
```

## Rust-Native Fallback

For users outside the v1 binary support matrix, use source installation:

```sh
cargo install --git https://github.com/SainyTK/goog-cli goog
```

This path requires a local Rust toolchain and builds from source instead of consuming Release Assets.

## Recovery

If preview release automation fails before the GitHub Release is created:

1. Fix the workflow or code on `preview`.
2. Delete the failed local and remote tag:

   ```sh
   git tag -d v0.2.4-preview.1
   git push origin :refs/tags/v0.2.4-preview.1
   ```

3. Create the tag again from the fixed `preview` commit and push it.

If stable release automation fails before the GitHub Release is created:

1. Fix the workflow or code on `main`.
2. Delete the failed local and remote tag:

   ```sh
   git tag -d v0.1.0
   git push origin :refs/tags/v0.1.0
   ```

3. Create the tag again from the fixed `main` commit and push it.

If the GitHub Release was created with missing or incorrect assets:

1. Delete the broken GitHub Release from the Releases page or with `gh release delete v0.1.0`.
2. Delete the remote tag if the tag points at the wrong commit.
3. Fix `main`, recreate the tag if needed, and rerun the release workflow.

Never point users to branch-head binaries as a recovery shortcut.
