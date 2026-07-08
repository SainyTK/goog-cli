# GitHub Releases Own Distribution

`goog` publishes installable versions as tagged GitHub Releases.
Stable LTS releases are created from `main`.
Preview releases are created from `preview` as GitHub pre-releases for opt-in validation before stable promotion.
Distribution channels such as the GitHub-hosted installer script, Homebrew tap, and Rust-native install command resolve to those release assets instead of each owning an independent version stream, keeping one release authority while still supporting simple install paths.

The first binary release scope is macOS arm64, macOS x64, Linux x64, and Linux arm64. The installer script supports macOS and Linux in v1; Windows users use source installation through Cargo until there is enough demand to add Windows release assets.

The installer script is served from `main`, but it installs the latest stable GitHub Release by default rather than building or downloading branch-head code.
Users can pass `--channel preview` to install the latest preview pre-release or an explicit version such as `--version v0.1.0` to pin installation to a specific release tag.

The public installer entrypoint is `install.sh` at the repository root so users and contributors can inspect the same path used by the one-line install command.

Homebrew distribution uses a `SainyTK/homebrew-tap` repository with a `Formula/goog.rb` formula that downloads the same Canonical Release assets. Users install it with `brew install SainyTK/tap/goog`. The application repository remains responsible for release artifacts; the tap only describes how Homebrew installs them.

Release Automation runs from GitHub Actions when a stable version tag is pushed from `main` or a preview version tag is pushed from `preview`.
The workflow builds macOS and Linux Release Assets, attaches checksums, and creates the GitHub Release.
Stable releases also update the Homebrew Tap formula; preview releases do not.
Manual release steps are fallback documentation only.
