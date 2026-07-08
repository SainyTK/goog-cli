# Release Operator Workflow

GitHub Releases are the only Canonical Release authority for `goog`. Installer Script, Homebrew Tap, and Rust-native fallback documentation all point users toward a tagged release or source installation, never a branch-head binary.

## Cut A Canonical Release

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

4. Watch the `Canonical Release` workflow. It verifies the tag commit is reachable from `origin/main`, builds macOS arm64, macOS x64, Linux x64, and Linux arm64 Release Assets, uploads checksums, and creates the GitHub Release.

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

## Verify Homebrew Tap

After release automation updates `SainyTK/homebrew-tap`, verify:

```sh
brew update
brew install SainyTK/tap/goog
goog --help
brew test SainyTK/tap/goog
```

Do not document official Homebrew core installation unless `goog` has been accepted there. The supported Homebrew path is `brew install SainyTK/tap/goog`.

## Verify Release Automation Changes

Before changing `.github/workflows/release.yml`, `install.sh`, or Homebrew formula generation, run:

```sh
sh -n install.sh
ruby -c scripts/render-homebrew-formula.rb
cargo test --test distribution_artifacts_tests
cargo test
```

To verify formula rendering without publishing a release, create local checksum fixtures and render the formula:

```sh
tmp="$(mktemp -d)"
for target in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
  printf '%064d  goog-v0.1.0-%s.tar.gz\n' 1 "$target" > "$tmp/goog-v0.1.0-$target.tar.gz.sha256"
done
ruby scripts/render-homebrew-formula.rb v0.1.0 "$tmp" > "$tmp/goog.rb"
ruby -c "$tmp/goog.rb"
```

## Rust-Native Fallback

For users outside the v1 binary support matrix, use source installation:

```sh
cargo install --git https://github.com/SainyTK/goog-cli goog
```

This path requires a local Rust toolchain and builds from source instead of consuming Release Assets.

## Recovery

If release automation fails before the GitHub Release is created:

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

If the Homebrew tap update fails after the GitHub Release is correct:

1. Keep the GitHub Release as the Canonical Release.
2. Rerun the failed workflow job after fixing tap credentials or repository configuration.
3. If needed, generate the formula locally:

   ```sh
   gh release download v0.1.0 --repo SainyTK/goog-cli --pattern '*.sha256' --dir checksums
   ruby scripts/render-homebrew-formula.rb v0.1.0 checksums > ../homebrew-tap/Formula/goog.rb
   ```

4. Commit the generated formula to `SainyTK/homebrew-tap`.

Never point users to branch-head binaries as a recovery shortcut.
