# 05 - `AuthClient` with token refresh middleware

Status: ready-for-agent

## Parent

`.scratch/goog-cli-mvp/PRD.md`

## What to build

Implement `AuthClient` in `goog-auth` - an auth-aware wrapper around `reqwest::Client` that all API crates receive and use for every HTTP call. `AuthClient` owns the Token lifecycle so that API crates never touch auth logic directly.

`AuthClient` must:
- Attach `Authorization: Bearer <access_token>` to every outgoing request.
- Before each request, check the Token's expiry. If it is within a configurable threshold (e.g., 60 seconds), proactively refresh it using the refresh token and update the keychain entry.
- If a request returns a 401, refresh the Token once and retry the request. A second 401 after refresh is a terminal error.
- If the refresh token exchange fails (revoked or expired), surface `AuthError::TokenRevoked` with a message prompting the user to run `goog auth login` again.
- Resolve which Account to use from the Active Account in config, overridable by the `--account` flag passed in at call time.

## Acceptance criteria

- [ ] Every request made through `AuthClient` carries a `Authorization: Bearer` header.
- [ ] A Token within the expiry threshold is refreshed before the request is sent (tested against a `wiremock` mock of Google's token endpoint).
- [ ] A 401 response triggers exactly one refresh-and-retry; a second 401 returns an error without further retries.
- [ ] A failed refresh due to a revoked token returns `AuthError::TokenRevoked`, not a generic HTTP error.
- [ ] Token refresh updates the keychain entry with the new access token and expiry.
- [ ] All behaviors above are covered by `wiremock`-based tests with no real credentials.

## Blocked by

- `02-auth-login-loopback-and-auth-list.md`
