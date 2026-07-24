# GitHub Releases Own Distribution

`goog` publishes installable versions as tagged GitHub Releases.
Stable LTS releases are created from `main`.
Preview releases are created from `preview` as GitHub pre-releases for opt-in validation before stable promotion.
The GitHub-hosted installer script is the only supported distribution channel.
It resolves to those release assets instead of building from branch-head source, keeping one release authority while still supporting a simple install path.

The first binary release scope is macOS arm64, macOS x64, Linux x64, and Linux arm64. The installer script supports macOS and Linux in v1; Windows users use source installation through Cargo until there is enough demand to add Windows release assets.

The installer script is served from `main`, but it installs the latest stable GitHub Release by default rather than building or downloading branch-head code.
Users can pass `--channel preview` to install the latest preview pre-release or an explicit version such as `--version v0.1.0` to pin installation to a specific release tag.

The public installer entrypoint is `install.sh` at the repository root so users and contributors can inspect the same path used by the one-line install command.

Every `goog` invocation checks Canonical Releases in parallel with command execution.
Stable builds consider stable releases, while preview builds consider both preview releases and newer stable releases.
Known updates are cached locally for 24 hours, no-update results are refreshed after 15 minutes, and failed checks are retried after one hour.
When a newer semantic version is known, the CLI prints its version and a public installer command pinned to that exact tag on stderr after all command output, including help, version, and error paths.
The check is advisory: network, cache, and response failures stay silent and never change command output on stdout or command exit status.

Release Automation runs from GitHub Actions when a stable version tag is pushed from `main` or a preview version tag is pushed from `preview`.
The workflow builds macOS and Linux Release Assets, attaches checksums, and creates the GitHub Release.
Manual release steps are fallback documentation only.
