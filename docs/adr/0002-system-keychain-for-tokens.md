# System Keychain for OAuth Tokens

Status: Superseded by the File Token Store model that stores auth state in `~/.goog/auth.json`.
This ADR is retained as historical context for the earlier Keychain design.

OAuth Tokens (access + refresh) are stored in the OS system keychain via the `keyring` crate, not in config files. Non-sensitive config (active account name, OAuth client ID) lives in `~/.config/goog/config.toml`.
Interactive `goog` commands should not trigger a Keychain Access Prompt on every invocation after the user has approved the local CLI once.
The intended trust boundary is the local `goog` executable running under the current OS user account.
If the executable changes after installation or upgrade, one fresh OS prompt is acceptable.

## Considered Options

The simpler alternative -- storing tokens in a plaintext TOML file -- was rejected because refresh tokens are long-lived secrets. A token file readable by any process (or backed up to cloud storage without care) is a persistent credential leak. The system keychain is the OS-provided secret store and the right tool for this.
Using a plaintext token file for normal interactive use was rejected for the same reason, even if it would avoid Keychain Access Prompt friction.
Plaintext token files remain an explicit headless or sandbox escape hatch, not the default user path.

## Consequences

Headless or server environments without a system keychain (some CI runners, Docker containers) cannot store tokens in the default store. This is acceptable because `goog` targets interactive developer use, not server-side automation. Users who need headless access should use an explicit token export or a GCP service account.
`goog auth login` should establish smooth local Keychain access as part of login, rather than introducing a separate trust-repair command.
If that setup cannot guarantee prompt-free future reads, login should warn and continue after the Token is saved.
Repeated Keychain Access Prompts are a degraded local-secret-store experience, not a reason to reject a valid Google login.
Existing users who see repeated Keychain Access Prompts should run `goog auth login` again for the affected Account.
The first implementation target is macOS Keychain prompt behavior.
Linux and Windows should remain supported through the existing cross-platform secret-store abstraction, but they are not the source of this UX requirement.
The `keyring` crate is an implementation detail, not the architectural decision.
If it cannot express the required macOS trust behavior cleanly, `goog` may add a macOS-specific Token store behind the existing AccountStore boundary.
