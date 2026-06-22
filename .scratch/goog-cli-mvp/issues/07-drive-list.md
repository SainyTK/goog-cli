# 07 - `goog drive list`

Status: ready-for-agent

## Parent

`.scratch/goog-cli-mvp/PRD.md`

## What to build

Implement `goog drive list` in the `goog-drive` crate using hand-written `serde` types for the Drive Files API response. The command receives an `AuthClient` - it makes no direct auth calls.

Behavior:
- Default: return the 50 most recent files, displayed in a human-readable table (name, file ID, MIME type, last modified date).
- `--limit N`: return at most N results.
- `--all`: lazily fetch all pages and stream results as they arrive, rather than buffering everything before printing.
- `--json`: emit each file as a newline-delimited JSON object.
- Global config key `output = "json"` flips the default to JSON; `--json` always overrides regardless of config.
- A `DriveError::NotFound` or permission error from the API surfaces as a clear error on stderr with a non-zero exit.
- `--quiet` suppresses any informational messages (pagination progress, etc.).

## Acceptance criteria

- [ ] `goog drive list` displays a table of the 50 most recent files by default.
- [ ] `--limit N` returns at most N results.
- [ ] `--all` fetches all pages and prints results as each page arrives.
- [ ] `--json` output is valid newline-delimited JSON with name, id, mimeType, and modifiedTime fields.
- [ ] `output = "json"` in config produces JSON output without the flag.
- [ ] A mocked 403 or 404 API response surfaces a clear error on stderr with a non-zero exit.
- [ ] `list_files()` is tested with `wiremock`: correct deserialization of a single-page response, correct next-page-token handling for multi-page responses, and `DriveError` on a 404 response.
- [ ] `clap` parsing for `--limit`, `--all`, and `--json` is unit-tested.

## Blocked by

- `06-incremental-scope-authorization.md`
