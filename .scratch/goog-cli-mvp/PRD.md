# PRD: goog CLI MVP

Status: ready-for-agent

## Problem Statement

Developers and power users who work with multiple Google accounts have no good terminal-native tool for managing their Google workspace content. The browser is slow and non-scriptable. `gcloud` handles GCP resources but not consumer Google APIs (Drive, Docs, Sheets). There is no `gh`-style CLI for Google that handles multi-account auth, works in scripts, and covers the APIs people actually use day-to-day.

## Solution

A Rust CLI called `goog` that manages multiple Google Accounts through a single OAuth App, lets users authenticate interactively via a browser flow, and provides a curated set of commands for Google Drive. Auth state is stored securely in the system keychain. Output is human-readable by default and machine-parseable via `--json`. The MVP covers auth management and full Drive access (list, upload, download).

## User Stories

### Auth Setup

1. As a developer, I want to run `goog auth setup` to import my OAuth credentials from a downloaded `client_secret_*.json` file, so that I can start using `goog` without manually entering client IDs.
2. As a developer, I want `goog auth setup` to print a numbered guide for creating a GCP project and downloading credentials, so that I can complete the setup without leaving the terminal to search for documentation.
3. As a developer, I want `goog auth setup --credentials <path>` to accept the credentials file path as a flag, so that I can automate the setup in a script.
4. As a developer, I want `goog auth setup` to prompt me interactively for the credentials file path if no flag is given, so that the command is usable without memorizing flags.
5. As a developer, I want `goog auth setup` to validate the credentials file and report a clear error if it is malformed or missing required fields, so that I know immediately if something is wrong.

### Auth Login

6. As a developer, I want to run `goog auth login` to authorize a Google Account through a browser-based OAuth flow, so that `goog` can act on my behalf.
7. As a developer, I want the browser to open automatically during `goog auth login`, so that I do not have to copy and paste a URL.
8. As a developer, I want the OAuth authorization code to be captured automatically via a localhost redirect, so that I do not have to manually paste it back into the terminal.
9. As a developer on a remote machine without a browser, I want to pass `--no-browser` to `goog auth login` and receive a URL and device code to authorize from another device, so that I can authenticate over SSH.
10. As a developer, I want `goog auth login` to store my Token securely in the system keychain, so that my refresh token is not exposed in plaintext files.
11. As a developer, I want to log in to multiple Google Accounts with `goog auth login`, so that I can manage content across personal and work accounts from one terminal.
12. As a developer, I want `goog auth login` to confirm success with the authorized account's email address, so that I know which Account was added.

### Auth List and Switch

13. As a developer, I want to run `goog auth list` to see all logged-in Accounts and which one is currently Active, so that I can orient myself before running commands.
14. As a developer, I want `goog auth list --json` to emit machine-readable account info, so that I can use it in scripts.
15. As a developer, I want to run `goog auth switch <email>` to change the Active Account, so that subsequent commands target the right account without repeating `--account`.
16. As a developer, I want `goog auth switch` to confirm the new Active Account, so that I can verify the switch happened correctly.
17. As a developer, I want to pass `--account <email>` to any command to override the Active Account for that one invocation, so that I can act on behalf of a specific account without permanently switching.

### Token Refresh

18. As a developer, I want my commands to succeed after more than an hour without re-authenticating, so that long scripts are not interrupted by token expiry.
19. As a developer, I want token refresh to happen silently in the background, so that I am not aware of it unless it fails.
20. As a developer, I want a clear error message if my refresh token has been revoked, prompting me to run `goog auth login` again, so that I understand what action to take.

### Incremental Scope Authorization

21. As a developer, I want `goog auth login` to only ask for minimal permissions upfront, so that I am not confronted with a long list of scary permissions before I have used any APIs.
22. As a developer, I want `goog drive list` to trigger a focused Drive-only consent prompt on first use if Drive scope has not been granted, so that I understand exactly what permission I am granting.
23. As a developer, I want subsequent Drive commands to not re-prompt for consent once Drive scope has been granted, so that the auth flow does not repeat unnecessarily.

### Drive List

