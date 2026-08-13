# Google Docs operations

## Contents

- Required workflow
- Command selection
- Editing safety
- Document quality rules
- Completion gate
- Command guide
  - Choose a creation path
  - Audit a source document
  - Compare a source and target
  - Reuse observed document styles
  - Define a native style system
  - Configure page geometry
  - Protect multi-step edits with revision guards
  - Insert and replace text
  - Apply styles
  - Internal navigation
  - Images
  - Headers and footers
  - Page and section breaks
  - Tables
  - Lists
  - Visual verification
  - Learn unfamiliar commands

## Required workflow

1. Decide the document's purpose, audience, content structure, and visual hierarchy before writing.
2. Run `goog auth list` and record the active account before any live mutation.
3. Run `goog docs --help` and the relevant nested `--help` command before the first mutation.
4. Choose blank creation for a new design or template copying when editor-only components must be preserved.
5. Create or inspect the document.
6. Prefer high-level `goog docs` commands over `batch-update`.
7. Use dry runs for supported edits, then apply the confirmed operation.
8. Map the document again after structural edits because indexes and entry numbers can change.
9. Export the finished document as PDF and inspect every page from top to bottom at 100% zoom, or use an authenticated browser when export is unavailable.
10. Correct content, hierarchy, tables, spacing, page breaks, and formatting before delivery.
11. Repeat the available visual inspection path after the last layout-affecting edit.
12. Return the Google Docs URL and a short description of the finished result.

## Command selection

- Use `goog docs create` for a blank native document.
- Use `goog docs copy` when a source document contains native tables of contents, page-number auto text, positioned images, first-page headers, or other components that the Docs API cannot create.
- Use `goog docs compare` for source-to-target inventory, visual-system, formatting, and mapped-content acceptance checks.
- Use `goog docs get` for raw structure and revision metadata.
- Use `goog docs map` for readable content locations, headings, tables, and images.
- Use `goog docs text` for search, insertion, and replacement.
- Use `goog docs style apply` for text and paragraph styling.
- Use `goog docs style named` for defining a blank document's native style system.
- Use `goog docs style copy-named` and `copy-page` for transferring an existing document's visual system to a blank target.
- Use `goog docs table` for inserting and populating tables.
- Use `goog docs image`, `break`, `footnote`, `header`, `footer`, `list-format`, and `named-range` for their named document features.
- Use `goog docs export-pdf` for page-level visual QA.

Use `goog docs batch-update` only when no high-level command supports the required operation.
Keep any temporary JSON request body in task-local scratch space and remove it after verification.

## Editing safety

- Accept a document ID or full Google Docs URL wherever the command supports either.
- Use the active account shown by `goog auth list` unless the user specifies another authorized account.
- Pass `--account EMAIL` when account selection must remain explicit across a multi-step task.
- If authorization opens an account chooser, select only the recorded active or user-specified account.
- Never infer an account from browser order, a remembered identity, or unrelated open documents.
- If a command fails because the account is missing required scopes, run `goog auth login` once and retry.
- Use `goog auth list` for the auth preflight and do not list unrelated Drive resources.
- Inspect with `docs map` before targeting content.
- Prefer semantic selectors such as `heading:`, `after-heading:`, `before-text:`, or `--text` over raw indexes.
- Run `--dry-run --json` when the command supports it.
- Use `--required-revision-id` for multi-step or potentially concurrent edits.
- Capture revision IDs directly from command output and never retype or shorten them manually.
- Refresh the Document Map after any failed guarded write before retrying.
- Re-fetch the revision and remap after each structural change.
- Preserve the existing design during local edits unless the user asks for a redesign.
- Treat browser inspection as read-only QA and avoid editor keyboard shortcuts that could mutate content.
- Use an already authenticated native browser session for the recorded account when one is available.
- If no matching authenticated browser session is available, export the document as PDF and inspect the rendered pages instead.
- Treat a denied PDF export as a Workspace or file-policy limitation and retry with `--account EMAIL` only when account selection may be the cause.
- If both browser inspection and PDF export are unavailable, report visual QA as blocked instead of claiming the completion gate passed.
- Never expose scratch files, request bodies, or internal QA notes in the document.

## Document quality rules

### Plan the reading experience

Identify the intended reader and the decision, understanding, or action the document should enable.
Choose a form that matches the job, such as a memo, report, proposal, guide, checklist, or reference.
Build a clear progression from context to substance to outcome.

Use the lightest readable structure for each content type:

- Use prose for explanation and rationale.
- Use short bullets for independent factors or requirements.
- Use numbered steps for sequence.
- Use a table only for repeated records with shared fields.
- Use a brief callout for a decision, warning, or key takeaway.

### Typography and hierarchy

Use native paragraph styles for titles and headings.
Keep one title treatment, one body style, and a consistent heading ladder.
For a clean Google Docs default, use Arial or another common sans serif, black or near-black body text, a restrained blue accent, and comfortable line spacing.
Avoid decorative rules, excessive color, underlining for emphasis, and too many type sizes.
Keep paragraphs short enough to scan without turning the document into disconnected fragments.

### Tables and lists

Give short fields narrow columns and explanatory fields more room.
Keep header labels concise.
Avoid sentence-length prose in most cells.
Use consistent alignment and number formatting within each column.
Use native list formatting so wrapped lines align with the item text.

### Final inspection

