# End-to-end testing

Before writing regression unit tests for a change, verify it by actually running the `goog` binary against a real, already-connected Google account.
This catches wrong assumptions about the real Google API shape that a `wiremock`-backed unit test would happily encode and never question.

## Accounts

Every account already authorized on this machine (`goog auth list`) is available inside the sandbox -- there is no dedicated throwaway test account.
Treat every account as real, with real mail, real Drive files, real Docs, real Sheets.
An agent picks whichever account best fits the issue at hand, or the account the issue itself names.

## Credentials inside the Sandcastle sandbox

The Sandcastle implementer/reviewer run inside an ephemeral Docker container.
To make every already-authorized account usable inside that sandbox, a human does this once, on the host, outside of any agent run:

1. `goog auth export --out .sandcastle/secrets/auth.json` writes a full auth state file with Accounts, the Active Account, Tokens, and Resource Account Mappings.
   Re-run this any time the set of logged-in accounts or mappings changes.
2. Copy your host `$HOME/.goog/config.toml` to `.sandcastle/secrets/config.toml` so the sandbox has the OAuth App setup.

`.sandcastle/secrets/` is gitignored -- never remove that from `.sandcastle/.gitignore`, and never commit either file.
`main.mts` mounts this directory read-only into the implementer/reviewer sandbox and points `GOOG_TOKEN_FILE` at the copied auth state file.
Delete the files from `.sandcastle/secrets/` when E2E testing is no longer needed -- they grant whoever can read them access to every account they contain, within its authorized scopes.

If neither file has been set up, `goog` commands inside the sandbox fail with "account is not logged in" -- the implementer should treat that as "E2E testing isn't available yet" and fall back to unit tests only, noting this in its issue comment rather than blocking on it.

## Read vs. mutate

- **Reads are unrestricted.** `mail list/read`, `drive ls/download`, `docs map/get`, `sheets get`, and `sheets values get` may target any existing file, message, or thread in the account.
- **Mutations must only ever touch a resource this same test run created.** `drive upload`, `docs batch-update`, `sheets values update/append/clear`, `sheets batch-update` take a Document/Spreadsheet/File ID directly and will happily overwrite whatever ID you give them -- the CLI has no built-in safety net here.
  - Never pass an ID you discovered via a read/list/search command to a mutating command.
  - Only pass an ID that came back from a `drive upload` (or similar creating call) earlier in the same test run.
  - Name every created resource with a `goog-e2e-` prefix so it's identifiable later.
  - There is no `drive delete`, `docs create`, or `sheets create` command today. If a test needs to mutate a Doc or Sheet, create the underlying file via `drive upload` first and capture its returned ID -- never reuse an existing Doc/Sheet ID as a "scratch" target, even one that looks disposable.
  - Drive uploads have no delete path in the CLI yet, so scratch files accumulate. Reuse a single `goog-e2e-scratch-*` file per resource type across test runs instead of creating a new one every time.

## Evidence

For each issue, capture every `goog` invocation and its output in `.sandcastle/evidence/issue-<ID>-e2e.log`. Include the exact command line and the full stdout/stderr, in the order run. This is what a human reviewer checks instead of re-running the test themselves, so it needs to be legible on its own -- command, then result, repeated per step.

Issue branches (`sandcastle/issue-<ID>`) are never pushed to GitHub, so a link to the file on that branch is always a dead link. The merge step embeds the log's contents directly in the human-review comment instead -- keep it short enough to read comfortably inline (a handful of commands, not a full transcript dump).

Evidence must be reproducible.
Do not paste a raw transcript and expect the reviewer to infer what passed.
Write the log as a short checklist of commands, expected result, observed result, and pass/fail status.

Use this shape:

```text
Issue <ID> E2E evidence, redacted
Result: PASS|PARTIAL|FAIL

Test setup:
- Accounts used: [active-account], [account-2]
- Fixture used: [document-id], [spreadsheet-id], [message-id], or [scratch resource created by this run]
- Skipped fixture: none, or exact reason

Local checks:
$ cargo test <targeted filter>
Result: PASS.
<short count summary>

Live checks:
$ goog <exact command with redacted ids>
Expected: <what this command proves>
Observed: <short structural result, with sensitive content redacted>
Result: PASS|FAIL.

Reviewer notes:
- <anything the human must know before closing or sending back>
```

Prefer structural checks over dumping full API payloads.
For example, use `jq -r .id`, row counts, status codes, file sizes, or a redacted JSON projection rather than full message, document, or spreadsheet content.
If a command needs a fixture, name how the reviewer can obtain an equivalent fixture.
If a live fixture is unavailable, record the exact unavailable fixture and still run the local and command-surface checks that can run safely.

## Test steps for human review

Every issue handed to `need-human-review` must have a `## Test steps` section in the GitHub issue body before it is merged.
The implementer is responsible for adding or updating that section after the implementation is working.

The section should translate each acceptance criterion into concrete reviewer actions:

- exact local test commands to run;
- exact `goog` commands to run, using shell variables when IDs or accounts are environment-specific;
- how to choose or create each required live fixture;
- what output or state proves pass/fail;
- what may be skipped only when a fixture is unavailable;
- what evidence must be redacted before commenting.

Keep the steps issue-specific.
Do not write "run E2E tests" or "verify manually" without the actual commands and expected results.

## Redaction, before committing

`.sandcastle/evidence/` is a tracked directory, not `.sandcastle/logs/` -- anything written there enters git history. Before staging an evidence log, scrub it:

- Real email addresses (senders, recipients, CC) other than the account's own address -- replace with placeholders like `sender-1@redacted.example`.
- Message subjects, bodies, and snippets -- replace with a placeholder or a one-line structural description (e.g. `[subject redacted, 42 chars]`).
- Document, Sheet, and Drive file names and contents that aren't the `goog-e2e-` scratch resources this test created itself.
- Any token, refresh token, client secret, or API key. These should never appear in normal CLI output, but tool crashes/debug output can leak them -- check anyway.

What's safe to leave in: the command line itself, exit codes, HTTP status codes, resource IDs for `goog-e2e-` scratch resources, counts (e.g. "returned 12 messages"), and structural JSON shape with values placeholder'd.

If in doubt about whether a piece of output is sensitive, redact it -- the log only needs to prove the command worked, not reproduce its full output.

## Real identifiers never enter unit test source

The redaction rules above cover `.sandcastle/evidence/` logs. Unit test source (`src/**/*_tests.rs`) has a stricter rule: a real Document, Spreadsheet, Drive file, or message ID or URL from live E2E verification must never be committed there, full stop -- not even redacted-looking fragments of one.

It's fine to temporarily hardcode a real ID while iterating against the live account (for example, to confirm a URL-parsing fix against an actual Google Docs URL). Once the behavior is confirmed, replace that real ID with an obviously-fake placeholder (`placeholder-document-id`, `document-123`, and similar patterns already used throughout `src/*_tests.rs`) before committing. If the only thing a test would verify is already covered by a placeholder-based test elsewhere, delete the real-ID version outright instead of swapping in a placeholder.

## When E2E surfaces something out of scope

If running the real CLI turns up a bug unrelated to, or too large for, the current issue: don't fix it inline. Open a new GitHub issue describing what was observed, comment on the current issue linking the new one, and continue with the current issue's own scope.
