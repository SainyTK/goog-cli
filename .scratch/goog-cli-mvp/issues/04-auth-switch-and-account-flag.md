# 04 - `goog auth switch` + `--account` flag

Status: ready-for-agent

## Parent

`.scratch/goog-cli-mvp/PRD.md`

## What to build

Implement `goog auth switch <email>` to update the Active Account in `~/.config/goog/config.toml`. The command should confirm the new Active Account in the terminal output. If the given email does not match any logged-in Account, surface a clear error.

Add a global `--account <email>` flag to the `clap` command tree so that any command can override the Active Account for a single invocation without permanently switching. The resolved Account (from `--account` or from config) must be threaded through to all command handlers consistently.

## Acceptance criteria

- [ ] `goog auth switch <email>` updates the active account field in config and prints confirmation.
- [ ] `goog auth switch` with an unrecognized email produces a clear error on stderr.
- [ ] After `goog auth switch`, `goog auth list` shows the new Active Account marked correctly.
- [ ] `--account <email>` on any command overrides the Active Account for that invocation only.
- [ ] `--account` with an unrecognized email produces a clear error.
- [ ] Config read/write and flag parsing are unit-tested.

## Blocked by

- `02-auth-login-loopback-and-auth-list.md`
