# Incremental OAuth Scope Authorization

`goog auth login` requests only minimal scopes (profile, email). When a command first needs an API-specific scope (e.g., Drive access), it detects the missing scope and re-triggers the OAuth flow for that scope alone. Granted scopes are tracked per Account in the keychain entry.

## Considered Options

The alternative is requesting all possible scopes at login time. This was rejected because Google's consent screen lists every requested permission verbatim -- a wall of scary permissions causes users to abandon the flow. Incremental authorization produces a focused, per-API consent screen and means a user who only uses Drive never consents to Gmail access.

## Consequences

The Token entry in the keychain must store the set of granted scopes alongside the token values. Each API command must declare which scopes it requires and check for them before making API calls. A second browser prompt on first use of a new API is expected behavior, not a bug.
