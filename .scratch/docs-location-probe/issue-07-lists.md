## Parent

Parent issue: #34

## What to build

Add `goog docs apply-list` for high-level list formatting over Document Ranges. The command should expose a small CLI List Style vocabulary for common list types and keep raw Google bullet presets available for less common list formats.

The command should share the Location Selector, Document Range, ambiguity, dry-run, JSON dry-run, and Revision Guard behavior used by the other high-level write commands.

## Acceptance criteria

- [ ] `goog docs apply-list DOC_ID --type bullet --page PAGE --line LINE` applies a bullet list to the selected content block.
- [ ] List Style supports `bullet`, `numbered`, `dash`, and `checkbox`.
- [ ] Raw Google bullet presets can be supplied for advanced list formats.
- [ ] `apply-list` supports explicit `--from-index` and `--to-index` Document Ranges.
- [ ] `apply-list` supports whole-block targeting through Location Selectors.
- [ ] Ambiguous selectors are rejected with candidate locations.
- [ ] `--dry-run` shows a Human Preview for the affected content and does not call the mutation endpoint.
- [ ] `--dry-run --json` emits the resolved Document Range, revision ID, list preset, and native Batch Update request body.
- [ ] `--required-revision-id` adds a Revision Guard to the native request body.
- [ ] Tests cover CLI List Style mapping, raw preset preservation, exact range targeting, block targeting, ambiguity handling, and mutation request construction.
- [ ] Existing raw `docs get` and `docs batch-update` behavior remains unchanged.

## Blocked by

- #35
- #36
- #37
