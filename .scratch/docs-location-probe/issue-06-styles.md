## Parent

Parent issue: #34

## What to build

Add `goog docs apply-styles` for high-level style edits over Document Ranges. The command should support common shorthand flags and raw Google Docs style JSON through Style Payloads. It should use the shared Location Selector, Document Range, ambiguity, dry-run, JSON dry-run, and Revision Guard behavior.

By default, style commands should apply to the whole content block selected by a Location Selector. Exact ranges should be available through explicit start and end indexes. Text-span targeting should be available through text selectors, with ambiguity handling.

## Acceptance criteria

- [ ] `goog docs apply-styles DOC_ID --bold --page PAGE --line LINE` applies bold styling to the selected content block.
- [ ] Style shorthand supports common formatting such as bold, italic, font size, foreground color, and heading level.
- [ ] Raw Google Docs style JSON can be supplied as a Style Payload for advanced coverage.
- [ ] `apply-styles` supports explicit `--from-index` and `--to-index` Document Ranges.
- [ ] `apply-styles` supports text-span targeting and rejects ambiguous text spans unless disambiguated.
- [ ] `--dry-run` shows a Human Preview for the affected content and does not call the mutation endpoint.
- [ ] `--dry-run --json` emits the resolved Document Range, revision ID, Style Payload, and native Batch Update request body.
- [ ] `--required-revision-id` adds a Revision Guard to the native request body.
- [ ] Tests cover shorthand mapping, raw Style Payload preservation, block range targeting, exact range targeting, text-span ambiguity, and mutation request construction.
- [ ] Existing raw `docs get` and `docs batch-update` behavior remains unchanged.

## Blocked by

- #35
- #36
- #37