Read the complete document in order.
Open the native document at 100% zoom and inspect every page from top to bottom.
Check page edges for clipped content, accidental overflow, stranded list items, and unwanted blank pages.
Check for abrupt transitions, repeated conclusions, orphan headings, empty sections, inconsistent capitalization, and placeholder copy.
Inspect every table for missing values, broken row structure, and unreadable density.
Confirm the title, headings, body, lists, and callouts use a coherent visual system.
Re-fetch the live document after the last mutation and verify that all expected text and structure are present.

## Completion gate

- The requested content is complete and factually consistent.
- Heading levels form a sensible hierarchy.
- Tables are used for real row-and-column comparisons, not as generic page layout.
- Lists use native list formatting.
- No placeholder, instruction text, or duplicated content remains.
- Styles are consistent and readable.
- The native document or its exported PDF has been visually inspected at 100% zoom, including every page edge and any requested page-count limit.
- When reproducing a source document, `goog docs compare SOURCE TARGET --fail-on-difference` has checked component inventory, named and page styles, formatting, and mapped content properties after generated IDs are removed.
- When copying a reviewed template, `goog docs copy` pins both the running executable SHA-256 and the reviewed source revision ID, then uses `--verify-fidelity` to gate the completed copy against the source.
- Retain the final JSON acceptance record with the document identities, edit URLs, account, executable identity, revisions, replay command, and matched fidelity fingerprints.
- The comparison evidence records its stable report type and schema version.
- Every reported difference is understood and intentional.
- The live document has been fetched again after the last write.
- The final URL opens the intended native Google Doc.

## Command guide

Use `target/debug/goog` in the repository examples below.
Replace it with `cargo run --` when the debug binary has not been built.

### Choose a creation path

Create a blank document when the design can be built from supported text, paragraph, list, table, image, header, footer, and page-layout operations:

```bash
target/debug/goog docs create "Quarterly operating review"
```

Copy a template when the source contains editor-only components such as a native table of contents, page-number auto text, positioned images, or first-page header content:

```bash
target/debug/goog docs copy SOURCE_DOCUMENT_ID "Quarterly operating review"
target/debug/goog docs copy SOURCE_DOCUMENT_ID "Quarterly operating review" --required-executable-sha256 SHA256 --required-source-revision-id REVISION_ID --verify-fidelity --json
```

Use `--required-executable-sha256` and `--required-source-revision-id` after inspecting a live template to prevent copying with a changed CLI binary or from a newer, unreviewed source revision.
Use `--verify-fidelity` to capture and validate the source before copying, then compare the completed copy with that exact snapshot across inventory, visual-system, formatting, and content scopes before accepting it.
The verified comparison replay command pins the resolved account used to read both the source and copied documents.
The final JSON acceptance record identifies both document IDs, Google-confirmed titles, and edit URLs, the resolved account, goog CLI version, execution OS and architecture, running executable path and SHA-256, when acceptance occurred, the exact source and copied revisions, the revision-pinned fidelity replay command, and the four matched scope fingerprints that passed verification.
Use `--json` to emit the comparison and typed copy acceptance records as newline-delimited JSON.
Verified acceptance records identify the comparison report type, schema, and SHA-256 fingerprint algorithm so retained scope fingerprints remain interpretable.
When fidelity verification fails, the command reports the differences but withholds its tab-separated or `goog.docs.copy.acceptance` success record so automation cannot treat the rejected copy as accepted.

Without `--json`, both commands print the document ID and edit URL separated by a tab.
Capture both values from the output rather than guessing the URL.

Inspect the selected document before editing:

```bash
target/debug/goog docs get DOCUMENT_ID
target/debug/goog docs map DOCUMENT_ID
target/debug/goog docs map DOCUMENT_ID --json
```

### Audit a source document

Before reproducing an existing document, inventory every mapped component instead of sampling only its visible body text:

```bash
target/debug/goog docs map SOURCE_DOCUMENT_ID --json | jq '{entriesByKind: (.entries | group_by(.kind) | map({kind: .[0].kind, count: length})), lists: (.lists | length), breaks: (.breaks | length), segments: (.segments | length), blankParagraphs: (.blankParagraphs | length), documentLocations: (.documentLocations | length), namedStyles: (.namedStyles | length), documentStyles: (.documentStyles | length)}'
```

The entry summary distinguishes headings, paragraphs, tables, inline images, positioned images, and native tables of contents.
The separate arrays cover native lists, explicit page and section breaks, header and footer segments, blank paragraphs, and tab-scoped page and named-style metadata.
Use the focused maps to inspect the components that need to be reproduced:

```bash
target/debug/goog docs map SOURCE_DOCUMENT_ID --type tables --json
target/debug/goog docs map SOURCE_DOCUMENT_ID --type images --json
target/debug/goog docs map SOURCE_DOCUMENT_ID --type lists --json
target/debug/goog docs map SOURCE_DOCUMENT_ID --type breaks --json
target/debug/goog docs map SOURCE_DOCUMENT_ID --type segments --json
```

Inspect `paragraphStyle`, `textRuns`, table-cell paragraphs and runs, image geometry, break section styles, segment auto text, named styles, and document styles rather than comparing text counts alone.
Use the raw tab-aware document only when the map exposes a component whose source metadata needs deeper inspection:

```bash
target/debug/goog docs get SOURCE_DOCUMENT_ID --include-tabs-content
```

