# End-to-end testing

Before writing regression unit tests for a change, verify it by actually running the `goog` binary against a real, already-connected Google account.
This catches wrong assumptions about the real Google API shape that a `wiremock`-backed unit test would happily encode and never question.

## Accounts

Every account already authorized on this machine (`goog auth list`) is available inside the sandbox -- there is no dedicated throwaway test account.
Treat every account as real, with real mail, real Drive files, real Docs, real Sheets.
An agent picks whichever account best fits the issue at hand, or the account the issue itself names.

## Credentials inside the Sandcastle sandbox

The Sandcastle implementer/reviewer run inside an ephemeral Docker container that has no access to this machine's OS keychain, which is where `goog` normally reads tokens from. To make every already-authorized account usable inside that sandbox, a human does this once, on the host, outside of any agent run:

1. `goog auth export --out .sandcastle/secrets/token.json` -- pulls every authorized account's token out of the keychain into one file, keyed by email. Re-run this any time the set of logged-in accounts changes; it overwrites the file with the current keychain state.
2. Copy your host `config.toml` to `.sandcastle/secrets/config.toml` as-is -- it already lists every authorized account and the OAuth App config, which is exactly what the sandbox needs to resolve `--account` the same way it does on the host. The host path depends on OS (`dirs::config_dir()`): `~/Library/Application Support/goog/config.toml` on macOS, `~/.config/goog/config.toml` on Linux. The sandbox is always Linux, so the copy always lands at `~/.config/goog/config.toml` inside the container regardless of the host's own path.

`.sandcastle/secrets/` is gitignored -- never remove that from `.sandcastle/.gitignore`, and never commit either file. `main.mts` mounts this directory read-only into the implementer/reviewer sandbox and points `GOOG_TOKEN_FILE` at the copied token file, so `goog` inside the sandbox reads from that file instead of the (nonexistent, in-container) keychain. Delete the files from `.sandcastle/secrets/` when E2E testing is no longer needed -- they grant whoever can read them full access to every account they contain, within its authorized scopes.

If neither file has been set up, `goog` commands inside the sandbox fail with "account is not logged in" -- the implementer should treat that as "E2E testing isn't available yet" and fall back to unit tests only, noting this in its issue comment rather than blocking on it.

## Read vs. mutate

- **Reads are unrestricted.** `mail list/search/read`, `drive ls/list/download`, `docs map/search-text/get-content/get`, `sheets get/batch-get` may target any existing file, message, or thread in the account.
- **Mutations must only ever touch a resource this same test run created.** `drive upload`, `docs batch-update`, `sheets values update/append/clear/batch-update`, `sheets batch-update` take a Document/Spreadsheet/File ID directly and will happily overwrite whatever ID you give them -- the CLI has no built-in safety net here.
  - Never pass an ID you discovered via a read/list/search command to a mutating command.
  - Only pass an ID that came back from a `drive upload` (or similar creating call) earlier in the same test run.
  - Name every created resource with a `goog-e2e-` prefix so it's identifiable later.
  - There is no `drive delete`, `docs create`, or `sheets create` command today. If a test needs to mutate a Doc or Sheet, create the underlying file via `drive upload` first and capture its returned ID -- never reuse an existing Doc/Sheet ID as a "scratch" target, even one that looks disposable.
  - Drive uploads have no delete path in the CLI yet, so scratch files accumulate. Reuse a single `goog-e2e-scratch-*` file per resource type across test runs instead of creating a new one every time.

## Evidence

For each issue, capture every `goog` invocation and its output in `.sandcastle/evidence/issue-<ID>-e2e.log`. Include the exact command line and the full stdout/stderr, in the order run. This is what a human reviewer checks instead of re-running the test themselves, so it needs to be legible on its own -- command, then result, repeated per step.

Issue branches (`sandcastle/issue-<ID>`) are never pushed to GitHub, so a link to the file on that branch is always a dead link. The merge step embeds the log's contents directly in the human-review comment instead -- keep it short enough to read comfortably inline (a handful of commands, not a full transcript dump).

## Redaction, before committing

`.sandcastle/evidence/` is a tracked directory, not `.sandcastle/logs/` -- anything written there enters git history. Before staging an evidence log, scrub it:

- Real email addresses (senders, recipients, CC) other than the account's own address -- replace with placeholders like `sender-1@redacted.example`.
- Message subjects, bodies, and snippets -- replace with a placeholder or a one-line structural description (e.g. `[subject redacted, 42 chars]`).
- Document, Sheet, and Drive file names and contents that aren't the `goog-e2e-` scratch resources this test created itself.
- Any token, refresh token, client secret, or API key. These should never appear in normal CLI output, but tool crashes/debug output can leak them -- check anyway.

What's safe to leave in: the command line itself, exit codes, HTTP status codes, resource IDs for `goog-e2e-` scratch resources, counts (e.g. "returned 12 messages"), and structural JSON shape with values placeholder'd.

If in doubt about whether a piece of output is sensitive, redact it -- the log only needs to prove the command worked, not reproduce its full output.

## When E2E surfaces something out of scope

If running the real CLI turns up a bug unrelated to, or too large for, the current issue: don't fix it inline. Open a new GitHub issue describing what was observed, comment on the current issue linking the new one, and continue with the current issue's own scope.
