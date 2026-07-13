# Docs command guide

Use `target/debug/goog` in the repository examples below.
Replace it with `cargo run --` when the debug binary has not been built.

## Choose a creation path

Create a blank document when the design can be built from supported text, paragraph, list, table, image, header, footer, and page-layout operations:

```bash
target/debug/goog docs create "Quarterly operating review"
```

Copy a template when the source contains editor-only components such as a native table of contents, page-number auto text, positioned images, or first-page header content:

```bash
target/debug/goog docs copy SOURCE_DOCUMENT_ID "Quarterly operating review"
```

Both commands print the document ID and edit URL separated by a tab.
Capture both values from the output rather than guessing the URL.

Inspect the selected document before editing:

```bash
target/debug/goog docs get DOCUMENT_ID
target/debug/goog docs map DOCUMENT_ID
target/debug/goog docs map DOCUMENT_ID --json
```

## Audit a source document

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

## Define a native style system

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

## Insert and replace text

```bash
target/debug/goog docs text insert DOCUMENT_ID $'Executive summary\n' --at index:1 --dry-run --json
target/debug/goog docs text insert DOCUMENT_ID $'Decision\n' --at before-heading:Appendix
target/debug/goog docs text insert DOCUMENT_ID 'Approved' --at after-text:'Status: '
target/debug/goog docs text search DOCUMENT_ID 'revenue'
target/debug/goog docs text replace DOCUMENT_ID 'Draft' 'Final' --dry-run --json
```

Quote selectors and text containing spaces.
Use ANSI-C shell quoting for intentional newlines.

## Apply styles

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

## Internal navigation

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

## Images

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

## Headers and footers

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

## Page and section breaks

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

## Tables

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

## Lists

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

## Visual verification

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

## Learn unfamiliar commands

```bash
target/debug/goog docs --help
target/debug/goog docs text --help
target/debug/goog docs style --help
target/debug/goog docs style apply --help
target/debug/goog docs table --help
target/debug/goog docs image --help
target/debug/goog docs list-format apply --help
target/debug/goog docs header create --help
target/debug/goog docs footer create --help
target/debug/goog docs break page --help
target/debug/goog docs break section --help
target/debug/goog docs copy --help
target/debug/goog docs export-pdf --help
target/debug/goog docs style named --help
target/debug/goog docs style copy-named --help
target/debug/goog docs style copy-page --help
```

Pass each command segment as its own shell argument.
Do not quote an entire command path such as `"docs text insert --help"`.
If a live command fails with a missing-scopes error, run `goog auth login` once, then re-run the original command.
