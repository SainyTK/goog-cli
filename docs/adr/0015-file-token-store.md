# File Token Store

`goog` stores authorized Accounts, the Active Account, OAuth Tokens, and Resource Account Mappings in `~/.goog/auth.json` by default, not in the operating-system keychain or the setup config.
The CLI is designed first for terminal-native power users and AI agents, and OS password prompts during ordinary command execution break that workflow.
The Token Store file is still secret-bearing local state and must be written with restricted permissions, kept out of git, and treated as equivalent to account access within the authorized Scopes.
The file uses a versioned JSON envelope so future auth-state migrations can be explicit.

## Status

Accepted.
Supersedes ADR-0002.

## Considered Options

Keeping the OS keychain as the default was rejected because it makes agent-driven operation unreliable and annoying when the operating system repeatedly asks for password approval.
Keeping Keychain support only as a migration source was rejected because the product should have one normal Token Store model rather than a hidden compatibility path.

## Consequences

Existing OAuth App setup may be copied from `~/.config/goog/config.toml` to `~/.goog/config.toml` when the new config file is missing.
Existing users must run `goog auth login` again after this change if they only have Tokens in the old keychain-backed store.
`GOOG_TOKEN_FILE` remains useful as an explicit override for sandboxes, CI, and custom mounts, but it points to the same full auth state schema as `~/.goog/auth.json`.
Other CLI-local state should also live under `~/.goog/` unless there is a stronger reason to use an operating-system-specific config or data directory.
OAuth App setup remains separate from runtime auth state.
Commands that need an Account Token fail with an explicit `goog auth login` repair message when no Token exists.
They do not launch login automatically from ordinary read or write command paths.
`goog auth export` remains, but it writes the same versioned auth state schema as the Token Store and may filter that state to selected Accounts.

The initial auth state shape is:

```json
{
  "version": 1,
  "accounts": {
    "person@example.com": {
      "token": {
        "access_token": "...",
        "refresh_token": "...",
        "expiry": "...",
        "scopes": []
      }
    }
  },
  "active_account": "person@example.com",
  "resource_account_mappings": {}
}
```
