## Problem Statement

`goog docs batch-update` gives strong Google Docs API coverage, but it is too hard to use for normal document edits. Users need deep familiarity with Google Docs raw document JSON, UTF-16 indexes, structural elements, request bodies, and style payloads before they can insert text, find content, edit tables, add images, or apply formatting.

The current raw surface is still valuable as a last-mile escape hatch, but it does not provide the day-to-day editing workflow that users expect from a CLI. The CLI needs a higher-level Docs editing layer that lets users navigate a Document, resolve human-readable Document Locations, preview edits, reject ambiguous targets, and then apply common changes without hand-authoring native `documents.batchUpdate` JSON.

## Solution

Add High-Level Docs Commands backed by a shared Document Map. The Document Map is the read model for locating content blocks, headings, tables, Inline Images, Positioned Images, and text matches. Every Document Map Entry exposes a stable Google Docs index plus derived labels such as page, content line, kind, style, preview text, and Location Confidence.

High-level read commands should default to human-readable tables and support `--json` for scripts. High-level write commands should use the same Location Selector grammar, apply immediately by default, support `--dry-run`, reject Ambiguous Locations, and expose optional Revision Guards.

The raw `goog docs get` and `goog docs batch-update` commands remain available. They are the low-level API coverage surface for advanced edits that the high-level commands do not cover.

## User Stories

1. As a CLI user, I want to run `goog docs map DOC_ID`, so that I can see a readable navigation map of a Google Docs Document.
2. As a CLI user, I want each map row to show index, page, content line, kind, style, Location Confidence, and preview text, so that I can choose an edit target without reading raw JSON.
3. As a CLI user, I want `docs map --page 5`, so that I can focus on content likely to be on a specific derived page.
4. As a CLI user, I want `docs map --kind heading`, so that I can inspect the document structure quickly.
5. As a script author, I want `docs map --json`, so that I can consume Document Locations and Document Map Entries programmatically.
6. As a CLI user, I want page labels to be derived from explicit page breaks and table-of-contents heading evidence, so that page-oriented navigation is useful on real documents.
7. As a CLI user, I want Location Confidence in map output, so that I know whether a page label is exact, inferred, or unavailable.
8. As a CLI user, I want content line to mean a top-level content block within a derived page, so that it is stable and inspectable.
9. As a CLI user, I do not want content line to mean a rendered wrapped visual line, so that the CLI does not promise Google-rendered layout data it cannot reliably obtain.
10. As a CLI user, I want `goog docs search-text DOC_ID "text"`, so that I can find every matching Document Range.
11. As a CLI user, I want search results to include match number, page, content line, index, confidence, and preview, so that I can choose the right occurrence.
12. As a script author, I want search results in JSON, so that I can pipe exact ranges into follow-up commands.
13. As a CLI user, I want `goog docs get-content DOC_ID --index 9853`, so that I can inspect content at an exact Google Docs index.
14. As a CLI user, I want `goog docs get-content DOC_ID --page 5 --line 1`, so that I can inspect content using a human-readable Location Selector.
15. As a CLI user, I want `goog docs get-content DOC_ID --entry 5`, so that I can inspect the fifth Document Map Entry without confusing it with a Google Docs index.
16. As a CLI user, I want `goog docs get-content DOC_ID --heading "Heading text"`, so that I can inspect content by heading anchor.
17. As a CLI user, I want `goog docs list-images DOC_ID`, so that I can see every image-like object in the Document.
18. As a CLI user, I want image output to distinguish Inline Images from Positioned Images, so that I know which images can be edited through the first high-level image commands.
19. As a CLI user, I want Inline Images to include Document Locations, so that I can insert nearby content or inspect their placement.
20. As a CLI user, I want Positioned Images to include object IDs and layout metadata, so that I can identify them even when they are not normal text-flow content.
21. As a CLI user, I want `goog docs list-tables DOC_ID`, so that I can see every table with a user-facing Table Handle.
22. As a CLI user, I want each table listing to include rows, columns, index, derived page, content line, confidence, and preview, so that I can choose the correct table.
23. As a CLI user, I want Table Handles such as `table-3`, so that I do not need to copy raw indexes for common table edits.
24. As a CLI user, I want Table Handles to be re-resolved from the latest Document Map on each command, so that commands do not depend on local cache state.
25. As a CLI user, I want `goog docs insert-text DOC_ID "text" --page 5 --line 3`, so that I can insert text at a readable location.
26. As a CLI user, I want `insert-text` to support `--index`, so that I can target the exact Google Docs index when needed.
27. As a CLI user, I want `insert-text` to support heading and text anchors such as `--after-heading`, `--before-heading`, `--after-text`, and `--before-text`, so that I can target edits by document content.
28. As a CLI user, I want `goog docs replace-text DOC_ID "old" "new"`, so that I can replace a known text span without writing a Batch Update payload.
29. As a CLI user, I want `replace-text` to reject ambiguous matches unless I pass `--match N` or `--all`, so that I do not accidentally replace the wrong occurrence.
30. As a CLI user, I want high-level write commands to fail on Ambiguous Locations, so that the CLI never silently edits the wrong place.
31. As a CLI user, I want ambiguous write failures to print candidate Document Locations, so that I can rerun with a narrower selector.
32. As a CLI user, I want `--dry-run` on write commands, so that I can inspect the resolved location and simulated change before applying it.
33. As a CLI user, I want Dry Run Preview output to be human-readable by default, so that I can quickly judge whether the edit is correct.
34. As a script author, I want `--dry-run --json`, so that I can inspect the native Batch Update request and resolved metadata.
35. As a CLI user, I want Dry Run Preview for text edits to show before and after content, so that I can verify the insertion or replacement locally.
36. As a CLI user, I want Dry Run Preview for image insertion to show a placeholder in context, so that I can verify target placement without requiring Google-rendered layout.
37. As a CLI user, I want Dry Run Preview for table insertion or table edits to show a compact table preview, so that I can verify the data shape before applying it.
38. As a CLI user, I want `goog docs insert-image DOC_ID path.png --page 5 --line 3`, so that I can add an Inline Image through a simple command.
39. As a CLI user, I want image insertion to use the shared Location Selector grammar, so that it behaves like text insertion.
40. As a CLI user, I want `goog docs insert-table DOC_ID --data table.csv --page 5 --line 3`, so that I can insert a table from rectangular Table Data.
41. As a CLI user, I want table insertion to accept CSV or TSV input, so that I can use common spreadsheet exports.
42. As a CLI user, I want `goog docs edit-table DOC_ID --table-id table-3 --data table.csv`, so that I can replace table cell text without hand-writing table mutation requests.
43. As a CLI user, I want full-table edits to fail when Table Data dimensions do not match the existing table, so that accidental row or column changes do not happen silently.
44. As a CLI user, I want full-table edits to allow dimension changes only when I pass `--resize`, so that destructive structural changes are explicit.
45. As a CLI user, I want `goog docs apply-styles DOC_ID --bold --page 5 --line 3`, so that I can apply common formatting without raw JSON.
46. As a CLI user, I want style commands to support shorthand flags such as bold, italic, font size, foreground color, and heading level, so that common formatting is readable.
47. As a CLI user, I want style commands to accept raw Google Docs style JSON, so that advanced style coverage remains possible.
48. As a CLI user, I want style commands to default to the whole content block selected by the Location Selector, so that simple block-level formatting is easy.
49. As a CLI user, I want style commands to support explicit `--from-index` and `--to-index`, so that exact Document Ranges remain possible.
50. As a CLI user, I want style commands to support text-span targeting, so that I can format only matched text inside a block.
51. As a CLI user, I want `goog docs apply-list DOC_ID --type bullet --page 5 --line 3`, so that I can create common lists without choosing Google bullet presets.
52. As a CLI user, I want list commands to support list types such as bullet, numbered, dash, and checkbox, so that common list styles are memorable.
53. As a CLI user, I want list commands to support raw Google presets, so that uncommon list formats remain available.
54. As a script author, I want `docs map --json` to include the current revision ID, so that I can perform guarded writes.
55. As a script author, I want high-level write commands to accept `--required-revision-id`, so that I can avoid applying edits to a changed Document.
56. As a casual CLI user, I want high-level write commands not to require Revision Guards by default, so that simple edits stay simple.
57. As a Docs power user, I want raw `docs batch-update` to remain available, so that I can perform advanced API operations outside the high-level command surface.
58. As an implementer, I want all high-level Docs commands to share one Document Map and Location Selector model, so that behavior is consistent and testable.
59. As an implementer, I want read commands to be filtered views over the Document Map, so that search, image listing, table listing, and content lookup do not invent separate location concepts.
60. As an implementer, I want write commands to emit native Batch Update bodies during dry run, so that users can see and debug the exact Google API mutation.

