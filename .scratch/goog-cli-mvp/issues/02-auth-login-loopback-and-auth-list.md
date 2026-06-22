# 02 - `goog auth login` (loopback) + `goog auth list`

Status: ready-for-agent

## Parent

`.scratch/goog-cli-mvp/PRD.md`

## What to build

Implement the full loopback OAuth flow in `goog-auth` and wire it to the `goog auth login` command. On login, bind a temporary HTTP server to a random localhost port, construct Google's OAuth consent URL with that port as the redirect URI, open the browser automatically, capture the authorization code from the incoming redirect request, exchange the code for a Token (access token, refresh token, expiry, granted scopes), and store the Token in the OS system keychain via the `keyring` crate, namespaced by the Account's email address.

After a successful login, print the authorized Account's email address as confirmation.

Implement `goog auth list` to read all stored Accounts from the keychain and display them in a table, marking the Active Account. `--json` emits newline-delimited JSON.

The Token stored in the keychain must include: access token, refresh token, expiry timestamp, and the set of granted Scopes.

## Acceptance criteria

- [ ] `goog auth login` opens the browser to Google's consent screen.
- [ ] The authorization code is captured automatically via the localhost redirect - no manual paste required.
- [ ] On success, the Token is stored in the system keychain under a key namespaced by the Account email.
- [ ] `goog auth login` prints the authorized email address on success.
- [ ] Running `goog auth login` twice with different accounts adds both to the keychain.
- [ ] `goog auth list` shows all logged-in Accounts in a table with the Active Account marked.
- [ ] `goog auth list --json` emits newline-delimited JSON with account email and active status.
- [ ] The loopback server's token-exchange logic is tested against a `wiremock` mock of Google's token endpoint (no real credentials required in CI).

## Blocked by

- `01-workspace-skeleton-and-auth-setup.md`
