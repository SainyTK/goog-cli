# Google Sheets operations

## Contents

- Required workflow
- Data and formula rules
- Formatting and workbook quality
- Completion gate
- Command guide
  - Create and inspect
  - Manage comments
  - Write values and formulas
  - Structure and format
  - Controlled inputs
  - Learn unfamiliar commands

## Required workflow

1. Define the workbook's audience, decisions, inputs, outputs, and update cadence.
2. Plan the tabs, table boundaries, formulas, formats, and validation rules before writing values.
3. Run `goog auth list` and record the active account before any live mutation.
4. Run `goog sheets --help` and the relevant nested `--help` command before the first mutation.
5. Create or inspect the workbook.
6. Write source data in bounded tables, then add formulas and formatting.
7. Prefer high-level `goog sheets values` and `goog sheets sheet` commands over `batch-update`.
8. Re-read important ranges after every write phase.
9. Inspect metadata and grid data for formulas, formats, and sheet structure.
10. Open the live workbook at 100% zoom and inspect each delivered tab in its normal reading position.
11. Return the live Google Sheets URL and summarize the finished workbook.

## Data and formula rules

- Use the active account shown by `goog auth list` unless the user specifies another authorized account.
- Pass `--account EMAIL` for every live command when account selection must remain explicit.
- If authorization opens an account chooser, select only the recorded active or user-specified account.
- Never infer an account from browser order, a remembered identity, or unrelated open workbooks.
- If a command fails because the account is missing required scopes, run `goog auth login` once and retry.
- Keep raw inputs separate from derived views when the workbook has meaningful calculation logic.
- Use `--value-input-option user-entered` when formulas, dates, percentages, or currencies should be parsed by Sheets.
- Use formulas for derived values instead of pasting calculated constants.
- Avoid unexplained magic numbers inside formulas.
- Prefer bounded ranges over whole-column formulas when a stable table boundary is known.
- Guard denominators and optional lookups with appropriate error handling.
- Never replace a working formula with a static value unless the user asks.

## Formatting and workbook quality

Give each tab one clear role.
Use names that describe the content rather than implementation details.
Place raw or manually maintained inputs before analysis and presentation tabs when that order helps navigation.
Keep assumptions visible and label units, dates, and currencies.

Use one header row for each rectangular dataset.
Avoid merged cells inside data tables.
Keep identifiers and categories on the left, measures to the right, and notes at the far right when practical.
Use blank space sparingly because empty rows and columns can disrupt filters, formulas, and exports.
Freeze headers on tables that extend beyond one screen.

Use references to assumption cells rather than embedding unexplained constants.
Keep formula patterns consistent down a column.
Use absolute references deliberately.
Check first, middle, and last formula rows after filling a range.
Inspect for parse errors, division errors, missing lookups, and unintended blanks.
Reconcile totals against source values or an independently computed check.

Use a restrained palette with a dark or saturated header and lighter section treatments.
Reserve warning colors for genuine exceptions.
Align labels left, numeric values right, and short controlled statuses consistently.
Use sensible precision and avoid displaying more decimal places than the decision requires.
Keep descriptions readable through wrapping and deliberate column widths.
Auto-resize first, then set deliberate widths for columns that remain cramped or excessively wide.
Use conditional formatting only when it helps the reader detect a meaningful state or threshold.
Use validation lists or checkboxes for controlled inputs, bounded to cells the user will actually edit.

Use an already authenticated native browser session for the recorded account when one is available.
If no matching authenticated browser session is available, report visual QA as blocked instead of claiming the completion gate passed.

## Completion gate

- Every requested tab, field, formula, and output exists.
- Formula cells return expected values and contain no errors.
- Headers are distinct, frozen where useful, and aligned with the correct columns.
- Dates, percentages, currencies, and decimals use correct number formats.
- Long text is readable without destroying table density.
- Validation and protected ranges match the intended editing workflow.
- The live workbook has been visually inspected at 100% zoom for readable widths, clean table boundaries, and distracting formatting outside the used range.
- Important source and output ranges have been read back from the live workbook.
- Metadata verification uses a bounded range, a narrow `--fields` selector, and a compact summary instead of emitting the complete grid response.
- No helper data, placeholder values, or test formulas remain visible in the delivered workbook.
- The final URL opens the intended native Google Sheet.

## Command guide

Use `target/debug/goog` in the repository examples below.
Replace it with `cargo run --` when the debug binary has not been built.

### Create and inspect

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

### Manage comments

```bash
target/debug/goog sheets comments SPREADSHEET_ID
target/debug/goog sheets comments SPREADSHEET_ID --open
target/debug/goog sheets comment-create SPREADSHEET_ID --text "Please review."
target/debug/goog sheets comment-edit SPREADSHEET_ID --comment-id COMMENT_ID --text "Updated comment."
target/debug/goog sheets comment-reply SPREADSHEET_ID --comment-id COMMENT_ID --text "Updated as requested."
target/debug/goog sheets comment-resolve SPREADSHEET_ID --comment-id COMMENT_ID --text "Addressed."
target/debug/goog sheets comment-delete SPREADSHEET_ID --comment-id COMMENT_ID
```

Spreadsheet comments use the Google Drive comment model and are unanchored at the file level when created by the CLI.
List comments after every mutation and use `comments --open` to confirm that a resolved comment is absent.
Use the exact comment ID returned by the list or create command.
Repeat `--mention EMAIL` on create, edit, reply, or resolve when the comment should notify specific collaborators.

### Write values and formulas

```bash
target/debug/goog sheets values update-table SPREADSHEET_ID 'Plan!A1:H20' --data plan.csv --value-input-option user-entered
target/debug/goog sheets values update-cell SPREADSHEET_ID 'Plan!H2' '=IFERROR(F2/G2,0)' --value-input-option user-entered
target/debug/goog sheets values append-row SPREADSHEET_ID 'Inputs!A:D' --value '2026-07-01' --value 'North' --value '1200' --value 'Approved'
```

Use CSV for comma-containing data and TSV when the content contains many commas.
Quote A1 ranges containing spaces or shell-sensitive characters.

### Structure and format

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

### Controlled inputs

```bash
target/debug/goog sheets sheet data-validation-list SPREADSHEET_ID SHEET_ID --start-row 1 --end-row 100 --start-column 3 --end-column 4 --value Open --value Blocked --value Done
target/debug/goog sheets sheet data-validation-checkbox SPREADSHEET_ID SHEET_ID --start-row 1 --end-row 100 --start-column 0 --end-column 1
target/debug/goog sheets sheet protect-range SPREADSHEET_ID SHEET_ID --start-row 0 --end-row 1 --start-column 0 --end-column 8 --description 'Protect headers'
```

### Learn unfamiliar commands

```bash
target/debug/goog sheets --help
target/debug/goog sheets values --help
target/debug/goog sheets sheet --help
target/debug/goog sheets sheet number-format --help
```

Pass each command segment as its own shell argument.
Do not quote an entire command path.
If a live command fails with a missing-scopes error, run `goog auth login` once, then re-run the original command.