## Implementation Decisions

- Preserve the existing raw Docs surface. `docs get` and `docs batch-update` remain available and continue to expose native Google API JSON.
- Add a Document Map read model for high-level commands. It should normalize Google Docs body content, tab-aware content, structural elements, headings, tables, Inline Images, Positioned Images, and text previews.
- Derive Document Locations from the Document Map. Every Document Location must include a stable Google Docs index when available.
- Treat page and content line as derived labels. Content line means top-level content block within a derived page, not rendered visual line after wrapping.
- Include Location Confidence in derived locations. Initial confidence values should cover explicit page breaks, table-of-contents heading evidence, section-derived or structural inference, and unknown.
- Use `index` only for raw Google Docs UTF-16 indexes. Use `entry` for Document Map Entry numbers.
- Add high-level read commands: `map`, `search-text`, `get-content`, `list-images`, and `list-tables`.
- Read commands default to human-readable table output. `--json` emits full structured metadata.
- Add high-level write commands: `insert-text`, `replace-text`, `insert-image`, `insert-table`, `edit-table`, `apply-styles`, and `apply-list`.
- Write commands apply immediately by default.
- Write commands support `--dry-run`. Human Preview is the default dry-run format. `--json` emits the resolved Document Location, Document Range when relevant, native Batch Update request body, revision ID, and preview metadata.
- Write commands reject Ambiguous Locations. They must return candidate locations and require disambiguation through a narrower selector, `--match N`, `--all` where appropriate, or exact index targeting.
- Support one shared Location Selector grammar across high-level write commands: exact index, page plus content line, Document Map Entry, heading anchors, and text anchors.
- Support one shared Document Range model for style and list commands: explicit index range, whole selected content block, or text spans from search.
- Add optional Revision Guards to high-level write commands through `--required-revision-id`.
- Table Handles are assigned by current document order and re-resolved from the latest Document Map on each command.
- `list-tables` emits Table Handles, dimensions, location metadata, confidence, and compact previews.
- `edit-table` initially performs full-table cell text replacement from Table Data. It requires matching dimensions unless `--resize` is supplied.
- Prefer `--data` for table CSV or TSV input. Avoid `--range` for full-table data because it sounds like a cell selector.
- `list-images` includes both Inline Images and Positioned Images.
- High-level `insert-image` creates Inline Images first. Positioned Image editing is later work.
- Style Payloads support shorthand flags for common formatting and raw Google style JSON for full coverage.
- List Style uses a small CLI vocabulary for common lists and keeps raw Google presets available.
- Batch Update remains the last-mile escape hatch for advanced or unsupported Google API features.