24. As a developer, I want to run `goog drive list` to see the files in my Google Drive, so that I can browse my content from the terminal.
25. As a developer, I want `goog drive list` to return the 50 most recent files by default, so that the command is fast and does not overwhelm me.
26. As a developer, I want to pass `--limit N` to `goog drive list` to control how many results are returned, so that I can tune the output to my needs.
27. As a developer, I want to pass `--all` to `goog drive list` to fetch every file across all pages, so that I can get a complete listing for scripts that need it.
28. As a developer, I want `goog drive list` output to include file name, ID, MIME type, and last modified date in a readable table, so that I can quickly identify files.
29. As a developer, I want `goog drive list --json` to emit newline-delimited JSON, so that I can pipe it to `jq` or other tools.
30. As a developer, I want to set `output = "json"` in my config once and have all commands default to JSON output, so that I do not have to pass `--json` in every script.

### Drive Download

31. As a developer, I want to run `goog drive download <file-id>` to download a file from Google Drive to my current directory, so that I can retrieve files without opening the browser.
32. As a developer, I want to pass `--output <path>` to specify where the downloaded file is saved, so that I can control the destination.
33. As a developer, I want downloads to stream directly to disk without loading the file into memory, so that large files do not exhaust RAM.
34. As a developer, I want a progress bar shown during download, so that I can see how much is left for large files.
35. As a developer, I want a clear error if the file ID does not exist or I lack permission, so that I understand why the download failed.

### Drive Upload

36. As a developer, I want to run `goog drive upload <path>` to upload a local file to Google Drive, so that I can add files to Drive from the terminal.
37. As a developer, I want uploads to stream directly from disk without loading the file into memory, so that large files do not exhaust RAM.
38. As a developer, I want files over 5 MB to use Google's resumable upload protocol automatically, so that uploads survive network interruptions.
39. As a developer, I want a progress bar shown during upload, so that I can monitor progress for large files.
40. As a developer, I want the uploaded file's Drive ID and share URL printed on success, so that I can use them immediately in scripts or share with others.
41. As a developer, I want to pass `--folder <folder-id>` to upload into a specific Drive folder, so that I can organize uploads without using the browser.

### Output and Config

42. As a developer, I want `goog` to exit with a non-zero status code on error, so that scripts can detect failures reliably.
43. As a developer, I want error messages to go to stderr and normal output to stdout, so that I can redirect them independently in scripts.
44. As a developer, I want a global `--quiet` flag to suppress progress bars and informational messages, so that scripts produce clean output.

## Implementation Decisions

### Project Structure

- Cargo workspace with one library crate per API surface: `goog-auth`, `goog-drive`.
- A thin `goog` binary crate composes them using `clap` with the derive macro for the command tree.
- The `goog-auth` crate is a dependency of all API crates and owns the `AuthClient` type.

### Auth Layer

- `goog-auth` exposes an `AuthClient` -- an auth-aware wrapper around a `reqwest::Client` -- as the single interface between auth logic and API logic.
- `AuthClient` implements token refresh as middleware: proactive refresh if the stored Token is within a threshold of expiry, plus a 401-triggered refresh-and-retry for clock skew.
- `AuthClient` checks granted Scopes before each API call and triggers an incremental OAuth flow if a required Scope is missing.
- All API crates accept an `AuthClient` and make no direct auth calls themselves.

### Credential Storage

- Non-sensitive config (Active Account email, OAuth client ID) lives in `~/.config/goog/config.toml`.
- Tokens (access token, refresh token, expiry, granted scopes) are stored per Account in the system keychain via the `keyring` crate.
- The keychain entry key is namespaced by Account email to support multiple Accounts.

### OAuth Flow

- Login uses the loopback redirect pattern: a temporary HTTP server binds to a random localhost port, the browser opens to Google's consent screen with that port as the redirect URI, the auth code is captured from the incoming request.
- `--no-browser` switches to device authorization grant (RFC 8628): prints a URL and user code, polls Google's token endpoint until the user completes authorization.

### Scope Management

