# Homebrew Tap Setup

Homebrew distribution uses `SainyTK/homebrew-tap`. The tap is generated from Canonical Release assets published by `SainyTK/goog-cli`; it does not build from branch-head source and does not own an independent version stream.

## Create The Tap Repository

Create the public tap repository once:

```sh
gh repo create SainyTK/homebrew-tap --public --description "Homebrew tap for SainyTK command-line tools"
gh repo clone SainyTK/homebrew-tap
cd homebrew-tap
mkdir -p Formula
printf '# SainyTK Homebrew Tap\n' > README.md
git add README.md
git commit -m "Initialize Homebrew tap"
git push origin main
```

## Connect Release Automation

In `SainyTK/goog-cli`, configure:

- Repository variable `GOOG_HOMEBREW_TAP_REPO` with value `SainyTK/homebrew-tap`.
- Repository secret `GOOG_HOMEBREW_TAP_TOKEN` with a token that can push to the tap repository.

When a stable version tag is published from `main`, `.github/workflows/release.yml` creates the Canonical Release and then writes `Formula/goog.rb` in the tap repository.
Preview tags from `preview` create GitHub pre-releases only and must not update the Homebrew tap.

## Generated Formula

The generated formula installs `goog` through:

```sh
brew install SainyTK/tap/goog
```

The formula downloads the same platform-specific Canonical Release assets used by the installer script:

- `goog-vX.Y.Z-aarch64-apple-darwin.tar.gz`
- `goog-vX.Y.Z-x86_64-apple-darwin.tar.gz`
- `goog-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
- `goog-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz`

Each formula URL has a matching checksum from the Release Asset `.sha256` file. The formula test runs `goog --help` to prove the installed binary starts.

Do not document or promise `brew install goog` until the project is intentionally submitted to and accepted by Homebrew core.
