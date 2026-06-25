## Parent

Parent issue: #34

## What to build

Add table discovery, insertion, and full-table editing: `goog docs list-tables`, `goog docs insert-table`, and `goog docs edit-table`. Table commands should use Table Handles, Table Data, Document Locations, dry-run previews, JSON dry-run output, ambiguity checks, and Revision Guards.

Table Handles should be assigned by current document order and re-resolved from the latest Document Map on each command. `edit-table` should initially replace full table cell text from rectangular CSV or TSV Table Data. Dimension mismatches must fail unless `--resize` is explicitly supplied.

## Acceptance criteria

- [ ] `goog docs list-tables DOC_ID` prints Table Handles, dimensions, locations, confidence, and compact previews.
- [ ] `goog docs list-tables DOC_ID --json` emits structured table metadata.
- [ ] Table Handles such as `table-3` are assigned by current document order.
- [ ] `edit-table --table-id table-N` re-resolves the table handle from the latest fetched Document Map.
- [ ] `goog docs insert-table DOC_ID --data table.csv --page PAGE --line LINE` inserts a table from CSV Table Data.
- [ ] Table insertion accepts CSV and TSV Table Data.
- [ ] `goog docs edit-table DOC_ID --table-id table-N --data table.csv` replaces full table cell text.
- [ ] Full-table edits fail on row or column mismatch unless `--resize` is supplied.
- [ ] `--dry-run` shows a Human Preview with compact table before and after content and does not call the mutation endpoint.
- [ ] `--dry-run --json` emits the resolved table, revision ID, and native Batch Update request body.
- [ ] `--required-revision-id` adds a Revision Guard to the native request body.
- [ ] Tests cover realistic table shapes including wide tables and multi-row tables.
- [ ] Existing raw `docs get` and `docs batch-update` behavior remains unchanged.

## Blocked by

- #35
- #36
- #37
