# Docs command guide

Use `target/debug/goog` in the repository examples below.
Replace it with `cargo run --` when the debug binary has not been built.

## Create and inspect

```bash
target/debug/goog docs create "Quarterly operating review"
target/debug/goog docs get DOCUMENT_ID
target/debug/goog docs map DOCUMENT_ID
target/debug/goog docs map DOCUMENT_ID --json
```

The create command prints the document ID and edit URL separated by a tab.
Capture both values from the output rather than guessing the URL.

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

Apply one numbering operation over the complete contiguous action range so all items share one native list:

```bash
target/debug/goog docs list-format apply DOCUMENT_ID --from-index START --to-index END --type numbered --dry-run --json
```

## Learn unfamiliar commands

```bash
target/debug/goog docs --help
target/debug/goog docs text --help
target/debug/goog docs style --help
target/debug/goog docs table --help
target/debug/goog docs image --help
```

Pass each command segment as its own shell argument.
Do not quote an entire command path such as `"docs text insert --help"`.
If a live command fails with a missing-scopes error, run `goog auth login` once, then re-run the original command.
