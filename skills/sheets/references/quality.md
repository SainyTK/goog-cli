# Spreadsheet quality rules

## Workbook architecture

Give each tab one clear role.
Use names that describe the content rather than implementation details.
Place raw or manually maintained inputs before analysis and presentation tabs when that order helps navigation.
Keep assumptions visible and label units, dates, and currencies.

## Table design

Use one header row for each rectangular dataset.
Avoid merged cells inside data tables.
Keep identifiers and categories on the left, measures to the right, and notes at the far right when practical.
Use blank space sparingly because empty rows and columns can disrupt filters, formulas, and exports.
Freeze headers on tables that extend beyond one screen.

## Formula integrity

Use references to assumption cells rather than embedding unexplained constants.
Keep formula patterns consistent down a column.
Use absolute references deliberately.
Check first, middle, and last formula rows after filling a range.
Inspect for parse errors, division errors, missing lookups, and unintended blanks.
Reconcile totals against source values or an independently computed check.

## Visual hierarchy

Use a restrained palette with a dark or saturated header and lighter section treatments.
Reserve warning colors for genuine exceptions.
Align labels left, numeric values right, and short controlled statuses consistently.
Use sensible precision and avoid displaying more decimal places than the decision requires.
Keep descriptions readable through wrapping and deliberate column widths.

## Final inspection

Read back the main input table, formula columns, totals, and summary cells.
Inspect sheet metadata and grid data for formats and formulas.
Confirm filters, frozen rows, validation rules, named ranges, and protection where requested.
Open each delivered tab at 100% zoom and inspect the used range as a reader would see it.
Remove formatting or dropdown rules that spill into unused cells and create visual noise.
Verify that no helper data, placeholder values, or test formulas remain visible in the delivered workbook.
