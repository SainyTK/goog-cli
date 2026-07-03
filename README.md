# goog

`goog` is an Early Open-Source CLI for Google APIs. It is built for developers and power users who want scriptable, terminal-native access to Google Drive, Google Docs, Google Sheets, and GoogleMail while the command surface continues to grow.

The CLI uses one OAuth App for all accounts, stores account tokens in the system keychain, and requests API scopes incrementally when commands need them.

## Current API Coverage

`goog` currently includes:

- Google Drive file and folder listing, upload, and download commands.
- Google Docs document mapping, text search, content lookup, raw document reads, and raw batch updates.
- Google Sheets spreadsheet reads, values reads and writes, appends, clears, and structural batch updates.
- GoogleMail message listing, search, raw message reads, and attachment downloads.
- Multi-account OAuth setup, login, account listing, and active account switching.

## Installation

### Installer Script

The installer script is the default install path for macOS and Linux. It installs the latest Canonical Release from GitHub Releases rather than branch-head code.

```sh
curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh
```

Install a specific Canonical Release with:

```sh
curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh -s -- --version v0.1.0
```

The installer supports macOS arm64, macOS x64, Linux x64, and Linux arm64 Release Assets.

### Homebrew Tap

Homebrew users can install from the project tap after a Canonical Release has been published:

```sh
brew install SainyTK/tap/goog
```

This is the supported tap install path. `goog` is not advertised as available through the official Homebrew core registry.

### Rust-Native Fallback

Users outside the binary release support matrix can install from source with Cargo:

```sh
cargo install --git https://github.com/SainyTK/goog-cli goog
```

This path requires a local Rust toolchain.

## OAuth Setup

Create a Google OAuth client for a desktop or web application, then configure `goog` once:

```sh
goog auth setup
```

You can also import a downloaded Google client secret file:

```sh
goog auth setup --client-secret-file client_secret_123.apps.googleusercontent.com.json
```

Authorize a Google Account:

```sh
goog auth login
```

List authorized Accounts:

```sh
goog auth list
goog auth list --json
```

Switch the Active Account:

```sh
goog auth switch alice@example.com
```

Run one command against a different Account without switching:

```sh
goog --account bob@example.com drive ls
```

## Examples

### Drive

```sh
goog drive ls --limit 20
goog drive folder list --parent FOLDER_ID --json
goog drive upload ./report.pdf --folder FOLDER_ID
goog drive download FILE_ID --output ./report.pdf
```

### Docs

```sh
goog docs map DOCUMENT_ID
goog docs search-text DOCUMENT_ID "quarterly plan"
goog docs get-content DOCUMENT_ID --heading "Summary"
goog docs batch-update DOCUMENT_ID --requests ./requests.json
```

### Sheets

```sh
goog sheets get SPREADSHEET_ID --fields 'properties.title,sheets.properties'
goog sheets values get SPREADSHEET_ID 'Sheet1!A1:D10'
goog sheets values update SPREADSHEET_ID 'Sheet1!A1' --values ./value-range.json
goog sheets values append SPREADSHEET_ID 'Sheet1!A:D' --values ./rows.json
```

### GoogleMail

```sh
goog mail list --limit 10
goog mail search 'from:alerts@example.com newer_than:7d'
goog mail read MESSAGE_ID
goog mail attachment download MESSAGE_ID ATTACHMENT_ID --output invoice.pdf
```

Use `goog help`, `goog <command> --help`, and nested command help for the full command reference.

## Contributor Workflow

Install local dependencies:

```sh
cargo fetch
npm install
```

Run checks before opening a pull request:

```sh
cargo fmt --check
cargo test
npm run typecheck
```

Issues live in GitHub Issues. Pick work labeled `ready-for-agent` and `Sandcastle`, keep changes scoped to the issue, and link the issue from the pull request.

Pull requests should include:

- A concise summary of user-facing behavior.
- Tests or verification notes covering the changed behavior.
- Documentation updates when commands, OAuth behavior, or distribution paths change.
- Release-document updates when Canonical Release assets, installer behavior, Homebrew tap behavior, or operator steps change.

Distribution changes must keep GitHub Releases as the only Canonical Release authority.