- Each command in each API crate declares a static list of required Scopes.
- `AuthClient` compares required Scopes against the granted Scopes stored in the Token before making any call.
- If Scopes are missing, `AuthClient` triggers an incremental OAuth flow for the missing Scopes only and updates the Token in the keychain.

### Drive Commands

- `goog drive list` uses the Drive Files API with a configurable page size. `--all` fetches pages lazily and streams results as they arrive.
- `goog drive download` streams the response body directly to a file handle using `response.bytes_stream()` with `tokio::io::copy`.
- `goog drive upload` streams a file from disk into the request body using `reqwest::Body::wrap_stream`. Files over 5 MB use Google's resumable upload API (initiates a session, uploads in chunks).
- Progress bars use `indicatif` for both upload and download.

### Error Handling

- Library crates (`goog-auth`, `goog-drive`) define typed error enums with `thiserror`.
- The binary crate uses `anyhow` for propagation and display.
- On error, `goog` exits with a non-zero status code, writes the error message to stderr, and writes nothing to stdout.

### Output Format

- Human-readable output uses a table formatter (e.g., `comfy-table`) for list commands.
- `--json` emits newline-delimited JSON via `serde_json`.
- A global config key `output = "json"` flips the default; `--json` always overrides.

## Testing Decisions

A good test asserts external behavior -- what a caller observes -- not internal implementation. Tests should not assert which internal functions were called, only what was returned or what HTTP request was made.

### Seam 1: `AuthClient` in `goog-auth`

Tests at this seam verify the contract that API crates depend on:
- A valid, non-expired Token produces requests with the correct `Authorization: Bearer <token>` header.
- An expired Token is refreshed before the request is made (proactive path).
- A 401 response triggers a refresh-and-retry exactly once (reactive path).
- A missing Scope triggers an incremental OAuth flow before the request proceeds.
- A revoked refresh token surfaces as a typed `AuthError::TokenRevoked`.

These tests use `wiremock` to mock Google's token endpoint and the target API endpoint. No real credentials or keychain are needed.

### Seam 2: `goog-drive` public functions

Tests at this seam verify Drive behavior:
- `list_files()` returns correctly deserialized results for a mocked response, and correctly handles next-page tokens when `--all` is set.
- `download()` streams bytes to disk correctly for a mocked binary response.
- `upload()` sends the correct multipart request for small files and initiates a resumable session for files over 5 MB.
- `list_files()` surfaces a `DriveError::NotFound` for a mocked 404 response.

These tests use `wiremock` to mock Google Drive's HTTP endpoints. File I/O uses temporary directories via `tempfile`.

### Seam 3: CLI argument parsing

Tests at this seam verify that subcommands and flags parse correctly to the right command variant and option values. Pure unit tests using `clap`'s built-in test utilities -- no HTTP, no disk I/O.

### Integration tests

Real-credential tests that exercise the full stack (keychain, browser flow, live Google APIs) are gated behind `--features integration-tests` and run locally only. These are not part of CI.

## Out of Scope

- Docs, Sheets, Slides, and Gmail API commands (post-MVP).
- `goog auth logout` and token revocation (post-MVP).
- Shared drive support in Drive commands.
- Drive file search and query filtering.
- Drive file metadata editing (rename, move, star).
- Google Workspace (G Suite) domain accounts and service account authentication.
- Windows support (keychain behavior on Windows via `keyring` is untested for MVP).
- Shell completions.
- A configuration command (`goog config set`) -- config is edited by hand for MVP.

## Further Notes

- The `goog-auth` crate is the most critical piece to get right before any API crate is built. All API crates depend on the `AuthClient` interface being stable.
- The recommended build order is: `goog-auth` fully working (including token refresh and incremental scopes), then `goog` binary with auth subcommands, then `goog-drive`.
- Refer to `CONTEXT.md` for canonical term definitions (Account, Token, Scope, Active Account, OAuth App, Incremental Authorization, Resumable Upload).
- Refer to `docs/adr/` for the rationale behind key decisions: single OAuth App model (ADR-0001), system keychain for tokens (ADR-0002), incremental scope authorization (ADR-0003), Cargo workspace layout (ADR-0004), hand-written API types (ADR-0005).
