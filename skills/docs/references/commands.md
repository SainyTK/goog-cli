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

Apply one numbering operation over the complete contiguous action range so all items share one native list:

```bash
target/debug/goog docs list-format apply DOCUMENT_ID --from-index START --to-index END --type numbered --dry-run --json
```

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
target/debug/goog docs copy --help
target/debug/goog docs export-pdf --help
target/debug/goog docs style copy-named --help
target/debug/goog docs style copy-page --help
```

Pass each command segment as its own shell argument.
Do not quote an entire command path such as `"docs text insert --help"`.
If a live command fails with a missing-scopes error, run `goog auth login` once, then re-run the original command.