Classify each source component before choosing the creation path.
Build supported components with high-level commands, copy the source as a template for editor-only components, and record any policy-limited visual verification separately from structural fidelity.

### Compare a source and target

When reproducing an existing document, compare fresh source and target maps after the last write.
Use the high-level comparison command for the complete structural acceptance check:

```bash
target/debug/goog docs compare SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID
target/debug/goog docs compare SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID --json
target/debug/goog docs compare SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID --fail-on-difference
target/debug/goog docs compare SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID --json --max-differences 0
target/debug/goog docs compare SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID --json --summary-only --fail-on-difference
target/debug/goog docs compare SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID --max-differences 100
target/debug/goog docs compare SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID --scope formatting --difference-pattern '/entries/*/paragraphStyle/alignment' --max-differences 100
target/debug/goog docs compare SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID --scope visual-system --fail-on-difference
target/debug/goog docs compare SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID --scope formatting --fail-on-difference
target/debug/goog docs compare SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID --account EMAIL --fail-on-difference
target/debug/goog docs compare SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID --source-account SOURCE_EMAIL --target-account TARGET_EMAIL --fail-on-difference
```

The command compares component inventory, the native named-style and page-style visual system, formatting and layout, and all mapped content and component properties.
Each report records its stable report type, schema version, a revision-guarded replay command, the UTC comparison time, the goog CLI version, execution OS and architecture, executable path and SHA-256, the selected scope, acceptance-gate setting, preview limit, summary mode, an explicit account override when supplied, and identifies the source and target by title, document ID, edit URL, revision ID, and the account that accessed it.
JSON reports expose the replay command as an argument array so the recorded executable path, pattern filters, and other values can be reused without ambiguous shell parsing.
Use `reportType` to identify `goog.docs.compare` evidence before interpreting `reportSchemaVersion` or any report-specific fields.
Use `reportSchemaVersion` to reject or migrate report shapes that an acceptance-evidence consumer does not understand.
The replay command starts with the recorded absolute goog executable path and includes `--required-executable-sha256`, `--required-source-revision-id`, and `--required-target-revision-id`, so it rejects a rebuilt binary at the same path or a later document state.
When the comparison uses `--account EMAIL`, the replay command preserves it so multi-account routing cannot silently select a different account.
When both documents resolve through the same account, the replay command pins that resolved account automatically.
When they resolve through different accounts, the replay command pins each one with `--source-account` and `--target-account`.
Keep the report type, schema version, replay command, timestamp, CLI version, execution OS and architecture, executable path and SHA-256, and revision IDs with acceptance evidence so a later document edit, tool change, execution environment change, or informational comparison cannot be mistaken for the tested acceptance gate.
It removes Google-assigned object, heading, segment, and list IDs before comparison while retaining tab IDs, indexes, ranges, and component order.
The visual-system scope also normalizes equivalent default fields that Google can materialize after copying styles, including false bold and page-break values and a page-number start of one.
An overall match proves semantic structural equivalence across all four scopes.
It does not replace page-level visual inspection.
Use `--fail-on-difference` in acceptance scripts that must return a nonzero status when any scope differs.
Use `--scope inventory`, `--scope visual-system`, `--scope formatting`, or `--scope content` when the target intentionally differs in other scopes.
For example, a blank-document recreation with different prose can independently gate its copied native named styles and page layout with `--scope visual-system`.
Use `--scope formatting` to compare paragraph and text styles, table and image geometry, lists, breaks, and header or footer presentation without requiring identical prose, indexes, alt text, or generated map handles.

When the command reports a difference, it includes complete path-pattern counts, one representative source-target example for every pattern, and up to 20 JSON Pointer paths per scope by default.
Array indexes become `*` in patterns, so repeated formatting gaps remain visible even when an earlier component consumes the raw-path preview.
The per-pattern example remains available when `--max-differences` limits the general path preview.
Set `--max-differences` to a larger or smaller number when a different diagnostic preview is useful.
Set it to zero for compact automation output with no raw path previews.
Complete pattern counts, representative examples, fingerprints, aggregate totals, and acceptance behavior remain available.
Use `--summary-only` when automation needs only fingerprints, aggregate counts, and the acceptance result.
This suppresses both mismatch patterns and raw path previews without changing `--fail-on-difference` behavior.
Copy a reported pattern into `--difference-pattern` to show concrete paths only for that pattern while retaining the complete pattern summary, total difference count, and acceptance result.
Filtered human-readable reports count additional paths that match the selected pattern separately from differences outside the filter.
Human-readable reports end with aggregate counts for total, displayed, and limit-hidden differences, and filtered reports also summarize the matching and out-of-filter split.
JSON reports expose the emitted, limit-truncated, and summary-suppressed path counts for every scope through `displayedDifferenceCount`, `differenceCountHiddenByLimit`, and `differenceCountHiddenBySummary`.
The report-level `totalDifferenceCount`, `totalDisplayedDifferenceCount`, `totalDifferenceCountHiddenByLimit`, and `totalDifferenceCountHiddenBySummary` fields provide the corresponding totals across all selected scopes.
Filtered JSON reports also expose the matching and out-of-filter split through `previewDifferenceCount` and `differenceCountOutsidePreview` for every selected scope, plus `totalPreviewDifferenceCount` and `totalDifferenceCountOutsidePreview` at report level.
The command rejects a pattern that is not present in the selected scope and suggests the closest reported patterns, so a typo cannot look like a successful empty drill-down.
Use the manual map comparisons below when those paths need more context.
Generate the same compact inventory for each document:

