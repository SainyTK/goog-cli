## Parent

Parent issue: #34

## What to build

Add high-level Document Map lookup commands: `goog docs search-text` and `goog docs get-content`. These commands should use the Document Map and Location Selector model from #35 rather than parsing raw Google Docs JSON independently.

`search-text` should find text matches and return Document Ranges with useful candidate locations. `get-content` should retrieve content by exact Google Docs index, Document Map Entry, page plus content line, or heading anchor. The command contract must keep `--index` reserved for raw Google Docs UTF-16 indexes and use `--entry` for Document Map Entry numbers.

## Acceptance criteria

- [ ] `goog docs search-text DOC_ID "text"` prints human-readable matches with match number, page, content line, index, confidence, and preview.
- [ ] `goog docs search-text DOC_ID "text" --json` emits structured Document Ranges and Document Locations.
- [ ] `goog docs get-content DOC_ID --index INDEX` retrieves content at an exact Google Docs index.
- [ ] `goog docs get-content DOC_ID --entry ENTRY` retrieves content by Document Map Entry number.
- [ ] `goog docs get-content DOC_ID --page PAGE --line LINE` retrieves content by derived page and content line.
- [ ] `goog docs get-content DOC_ID --heading "Heading text"` retrieves content by heading anchor.
- [ ] Ambiguous heading or text selectors return candidate locations rather than choosing silently.
- [ ] `--index` and `--entry` are tested as distinct concepts.
- [ ] Tests cover human-readable output and JSON output.
- [ ] Existing raw `docs get` and `docs batch-update` behavior remains unchanged.

## Blocked by

- #35
