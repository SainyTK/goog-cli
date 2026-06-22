# System Keychain for OAuth Tokens

OAuth Tokens (access + refresh) are stored in the OS system keychain via the `keyring` crate, not in config files. Non-sensitive config (active account name, OAuth client ID) lives in `~/.config/goog/config.toml`.

## Considered Options

The simpler alternative -- storing tokens in a plaintext TOML file -- was rejected because refresh tokens are long-lived secrets. A token file readable by any process (or backed up to cloud storage without care) is a persistent credential leak. The system keychain is the OS-provided secret store and the right tool for this.

## Consequences

Headless or server environments without a system keychain (some CI runners, Docker containers) cannot store tokens. This is acceptable because `goog` targets interactive developer use, not server-side automation. Users who need headless access should use a GCP service account instead.
