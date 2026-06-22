# 03 - `goog auth login --no-browser` (device flow)

Status: ready-for-agent

## Parent

`.scratch/goog-cli-mvp/PRD.md`

## What to build

Add a `--no-browser` flag to `goog auth login` that switches from the loopback redirect to the device authorization grant (RFC 8628). When `--no-browser` is passed:
- Print the verification URL and the user code to the terminal.
- Poll Google's device token endpoint at the interval specified in the response until the user completes authorization on another device.
- On success, exchange the device grant for a Token and store it in the keychain exactly as the loopback flow does.
- On timeout or denial, surface a clear error message.

This slice shares all Token storage and account-confirmation behavior with the loopback flow - only the authorization step differs.

## Acceptance criteria

- [ ] `goog auth login --no-browser` prints a verification URL and user code without opening a browser.
- [ ] The CLI polls until the user completes authorization and then stores the Token.
- [ ] On success, the authorized email address is confirmed in the terminal.
- [ ] A denied or timed-out device grant produces a clear error on stderr with a non-zero exit code.
- [ ] The polling loop is tested against a `wiremock` mock of Google's device token endpoint, covering the pending state, the success state, and the access-denied state.

## Blocked by

- `02-auth-login-loopback-and-auth-list.md`