```bash
target/debug/goog docs map SOURCE_DOCUMENT_ID --json | jq '{entriesByKind: (.entries | group_by(.kind) | map({kind: .[0].kind, count: length})), lists: (.lists | length), breaks: (.breaks | length), segments: (.segments | length), blankParagraphs: (.blankParagraphs | length), namedStyleTabs: (.namedStyles | length), documentStyleTabs: (.documentStyles | length)}' > /tmp/source-doc-inventory.json
target/debug/goog docs map TARGET_DOCUMENT_ID --json | jq '{entriesByKind: (.entries | group_by(.kind) | map({kind: .[0].kind, count: length})), lists: (.lists | length), breaks: (.breaks | length), segments: (.segments | length), blankParagraphs: (.blankParagraphs | length), namedStyleTabs: (.namedStyles | length), documentStyleTabs: (.documentStyles | length)}' > /tmp/target-doc-inventory.json
diff -u /tmp/source-doc-inventory.json /tmp/target-doc-inventory.json
```

An empty diff is useful for a template copy, but counts alone do not prove fidelity.
Compare the focused table, image, list, break, and segment maps when those components affect the design.
For a blank-document recreation, explain intentional count differences and verify the properties that matter, including named styles, page geometry, paragraph and text-run styles, table geometry, image sizes, and header or footer content.
Do not compare generated object IDs, heading IDs, segment IDs, or revision IDs because Google assigns those independently.
Complete the comparison with page-level visual inspection of the target PDF or native document.
Remove the temporary inventory files after verification.

Compare the native visual systems directly after the inventory check.
Canonicalize named styles by removing generated heading IDs, and canonicalize page styles by removing generated header and footer segment IDs:

```bash
target/debug/goog docs map SOURCE_DOCUMENT_ID --json > /tmp/source-doc-map.json
target/debug/goog docs map TARGET_DOCUMENT_ID --json > /tmp/target-doc-map.json

jq '[.namedStyles[] | {tabId, styles: [.namedStyles.styles[] | del(.paragraphStyle.headingId)]}] | sort_by(.tabId)' /tmp/source-doc-map.json > /tmp/source-doc-named-styles.json
jq '[.namedStyles[] | {tabId, styles: [.namedStyles.styles[] | del(.paragraphStyle.headingId)]}] | sort_by(.tabId)' /tmp/target-doc-map.json > /tmp/target-doc-named-styles.json
diff -u /tmp/source-doc-named-styles.json /tmp/target-doc-named-styles.json

jq '[.documentStyles[] | {tabId, documentStyle: (.documentStyle | del(.defaultHeaderId, .defaultFooterId, .firstPageHeaderId, .firstPageFooterId, .evenPageHeaderId, .evenPageFooterId))}] | sort_by(.tabId)' /tmp/source-doc-map.json > /tmp/source-doc-page-styles.json
jq '[.documentStyles[] | {tabId, documentStyle: (.documentStyle | del(.defaultHeaderId, .defaultFooterId, .firstPageHeaderId, .firstPageFooterId, .evenPageHeaderId, .evenPageFooterId))}] | sort_by(.tabId)' /tmp/target-doc-map.json > /tmp/target-doc-page-styles.json
diff -u /tmp/source-doc-page-styles.json /tmp/target-doc-page-styles.json
```

An empty named-style diff proves that source-defined title, heading, subtitle, and body typography and paragraph properties match.
An empty page-style diff proves that document mode, page geometry, margins, and supported header and footer behavior match.
These semantic comparisons deliberately retain tab IDs because a multi-tab reproduction should preserve the intended tab-level style assignment.
Remove all six temporary map and style files after verification.

Compare mapped content and component properties after the visual-system check.
Keep content order, ranges, tab assignment, text and paragraph styles, table geometry, image geometry, list formatting, breaks, and header or footer content while removing IDs that Google can assign independently:

```bash
jq '{entries, blankParagraphs, lists, breaks, segments, documentLocations} | walk(if type == "object" then del(.objectId, .headingId, .segmentId, .listId, .defaultHeaderId, .defaultFooterId, .firstPageHeaderId, .firstPageFooterId, .evenPageHeaderId, .evenPageFooterId) else . end) | walk(if type == "object" and (.heading? | type) == "object" then .heading |= del(.id) else . end)' /tmp/source-doc-map.json > /tmp/source-doc-content.json
jq '{entries, blankParagraphs, lists, breaks, segments, documentLocations} | walk(if type == "object" then del(.objectId, .headingId, .segmentId, .listId, .defaultHeaderId, .defaultFooterId, .firstPageHeaderId, .firstPageFooterId, .evenPageHeaderId, .evenPageFooterId) else . end) | walk(if type == "object" and (.heading? | type) == "object" then .heading |= del(.id) else . end)' /tmp/target-doc-map.json > /tmp/target-doc-content.json
diff -u /tmp/source-doc-content.json /tmp/target-doc-content.json
```

