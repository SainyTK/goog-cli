## Project workflow

GnHF is the default workflow for substantial autonomous implementation.
Give it a durable objective or a repository plan, let it work in its own branch and worktree, and return to a pushed branch that is ready for human review.
See `docs/agents/gnhf-workflow.md`.

GitHub Issues and Sandcastle are not part of this project's workflow.
Use pull requests for reviewing and merging completed work.
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
