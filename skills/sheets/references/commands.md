# Sheets command guide

Use `target/debug/goog` in the repository examples below.
Replace it with `cargo run --` when the debug binary has not been built.

## Create and inspect

```bash
target/debug/goog sheets create "Operating plan"
target/debug/goog sheets get SPREADSHEET_ID
target/debug/goog sheets values get-table SPREADSHEET_ID 'Plan!A1:H30'
```

The create command prints the spreadsheet ID and edit URL separated by a tab.
Use the numeric `sheetId` from spreadsheet metadata for `goog sheets sheet` commands.
Do not confuse a tab title with its numeric `sheetId`.

For formula and format verification, request only the bounded fields needed for the audit:

```bash
target/debug/goog sheets get SPREADSHEET_ID \
  --include-grid-data \
  --range 'Plan!A1:H30' \
  --fields 'spreadsheetId,spreadsheetUrl,sheets(properties,conditionalFormats,data(rowData(values(userEnteredValue,effectiveValue,formattedValue,dataValidation,effectiveFormat(numberFormat,backgroundColor,horizontalAlignment,verticalAlignment,textFormat(bold,fontSize,foregroundColor)))),rowMetadata(pixelSize,hiddenByUser),columnMetadata(pixelSize,hiddenByUser)))'
```

Do not emit complete grid data without a narrow `--fields` selector.
Large raw responses waste context and make verification harder.

## Write values and formulas

```bash
target/debug/goog sheets values update-table SPREADSHEET_ID 'Plan!A1:H20' --data plan.csv --value-input-option user-entered
target/debug/goog sheets values update-cell SPREADSHEET_ID 'Plan!H2' '=IFERROR(F2/G2,0)' --value-input-option user-entered
target/debug/goog sheets values append-row SPREADSHEET_ID 'Inputs!A:D' --value '2026-07-01' --value 'North' --value '1200' --value 'Approved'
```

Use CSV for comma-containing data and TSV when the content contains many commas.
Quote A1 ranges containing spaces or shell-sensitive characters.

## Structure and format

```bash
target/debug/goog sheets sheet freeze SPREADSHEET_ID SHEET_ID --rows 1
target/debug/goog sheets sheet background-color SPREADSHEET_ID SHEET_ID --start-row 0 --end-row 1 --start-column 0 --end-column 8 '#1A73E8'
target/debug/goog sheets sheet text-color SPREADSHEET_ID SHEET_ID --start-row 0 --end-row 1 --start-column 0 --end-column 8 '#FFFFFF'
target/debug/goog sheets sheet bold SPREADSHEET_ID SHEET_ID --start-row 0 --end-row 1 --start-column 0 --end-column 8
target/debug/goog sheets sheet number-format SPREADSHEET_ID SHEET_ID --start-row 1 --end-row 20 --start-column 5 --end-column 6 --type currency --pattern '$#,##0.00'
target/debug/goog sheets sheet text-wrap SPREADSHEET_ID SHEET_ID --start-row 0 --end-row 20 --start-column 2 --end-column 5 --strategy wrap
target/debug/goog sheets sheet auto-resize SPREADSHEET_ID SHEET_ID --dimension columns --start-index 0 --end-index 8
```

Grid indexes are zero-based, start-inclusive, and end-exclusive.
For example, row 1 uses `--start-row 0 --end-row 1`.

## Controlled inputs

```bash
target/debug/goog sheets sheet data-validation-list SPREADSHEET_ID SHEET_ID --start-row 1 --end-row 100 --start-column 3 --end-column 4 --value Open --value Blocked --value Done
target/debug/goog sheets sheet data-validation-checkbox SPREADSHEET_ID SHEET_ID --start-row 1 --end-row 100 --start-column 0 --end-column 1
target/debug/goog sheets sheet protect-range SPREADSHEET_ID SHEET_ID --start-row 0 --end-row 1 --start-column 0 --end-column 8 --description 'Protect headers'
```

## Learn unfamiliar commands

```bash
target/debug/goog sheets --help
target/debug/goog sheets values --help
target/debug/goog sheets sheet --help
target/debug/goog sheets sheet number-format --help
```

Pass each command segment as its own shell argument.
Do not quote an entire command path.
If a live command fails with a missing-scopes error, run `goog auth login` once, then re-run the original command.