An empty content diff proves that every mapped body, table, image, list, break, and header or footer property matches in document order.
For a blank-document recreation, review each difference and record whether it is intentional, unsupported by the Docs API, or a defect to correct.
Do not remove tab IDs, indexes, ranges, or component order because differences in those fields can reveal misplaced content or a changed document structure.
Remove the two temporary content files with the other comparison files after verification.

For a blank target that should inherit an existing visual system, preview and then copy its named styles and page layout:

```bash
target/debug/goog docs style copy-named SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID --dry-run
target/debug/goog docs style copy-named SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID
target/debug/goog docs style copy-page SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID --dry-run
target/debug/goog docs style copy-page SOURCE_DOCUMENT_ID TARGET_DOCUMENT_ID
```

Named-style copying transfers the native title, subtitle, heading, and normal-text definitions.
Page copying transfers document mode, page dimensions, margins, and supported first-page or even-page header and footer behavior.
It does not create source header and footer segments in a blank target.

### Reuse observed document styles

A complete document fetch refreshes a local style template for that document:

```bash
target/debug/goog docs get DOCUMENT_ID > /tmp/document.json
target/debug/goog docs style template DOCUMENT_ID --json
```

The template records observed named-style text and paragraph properties, the first suitable table's header and body treatment, and the document's native list preset.
The cache is keyed by document ID and supports consistent edits within that document.
It is not a cross-document style-copy mechanism.
Use `copy-named`, `copy-page`, or `docs copy` when a new document must inherit a source design.

After the full fetch, applying a named paragraph style also reuses the cached text and paragraph properties that were observed for that style:

```bash
target/debug/goog docs style apply DOCUMENT_ID --text 'Executive summary' --paragraph-style HEADING_1 --dry-run --json
target/debug/goog docs style apply DOCUMENT_ID --text 'Executive summary' --paragraph-style HEADING_1
```

Explicit style flags override the corresponding cached values.
Pass `--no-cached-style` when the edit should apply only the properties named on the command line.

List formatting can reuse the cached native list preset when neither `--type` nor `--preset` is supplied:

```bash
target/debug/goog docs list-format apply DOCUMENT_ID --from-index LIST_START_INDEX --to-index LIST_END_INDEX --dry-run --json
target/debug/goog docs list-format apply DOCUMENT_ID --from-index LIST_START_INDEX --to-index LIST_END_INDEX
```

Populated table insertion automatically reuses the cached header-row treatment when one is available.
Pass `docs table insert --no-auto-style` to preserve Google Docs defaults for the new table.
Run another complete `docs get` after deliberate design changes so later edits do not use stale observations.
A partial fetch with `--fields` does not replace the cached template.

### Define a native style system

When a blank document needs a new visual system instead of one copied from a source, define its native styles before authoring the body.
Preview each named-style update before applying it:

```bash
target/debug/goog docs style named DOCUMENT_ID HEADING_1 --style-json '{"textStyle":{"weightedFontFamily":{"fontFamily":"Bai Jamjuree"},"fontSize":{"magnitude":20,"unit":"PT"},"foregroundColor":{"color":{"rgbColor":{"red":0.85,"green":0.33,"blue":0.10}}}},"paragraphStyle":{"spaceAbove":{"magnitude":14,"unit":"PT"},"spaceBelow":{"magnitude":6,"unit":"PT"},"keepWithNext":true,"keepLinesTogether":true}}' --dry-run --json
target/debug/goog docs style named DOCUMENT_ID HEADING_1 --style-json '{"textStyle":{"weightedFontFamily":{"fontFamily":"Bai Jamjuree"},"fontSize":{"magnitude":20,"unit":"PT"},"foregroundColor":{"color":{"rgbColor":{"red":0.85,"green":0.33,"blue":0.10}}}},"paragraphStyle":{"spaceAbove":{"magnitude":14,"unit":"PT"},"spaceBelow":{"magnitude":6,"unit":"PT"},"keepWithNext":true,"keepLinesTogether":true}}'
target/debug/goog docs map DOCUMENT_ID --json
```

`--style-json` uses native Google Docs `textStyle` and `paragraphStyle` objects.
Select the style with the `NAMED_STYLE` argument and do not include the read-only `paragraphStyle.namedStyleType` field in the JSON.
The supported styles are `NORMAL_TEXT`, `TITLE`, `SUBTITLE`, and `HEADING_1` through `HEADING_6`.
Repeat the operation only for styles the document will use, then inspect the matching entry in the map's tab-scoped `namedStyles` metadata.
Use `--tab-id` for a non-default document tab and `--required-revision-id` when concurrent edits are possible.
After defining the style system, apply the native paragraph style to content with `docs style apply --paragraph-style` so later changes propagate consistently.
Use `copy-named` instead when an approved source document already contains the intended visual system.

### Configure page geometry

Set page dimensions and margins before adding layout-sensitive content to a blank document.
Preview the complete geometry first, then apply the same values:

```bash
target/debug/goog docs style page DOCUMENT_ID --page-width 612 --page-height 792 --margin-top 72 --margin-bottom 72 --margin-left 72 --margin-right 72 --margin-header 36 --margin-footer 36 --dry-run --json
target/debug/goog docs style page DOCUMENT_ID --page-width 612 --page-height 792 --margin-top 72 --margin-bottom 72 --margin-left 72 --margin-right 72 --margin-header 36 --margin-footer 36
target/debug/goog docs map DOCUMENT_ID --json
```

