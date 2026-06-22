# 06 - Incremental scope authorization

Status: ready-for-agent

## Parent

`.scratch/goog-cli-mvp/PRD.md`

## What to build

Extend `AuthClient` to check required OAuth Scopes before every API call and trigger an incremental OAuth flow for any that are missing.

Each command in each API crate must declare a static list of required Scopes (e.g., Drive commands declare `https://www.googleapis.com/auth/drive`). Before `AuthClient` makes a call, it compares the required Scopes against the granted Scopes stored in the Account's keychain Token. If any are missing, it runs the loopback OAuth flow scoped only to the missing Scopes, then merges the newly granted Scopes into the existing Token and updates the keychain.

Subsequent calls that require the same Scope must not re-prompt - the keychain Token is the source of truth.

## Acceptance criteria

- [ ] `goog drive list` (from a freshly logged-in Account with no Drive scope) triggers a browser-based consent prompt for Drive scope only.
- [ ] After consenting, the Drive scope is persisted to the keychain Token.
- [ ] A second `goog drive list` immediately after does not re-prompt.
- [ ] `goog auth login` itself still requests only minimal scopes (profile, email) - no Drive scope appears on the initial consent screen.
- [ ] Scope checking and the incremental flow trigger are tested against `wiremock` mocks - one test for the missing-scope path (re-auth fires), one for the already-granted path (no re-auth).

## Blocked by

- `05-auth-client-token-refresh-middleware.md`
