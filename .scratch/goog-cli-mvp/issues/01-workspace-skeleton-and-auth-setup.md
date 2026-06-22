# 01 - Workspace skeleton + `goog auth setup`

Status: ready-for-agent

## Parent

`.scratch/goog-cli-mvp/PRD.md`

## What to build

Initialize the Cargo workspace with three crates: `goog-auth` (library), `goog-drive` (library), and `goog` (binary). Wire up the full `clap` command tree with stub implementations for every subcommand (returning "not yet implemented" where not built in this slice). Implement config file read/write for `~/.config/goog/config.toml`.

Deliver `goog auth setup` end-to-end:
- With no flags: print a numbered guide walking the user through GCP Console setup, then prompt interactively for the path to a downloaded `client_secret_*.json` file.
- With `--credentials <path>`: skip the prompt, read the file directly.
- Validate the file structure and surface a clear error if required fields (client ID, client secret) are missing or malformed.
- Store the OAuth App's client ID and client secret in `~/.config/goog/config.toml`.
- Exit with a non-zero status code on error; write errors to stderr, normal output to stdout.

## Acceptance criteria

- [ ] `cargo build` succeeds for the full workspace with all three crates present.
- [ ] `goog auth setup` prints a numbered GCP Console guide when run without flags.
- [ ] `goog auth setup` prompts interactively for a credentials file path when no `--credentials` flag is given.
- [ ] `goog auth setup --credentials <path>` imports without prompting.
- [ ] A valid `client_secret_*.json` is parsed and the client ID and client secret are written to `~/.config/goog/config.toml`.
- [ ] A malformed or missing credentials file produces a clear error on stderr and a non-zero exit code.
- [ ] All other subcommand stubs (`goog auth login`, `goog drive list`, etc.) parse without panicking and print "not yet implemented".
- [ ] `clap` argument parsing is unit-tested for the auth setup subcommand flags.

## Blocked by

None - can start immediately.
