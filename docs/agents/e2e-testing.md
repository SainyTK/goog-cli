# End-to-end testing

Verify changes through the real `goog` CLI against an already-connected Google account before treating mocked tests as proof of real API behavior.
For a bug fix, reproduce the reported failure before changing code.
For a new command or behavior, exercise the smallest live path that proves the implementation works.

## Accounts and credentials

Every account shown by `goog auth list` is real and may contain sensitive data.
Use the account named by the objective when one is specified.
Otherwise, choose the account and resource that minimize risk.

GnHF runs on the host or in its own git worktree, so it uses the normal local `goog` authentication state.
Do not copy tokens or OAuth configuration into the repository.
Never commit credentials, command output containing credentials, or local authentication files.

## Reads and mutations

Read commands may inspect existing resources when the test requires them, but their content is still private.
Prefer structural observations such as result counts, status codes, field presence, and resource type over recording content.

Mutating tests must only change a resource created for testing or explicitly supplied by the user for that purpose.
Never mutate a resource merely because a list or search command discovered it.
Prefix created resources with `goog-e2e-` so they are recognizable later.
Delete temporary resources after the test when the CLI supports deletion.
When deletion is unavailable, reuse a clearly named scratch resource instead of creating one on every run.

## Evidence for pull requests

Summarize live verification in the pull request without committing raw transcripts.
Include the command surface tested, the expected behavior, the observed structural result, and whether it passed.
Redact account addresses, message content, document titles, file names, IDs, URLs, and other private values.

Do not add a tracked evidence directory.
Local GnHF logs under `.gnhf/runs/` are disposable and ignored by git.

## Regression tests

After live behavior is understood, add focused regression tests using mocks or local fixtures.
Use obvious placeholders such as `document-123` or `placeholder-spreadsheet-id` wherever an identifier is needed.
Real Document, Spreadsheet, Drive, message, event, calendar, or presentation IDs and URLs must never appear in test source, snapshots, fixture files, or documentation.

Run targeted tests while iterating.
Before a branch is ready for human review, run:

```sh
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features
cargo test
```

If a live dependency is unavailable, report the exact verification that could not run.
Do not describe mocked coverage as a substitute for a live check.