The example configures US Letter pages with one-inch body margins and half-inch header and footer margins.
Page width and height are measured in points and must be supplied together.
Margins can be changed independently when the existing page size should remain unchanged.
Use the map's tab-scoped `documentStyles` metadata to verify the stored page size, margins, and document mode.
Use `--required-revision-id` when concurrent edits are possible.
Use `copy-page` instead when the target must inherit an approved source document's page mode, geometry, margins, and supported header or footer behavior.

### Protect multi-step edits with revision guards

Capture the current revision immediately before a related sequence of edits:

```bash
REVISION_ID="$(target/debug/goog docs get DOCUMENT_ID --fields revisionId | jq -r '.revisionId')"
test -n "$REVISION_ID" && test "$REVISION_ID" != null
```

Pass that exact value to both the preview and the confirmed write:

```bash
target/debug/goog docs text insert DOCUMENT_ID $'Executive summary\n' --at index:1 --required-revision-id "$REVISION_ID" --dry-run --json
target/debug/goog docs text insert DOCUMENT_ID $'Executive summary\n' --at index:1 --required-revision-id "$REVISION_ID"
```

The confirmed write fails instead of overwriting concurrent changes when the live document no longer has that revision.
Fetch a new revision and remap the document after every successful structural edit because later indexes and entry numbers may have moved.
If a guarded write fails, discard the old revision and location, then fetch and map again before rebuilding the edit.
Never shorten, retype, or reuse a revision ID from an earlier editing session.

### Insert and replace text

```bash
target/debug/goog docs text insert DOCUMENT_ID $'Executive summary\n' --at index:1 --dry-run --json
target/debug/goog docs text insert DOCUMENT_ID $'Decision\n' --at before-heading:Appendix
target/debug/goog docs text insert DOCUMENT_ID 'Approved' --at after-text:'Status: '
target/debug/goog docs text search DOCUMENT_ID 'revenue'
target/debug/goog docs text replace DOCUMENT_ID 'Draft' 'Final' --dry-run --json
```

Quote selectors and text containing spaces.
Use ANSI-C shell quoting for intentional newlines.

### Apply styles

Build the document hierarchy with native paragraph styles, then add only the explicit typography and layout properties the design requires:

```bash
target/debug/goog docs style apply DOCUMENT_ID --text 'Quarterly operating review' --paragraph-style TITLE --font-family 'Bai Jamjuree' --font-size 26 --foreground-color '#202124' --alignment center --space-below 10 --dry-run --json
target/debug/goog docs style apply DOCUMENT_ID --text 'Quarterly operating review' --paragraph-style TITLE --font-family 'Bai Jamjuree' --font-size 26 --foreground-color '#202124' --alignment center --space-below 10
target/debug/goog docs style apply DOCUMENT_ID --text 'Executive summary' --paragraph-style HEADING_1 --keep-with-next --keep-lines-together --dry-run --json
target/debug/goog docs style apply DOCUMENT_ID --text 'Executive summary' --paragraph-style HEADING_1 --keep-with-next --keep-lines-together
target/debug/goog docs style apply DOCUMENT_ID --entry BODY_ENTRY --font-family 'Bai Jamjuree' --font-size 11 --alignment justified --direction left-to-right --line-spacing 115 --space-below 10 --spacing-mode never-collapse --dry-run --json
target/debug/goog docs style apply DOCUMENT_ID --entry BODY_ENTRY --font-family 'Bai Jamjuree' --font-size 11 --alignment justified --direction left-to-right --line-spacing 115 --space-below 10 --spacing-mode never-collapse
```

Use `--entry` after mapping when the same text appears more than once or when the complete paragraph must receive spacing and pagination properties.
Use `--text` for a unique text span and `--match N` for an intentional repeated match.
Paragraph layout options also include `--space-above`, `--indent-start`, `--indent-end`, `--indent-first-line`, `--avoid-widow-and-orphan`, and `--page-break-before`.
Text styling also supports `--bold`, `--italic`, `--underline`, and internal heading links through `--link-heading-id`.
Run `target/debug/goog docs style apply --help` before using a paragraph style that has not already been observed in the document.

Map again after applying styles and inspect both `paragraphStyle` and `textRuns` in JSON output:

```bash
target/debug/goog docs map DOCUMENT_ID --json
```

The customer reference uses explicit paragraph spacing, line spacing, alignment, indentation, direction, custom fonts, and pagination controls.
Do not rely on visual similarity alone when these native properties can be verified through the map.

### Internal navigation

Map the document after the heading structure is final and copy the target heading's native `headingId` from the JSON output:

```bash
target/debug/goog docs map DOCUMENT_ID --json
```

Insert the navigation label as normal text, then preview and apply an internal heading link over that exact text:

```bash
target/debug/goog docs text insert DOCUMENT_ID $'Executive summary\n' --at after-heading:'Contents' --dry-run --json
target/debug/goog docs text insert DOCUMENT_ID $'Executive summary\n' --at after-heading:'Contents'
target/debug/goog docs style apply DOCUMENT_ID --text 'Executive summary' --link-heading-id TARGET_HEADING_ID --dry-run --json
target/debug/goog docs style apply DOCUMENT_ID --text 'Executive summary' --link-heading-id TARGET_HEADING_ID
```

Use `--entry` or `--match N` when the label also appears elsewhere in the document.
Google applies native link color and underline styling to linked text automatically.
Map again and verify that the selected text run contains the expected `headingId` link and that the target heading retains the same ID.

