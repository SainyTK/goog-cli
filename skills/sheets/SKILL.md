---
name: sheets
description: Create, analyze, format, and verify native Google Sheets workbooks with the goog CLI. Use for trackers, budgets, schedules, reports, models, data cleanup, formulas, charts-ready tables, and any request involving Google Sheets values or formatting.
---

# Google Sheets

Build production-quality native spreadsheets with `goog sheets`.
Use `target/debug/goog` from the goog-cli repository, or `cargo run --`, while developing the CLI.

## Required workflow

1. Define the workbook's audience, decisions, inputs, outputs, and update cadence.
2. Plan the tabs, table boundaries, formulas, formats, and validation rules before writing values.
3. Run `target/debug/goog auth list` and record the active account before any live mutation.
4. Run `target/debug/goog sheets --help` and the relevant nested `--help` command before the first mutation.
5. Create or inspect the workbook.
6. Write source data in bounded tables, then add formulas and formatting.
7. Prefer high-level `goog sheets values` and `goog sheets sheet` commands over `batch-update`.
8. Re-read important ranges after every write phase.
9. Inspect metadata and grid data for formulas, formats, and sheet structure.
10. Open the live workbook at 100% zoom and inspect each delivered tab in its normal reading position.
11. Return the live Google Sheets URL and summarize the finished workbook.

Read [references/quality.md](references/quality.md) before creating or substantially restructuring a workbook.
Read [references/commands.md](references/commands.md) for command patterns and index rules.

## Data and formula rules

- Use the active account shown by `goog auth list` unless the user specifies another authorized account.
- Pass `--account EMAIL` for every live command when account selection must remain explicit.
- If authorization opens an account chooser, select only the recorded active or user-specified account.
- Never infer an account from browser order, a remembered identity, or unrelated open workbooks.
- If a command fails because the account is missing required scopes, run `goog auth login` once and retry; do not expect the original command to pause and resume on its own.
- Keep raw inputs separate from derived views when the workbook has meaningful calculation logic.
- Use `--value-input-option user-entered` when formulas, dates, percentages, or currencies should be parsed by Sheets.
- Use formulas for derived values instead of pasting calculated constants.
- Avoid unexplained magic numbers inside formulas.
- Prefer bounded ranges over whole-column formulas when a stable table boundary is known.
- Guard denominators and optional lookups with appropriate error handling.
- Never replace a working formula with a static value unless the user asks.

## Formatting rules

- Freeze header rows for long tables.
- Format numbers by semantic type, including currency, percent, date, time, and integer or decimal precision.
- Use restrained colors and one clear header treatment.
- Wrap text where descriptions need it, but keep compact identifiers and numeric columns unwrapped.
- Auto-resize first, then set deliberate widths for columns that remain cramped or excessively wide.
- Use conditional formatting only when it helps the reader detect a meaningful state or threshold.
- Use validation lists or checkboxes for controlled inputs, bounded to cells the user will actually edit.
- Use an already authenticated native browser session for the recorded account when one is available.
- If no matching authenticated browser session is available, report visual QA as blocked instead of claiming the completion gate passed.

## Completion gate

Do not call the workbook finished until all of these are true:

- Every requested tab, field, formula, and output exists.
- Formula cells return expected values and contain no errors.
- Headers are distinct, frozen where useful, and aligned with the correct columns.
- Dates, percentages, currencies, and decimals use correct number formats.
- Long text is readable without destroying table density.
- Validation and protected ranges match the intended editing workflow.
- The live workbook has been visually inspected at 100% zoom for readable widths, clean table boundaries, and distracting formatting outside the used range.
- Important source and output ranges have been read back from the live workbook.
- Metadata verification uses a bounded range, a narrow `--fields` selector, and a compact summary instead of emitting the complete grid response.
- The final URL opens the intended native Google Sheet.
