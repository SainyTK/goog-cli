# Windows Release Assets And PowerShell Installer

Windows x64 (`x86_64-pc-windows-msvc`) is part of the binary release matrix.
ADR 0013 deferred Windows to source installation through Cargo; that deferral no longer applies.
Windows on Arm is not a separate release target because it runs the x64 build under emulation.

Windows Release Assets are `.zip` archives containing `goog.exe`.
macOS and Linux Release Assets stay `.tar.gz` archives containing `goog`.
Zip is what Windows users expect, and it is the format future package-manager manifests consume.
Each Windows archive ships the same `.sha256` sidecar as every other Release Asset, so checksum verification is identical across platforms.

The public Windows installer entrypoint is `install.ps1` at the repository root, alongside `install.sh`.
It resolves the same Canonical Releases, verifies the same checksums, and supports the same channel and version selection.
It installs per-user to `%LOCALAPPDATA%\Programs\goog\bin` and appends that directory to the user PATH, so installation never requires Administrator rights.
Piping a script into `iex` cannot forward arguments, so the installer also reads `GOOG_CHANNEL`, `GOOG_VERSION`, `GOOG_INSTALL_DIR`, and `GOOG_NO_MODIFY_PATH`.

`install.sh` keeps refusing to run under MSYS, MinGW, Cygwin, and Git Bash, but now points at `install.ps1` instead of at Cargo.
Only the PowerShell installer manages the user PATH; two installers writing the same location would drift.

The update notice prints the PowerShell installer command on Windows and the `curl` command everywhere else, so the suggested command is always runnable in the shell the user is holding.

Continuous integration runs `cargo fmt --check`, `cargo check`, and `cargo test` on Linux and Windows for every push and pull request.
Release Automation only runs on version tags, so without this a Windows regression would first surface mid-release, after a tag exists.

Supersedes the Windows paragraph of ADR 0013.