The Docs API cannot create a native table of contents.
Copy a template when the document requires an editor-managed table of contents with page numbers and automatic entry updates.
Use heading links to build a manual navigation list only when automatic TOC behavior is unnecessary.

### Images

Insert body images from a publicly reachable URI.
Provide both dimensions in points when the layout needs a predictable image footprint:

```bash
target/debug/goog docs image insert DOCUMENT_ID 'https://example.com/report-chart.png' --at after-heading:'Key metrics' --width 360 --height 203 --dry-run --json
target/debug/goog docs image insert DOCUMENT_ID 'https://example.com/report-chart.png' --at after-heading:'Key metrics' --width 360 --height 203
target/debug/goog docs map DOCUMENT_ID --type images --json
```

Google treats the requested width and height as a bounding box and preserves the source image's aspect ratio.
Use the mapped native size for layout verification instead of assuming both requested dimensions were stored exactly.
The image belongs to its containing paragraph, so map again after insertion and style the returned image entry when the paragraph needs deliberate alignment or spacing:

```bash
target/debug/goog docs style apply DOCUMENT_ID --entry IMAGE_ENTRY --alignment center --space-above 6 --space-below 6 --dry-run --json
target/debug/goog docs style apply DOCUMENT_ID --entry IMAGE_ENTRY --alignment center --space-above 6 --space-below 6
```

Header and footer images require a segment ID from `docs map --type segments --json`, `docs get`, or a header or footer creation result:

```bash
target/debug/goog docs image insert DOCUMENT_ID 'https://example.com/company-mark.png' --segment-id HEADER_SEGMENT_ID --width 72 --height 24 --dry-run --json
target/debug/goog docs image insert DOCUMENT_ID 'https://example.com/company-mark.png' --segment-id HEADER_SEGMENT_ID --width 72 --height 24
```

Inline image insertion cannot create positioned or floating images.
Copy a template when those editor-only image components are required.

### Headers and footers

Create and populate the default header and footer for the first section, previewing each write before applying it:

```bash
target/debug/goog docs header create DOCUMENT_ID --text 'Customer delivery report' --dry-run --json
target/debug/goog docs header create DOCUMENT_ID --text 'Customer delivery report'
target/debug/goog docs footer create DOCUMENT_ID --text 'Confidential' --dry-run --json
target/debug/goog docs footer create DOCUMENT_ID --text 'Confidential'
target/debug/goog docs map DOCUMENT_ID --type segments --json
```

The create responses return a header ID or footer ID.
Use the segment map to confirm the editable range because header and footer indexes begin at zero and are separate from body indexes.
Apply text and paragraph formatting with that segment ID and the mapped range:

```bash
target/debug/goog docs style apply DOCUMENT_ID --segment-id HEADER_SEGMENT_ID --from-index 0 --to-index HEADER_END_INDEX --font-family 'Bai Jamjuree' --font-size 10 --foreground-color '#666666' --alignment end --dry-run --json
target/debug/goog docs style apply DOCUMENT_ID --segment-id HEADER_SEGMENT_ID --from-index 0 --to-index HEADER_END_INDEX --font-family 'Bai Jamjuree' --font-size 10 --foreground-color '#666666' --alignment end
```

For a later section, insert the section break, map again to get its actual start index, then target that index when creating the new segments:

```bash
target/debug/goog docs break section DOCUMENT_ID --at before-heading:'Appendix' --section-type next-page --dry-run --json
target/debug/goog docs break section DOCUMENT_ID --at before-heading:'Appendix' --section-type next-page
target/debug/goog docs map DOCUMENT_ID --type breaks --json
target/debug/goog docs header create DOCUMENT_ID --section-break-index SECTION_BREAK_INDEX --text 'Appendix' --dry-run --json
target/debug/goog docs header create DOCUMENT_ID --section-break-index SECTION_BREAK_INDEX --text 'Appendix'
target/debug/goog docs footer create DOCUMENT_ID --section-break-index SECTION_BREAK_INDEX --text 'Confidential' --dry-run --json
target/debug/goog docs footer create DOCUMENT_ID --section-break-index SECTION_BREAK_INDEX --text 'Confidential'
```

The Docs API cannot create first-page header content or page-number auto text in a blank document.
Copy a template when those editor-only components are required.

### Page and section breaks

Use an explicit page break when the following content must begin on a new page without creating a new section:

```bash
target/debug/goog docs break page DOCUMENT_ID --at before-heading:'Appendix' --dry-run --json
target/debug/goog docs break page DOCUMENT_ID --at before-heading:'Appendix'
target/debug/goog docs map DOCUMENT_ID --type breaks --json
```

Use a section break when later content needs independent headers, footers, or section formatting:

```bash
target/debug/goog docs break section DOCUMENT_ID --at before-heading:'Appendix' --section-type next-page --dry-run --json
target/debug/goog docs break section DOCUMENT_ID --at before-heading:'Appendix' --section-type next-page
target/debug/goog docs map DOCUMENT_ID --type breaks --json
```

`--section-type next-page` starts the section on a new page, while `continuous` starts it at the selected location without forcing a new page.
Google can insert a newline while creating a section break, so use the remapped break index for later header or footer creation instead of reusing the requested insertion index.
Use `--page-break-before` in `docs style apply` when the page boundary is an intentional property of a paragraph style rather than a standalone document element.
Map after every break insertion because all following body indexes and entry locations can change.

