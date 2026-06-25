## Parent

Parent issue: #34

## What to build

Add high-level text write commands: `goog docs insert-text` and `goog docs replace-text`. These commands should use the shared Document Map, Location Selector, Document Range, Ambiguous Location, Dry Run Preview, Human Preview, and Revision Guard concepts from the earlier slices.

Text writes should apply immediately by default. `--dry-run` should simulate the edit locally, show before and after content around the target, and avoid calling the mutation endpoint. `--dry-run --json` should include the resolved Document Location or Document Range, revision ID, and native Batch Update request body that would be sent.

## Acceptance criteria

- [ ] `goog docs insert-text DOC_ID "text" --page PAGE --line LINE` inserts text at the resolved Document Location.
- [ ] `insert-text` supports exact `--index` targeting.
- [ ] `insert-text` supports heading and text anchors such as `--after-heading`, `--before-heading`, `--after-text`, and `--before-text`.
- [ ] `goog docs replace-text DOC_ID "old" "new"` replaces a single unambiguous match.
- [ ] `replace-text` rejects ambiguous matches unless the user passes `--match N` or `--all`.
- [ ] Ambiguous write failures print candidate Document Locations.
- [ ] `--dry-run` prints a Human Preview with before and after content and does not call the mutation endpoint.
- [ ] `--dry-run --json` emits the resolved target, revision ID, and native Batch Update request body.
- [ ] Non-dry-run writes call the mutation endpoint with the expected native Google request shape.
- [ ] `--required-revision-id` adds a Revision Guard to the native request body.
- [ ] Tests cover selector resolution, ambiguity errors, dry-run output, JSON dry-run output, and successful mutation requests.
- [ ] Existing raw `docs get` and `docs batch-update` behavior remains unchanged.

## Blocked by

- #35
- #36
