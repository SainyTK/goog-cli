# goog

A Rust CLI for managing multiple Google accounts and interacting with Google APIs from the terminal.
It covers Drive, Docs, Sheets, and Gmail, and is designed for developers and power users who want a scriptable, terminal-native alternative to the browser.

## Features

- Log in to multiple Google accounts and switch between them, or let `goog` fall back across accounts automatically when a resource isn't accessible from the active one.
- Requests only the OAuth scopes a command needs, the first time it needs them, instead of asking for broad access up front.
- Stores tokens in the operating system keychain, never in plain config files.
- Interact with Drive (browse, list, upload, download, manage folders), Docs (read, search, and apply batch updates), Sheets (read and write values, raw batch updates), and Gmail (list, search, read messages, download attachments).
- Machine-readable JSON output on most read commands, for scripting.

## Installation

### One-liner (macOS and Linux)

```sh
curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh
```

This downloads the latest release binary for your platform into `~/.local/bin/goog`.
Make sure `~/.local/bin` is on your `PATH` (the installer prints a hint if it isn't).

Supported platforms: macOS (Apple Silicon and Intel) and Linux (x86_64 and aarch64).

To install a specific version instead of the latest release:

```sh
GOOG_VERSION=0.1.0 curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh
```

To install somewhere other than `~/.local/bin`:

```sh
GOOG_INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh
```

### From source

```sh
git clone https://github.com/SainyTK/goog-cli.git
cd goog-cli
cargo build --release
./target/release/goog --help
```

Verify the install:

```sh
goog --version
goog --help
```

## Getting started

`goog` needs a Google OAuth App (a GCP project's OAuth 2.0 client) before it can authorize any account.
You create this once, using your own Google Cloud project:

```sh
goog auth setup
```

`auth setup` can collect the client ID and secret interactively, or import them from a `client_secret_*.json` file downloaded from Google Cloud Console:

```sh
goog auth setup --client-secret-file ./client_secret_1234.json
```

Then authorize an account through the browser-based OAuth flow:

```sh
goog auth login
```

Log in to additional accounts the same way, and switch the active one at any time:

```sh
goog auth list
goog auth switch someone@example.com
```

## Usage

```sh
goog auth login
goog drive list
goog drive upload ./report.pdf --folder <folder-id>
goog docs get <document-id>
goog sheets values get <spreadsheet-id> "Sheet1!A1:B10"
goog mail list
```

Run `goog <command> --help` for the full set of options on any command.

## Development

```sh
cargo build
cargo test
```

See `CONTEXT.md` for the project's domain language and `docs/adr/` for architectural decisions.

## Releases

Releases are built and published automatically by [`.github/workflows/release.yml`](.github/workflows/release.yml) whenever a `v*.*.*` tag is pushed.
Each release includes prebuilt binaries for macOS (aarch64, x86_64) and Linux (aarch64, x86_64), which `install.sh` downloads.