## Testing Decisions

- Test at the highest useful seam: the Docs command execution layer with a mocked Google Docs API and captured stdout/stderr. This should cover CLI behavior, request construction, ambiguity errors, dry-run output, and JSON output without live Google dependencies.
- Keep lower-level unit tests for pure Document Map and Location Selector behavior, because those rules are shared by every high-level command.
- Use sanitized fixtures derived from two real document shapes explored during design: a short generated document with manual page breaks and a table of contents, and a long human-written document with Thai text, many headings, tables, Inline Images, Positioned Images, table-of-contents page hints, and sparse explicit page breaks.
- Test that Document Map output includes indexes, derived page labels, content lines, Location Confidence, kind, style, and previews.
- Test that page labels are useful when explicit page breaks exist.
- Test that table-of-contents heading evidence can assign page labels to headings even when automatic page breaks are not exposed as body elements.
- Test that missing page evidence produces unknown confidence instead of fabricated precision.
- Test that `search-text` returns multiple matches with candidate locations.
- Test that write commands reject ambiguous targets and show candidates.
- Test that exact `--index` targeting bypasses ambiguity.
- Test that `--entry` targets a Document Map Entry and is distinct from `--index`.
- Test that dry-run text insertion and replacement show before and after previews and do not call the mutation endpoint.
- Test that dry-run JSON includes the native Batch Update body and revision ID.
- Test that non-dry-run writes call the mutation endpoint with the expected Google request shape.
- Test that Revision Guards are included in native request bodies when supplied.
- Test that table listing emits document-order Table Handles and that table editing re-resolves handles from the latest fetched document.
- Test that full-table replacement rejects mismatched dimensions unless resize is explicitly requested.
- Test that image listing distinguishes Inline Images from Positioned Images.
- Test that image insertion creates inline image requests at the resolved location.
- Test that style shorthand maps to the expected Google style request.
- Test that raw Style Payload JSON is preserved.
- Test that list types map to the expected Google bullet presets.
- Use existing CLI parse tests as prior art for command shape and help text.
- Use existing command tests as prior art for stdout JSON, mocked HTTP endpoints, account scope behavior, and error context.
- Use existing Docs client tests as prior art for readonly and write scopes, request URLs, request bodies, and error mapping.

## Out of Scope

- Removing or deprecating raw `docs batch-update`.
- Exact Google-rendered page layout reconstruction.
- Treating content line as a rendered visual wrapped line.
- Guaranteeing complete automatic page break detection from the Docs API.
- Local cache state for Document Map Entries, Table Handles, or Image Handles.
- A browser or Google-rendered visual preview for dry runs.
- Positioned Image editing in the first implementation.
- Partial table cell-range editing in the first implementation.
- Semantic table patching by cell coordinate in the first implementation.
- Full support for every Google Docs request type through high-level commands.
- Changing Drive discovery or file lifecycle behavior.

## Further Notes

- Future work should add stronger page evidence if a reliable rendered layout source becomes available, such as export-based layout probing or another Google-supported render signal.
- Future work should add Positioned Image editing once the inline image path is stable.
- Future work should add partial table edits such as targeting a cell, targeting a rectangular cell range, and patching from a CSV or TSV origin cell.
- Future work should add table targeting by nearby heading, nearby text, or table preview text.
- Future work should add richer style shorthands after the first Style Payload slice proves stable.
- Future work should add more list formats if the initial bullet, numbered, dash, checkbox, and raw preset vocabulary is insufficient.
- Future work should consider a command that exports or saves a Document Map snapshot for human review, but high-level commands should not depend on local cache state.
- Future work should consider optional live-document smoke tests gated behind environment configuration. Unit and command tests should use sanitized fixtures and must not hardcode private document IDs.
- Future work should consider richer dry-run diff output for long edits, large tables, and multi-match replacements.
- Future work should consider converting a dry-run Batch Update body into a directly reusable `docs batch-update --requests` file for advanced debugging.
