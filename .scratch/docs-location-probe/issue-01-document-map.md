## Parent

Parent issue: #34

## What to build

Add the first vertical slice of the high-level Docs editing surface: a shared Document Map read model and `goog docs map`. This command should fetch a Google Docs Document, normalize its editable content into Document Map Entries, and print a human-readable map by default while supporting full JSON output for scripts.

The map should expose Document Locations with stable Google Docs indexes, derived page labels, content lines, kind, style, preview text, Location Confidence, and the current revision ID. Page labels should use explicit page break evidence and table-of-contents heading evidence where available. Content line means a top-level content block within a derived page, not a rendered visual line.

## Acceptance criteria

- [ ] `goog docs map DOC_ID` prints a human-readable table of Document Map Entries.
- [ ] `goog docs map DOC_ID --json` emits structured JSON containing revision ID, entries, Document Locations, and Location Confidence.
- [ ] Entries include stable Google Docs indexes where available, derived page labels where evidence exists, content lines, kind, style, and preview text.
- [ ] Page labels can be derived from explicit page breaks.
- [ ] Table-of-contents heading evidence can assign page labels to matching headings.
- [ ] Missing page evidence produces unknown confidence rather than fabricated precision.
- [ ] Content line is counted as a top-level content block within a derived page.
- [ ] Tests cover a short document shape with manual page breaks and a table of contents.
- [ ] Tests cover a long human-written document shape with headings, Thai text, tables, Inline Images, Positioned Images, table-of-contents page hints, and sparse explicit page breaks.
- [ ] Existing raw `docs get` and `docs batch-update` behavior remains unchanged.

## Blocked by

None - can start immediately
