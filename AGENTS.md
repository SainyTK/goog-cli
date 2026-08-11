## Project workflow

Develop and verify changes locally in the current checkout first.
Use normal Git branches and commits, then push and open a pull request when work is ready for human review.
Do not invoke the `no-mistakes` workflow unless the user explicitly asks for it.

## Domain docs

This is a single-context repository with one `CONTEXT.md` and architectural decisions under `docs/adr/`.
See `docs/agents/domain.md`.

## Verification

For bug fixes, reproduce the failure through the real CLI before changing code.
For new Google API behavior, verify the implementation against a live connected Google account before relying on regression unit tests.
Run targeted tests while iterating and the full Rust test suite before declaring a branch ready for review.
See `docs/agents/e2e-testing.md`.

Real Document, Spreadsheet, Drive, message, event, or presentation IDs and URLs discovered during live verification are local and temporary.
Never commit them into unit test source or repository documentation.
Use obvious placeholder IDs in committed tests.