### Tables

```bash
target/debug/goog docs style apply DOCUMENT_ID --text 'Key metrics' --paragraph-style HEADING_1
target/debug/goog docs table insert DOCUMENT_ID --at after-heading:'Key metrics' --data metrics.csv --dry-run --json
target/debug/goog docs map DOCUMENT_ID --type tables --json
target/debug/goog docs table edit DOCUMENT_ID --table-id table-1 --data metrics.csv --dry-run --json
```

Create CSV or TSV data with one row per table row and one field per cell.
Use either `--data FILE` or `--rows N --columns N`, never both.
Style a text anchor as a heading before using `after-heading:` or `before-heading:`.
Map again after insertion to obtain the actual table handle.

For a report table that needs deliberate geometry and a repeating header, preview each operation before applying it:

```bash
target/debug/goog docs table columns DOCUMENT_ID --table-id table-1 --widths 120,348 --dry-run --json
target/debug/goog docs table columns DOCUMENT_ID --table-id table-1 --widths 120,348
target/debug/goog docs table header-rows DOCUMENT_ID --table-id table-1 --rows 1 --dry-run --json
target/debug/goog docs table header-rows DOCUMENT_ID --table-id table-1 --rows 1
target/debug/goog docs table style DOCUMENT_ID --table-id table-1 --row 1 --background-color '#D9EAF7' --dry-run --json
target/debug/goog docs table style DOCUMENT_ID --table-id table-1 --row 1 --background-color '#D9EAF7'
```

Column widths are comma-separated points and must match the table's column count.
Row and column arguments are one-based.
Omit `--column` to style a complete row, or include it to target one cell.
Cell styling also supports `--content-alignment top|middle|bottom` and paired `--border-color` plus `--border-width` controls.
Map the document again after table changes and inspect the resulting `layoutMetadata`, `pinnedHeaderRowsCount`, and cell styles.

### Lists

Insert list items as separate paragraphs, then apply one formatting operation over the complete contiguous range so the items share one native list:

```bash
target/debug/goog docs text insert DOCUMENT_ID $'Confirm scope\nCollect evidence\nRecord the decision\n' --at after-heading:'Next steps' --dry-run --json
target/debug/goog docs text insert DOCUMENT_ID $'Confirm scope\nCollect evidence\nRecord the decision\n' --at after-heading:'Next steps'
target/debug/goog docs map DOCUMENT_ID --json
target/debug/goog docs list-format apply DOCUMENT_ID --from-index LIST_START_INDEX --to-index LIST_END_INDEX --type numbered --dry-run --json
target/debug/goog docs list-format apply DOCUMENT_ID --from-index LIST_START_INDEX --to-index LIST_END_INDEX --type numbered
target/debug/goog docs map DOCUMENT_ID --type lists --json
```

The supported shorthand types are `bullet`, `numbered`, `dash`, and `checkbox`.
Use `--preset` instead of `--type` when an existing document requires a specific raw Google Docs bullet preset.
Do not pass both options.

For a nested list, put one leading tab on each second-level paragraph and two leading tabs on each third-level paragraph before applying list formatting to the complete range:

```bash
target/debug/goog docs text insert DOCUMENT_ID $'Prepare delivery\n\tReview content\n\tReview layout\nPublish\n' --at after-heading:'Delivery plan' --dry-run --json
target/debug/goog docs text insert DOCUMENT_ID $'Prepare delivery\n\tReview content\n\tReview layout\nPublish\n' --at after-heading:'Delivery plan'
```

Google removes those leading tabs when it creates the native list and uses them to determine nesting levels.
Map again after formatting and inspect the list's item count, nesting levels, and glyph metadata.

### Visual verification

Export the completed native document and inspect every rendered page at 100% zoom:

```bash
target/debug/goog docs export-pdf DOCUMENT_ID --output ./quarterly-operating-review.pdf
```

Re-export after the last layout-affecting edit.
If export is denied, confirm the account explicitly before treating it as a Workspace or file-policy restriction:

```bash
target/debug/goog docs export-pdf DOCUMENT_ID --output ./quarterly-operating-review.pdf --account alice@example.com
```

Template copying preserves source components, but it does not bypass restrictions on downloading, printing, or copying.
Use an authenticated browser for visual inspection when export remains unavailable.
If neither path is available, report visual QA as blocked.

### Learn unfamiliar commands

```bash
target/debug/goog docs --help
target/debug/goog docs text --help
target/debug/goog docs style --help
target/debug/goog docs style apply --help
target/debug/goog docs style template --help
target/debug/goog docs table --help
target/debug/goog docs image --help
target/debug/goog docs list-format apply --help
target/debug/goog docs header create --help
target/debug/goog docs footer create --help
target/debug/goog docs break page --help
target/debug/goog docs break section --help
target/debug/goog docs copy --help
target/debug/goog docs compare --help
target/debug/goog docs export-pdf --help
target/debug/goog docs style named --help
target/debug/goog docs style page --help
target/debug/goog docs style copy-named --help
target/debug/goog docs style copy-page --help
```

Pass each command segment as its own shell argument.
Do not quote an entire command path such as `"docs text insert --help"`.
If a live command fails with a missing-scopes error, run `goog auth login` once, then re-run the original command.
