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

```bash
target/debug/goog docs style apply DOCUMENT_ID --text 'Quarterly operating review' --paragraph-style TITLE --font-size 26 --foreground-color '#202124' --dry-run
target/debug/goog docs style apply DOCUMENT_ID --text 'Executive summary' --paragraph-style HEADING_1 --dry-run
target/debug/goog docs style apply DOCUMENT_ID --text 'Decision required' --bold --foreground-color '#174EA6' --dry-run
```

Run `target/debug/goog docs style apply --help` before using a paragraph style that has not already been observed in the document.

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
target/debug/goog docs table --help
target/debug/goog docs image --help
target/debug/goog docs list-format apply --help
target/debug/goog docs header create --help
target/debug/goog docs footer create --help
target/debug/goog docs break section --help
target/debug/goog docs copy --help
target/debug/goog docs export-pdf --help
target/debug/goog docs style copy-named --help
target/debug/goog docs style copy-page --help
```

Pass each command segment as its own shell argument.
Do not quote an entire command path such as `"docs text insert --help"`.
If a live command fails with a missing-scopes error, run `goog auth login` once, then re-run the original command.
