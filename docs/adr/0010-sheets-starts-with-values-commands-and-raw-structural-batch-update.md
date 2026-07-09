# Sheets Starts With Values Commands and Raw Structural Batch Update

The first Google Sheets surface separates cell value operations from structural spreadsheet operations.
`goog sheets values get`, `update`, `batch-update`, `append`, and `clear` use the Google Sheets values endpoints because reading and editing cell values is the common scripting path, while `goog sheets batch-update` accepts the full native `spreadsheets.batchUpdate` request body for structural changes such as adding sheets, formatting cells, changing dimensions, creating filters, merges, and protected ranges.
`goog sheets values get` accepts repeated `--range` flags for multi-range reads, replacing the original separate `batch-get` command.
`goog sheets values clear` accepts repeated `--range` flags for multi-range clears, replacing the original separate `batch-clear` command.
This avoids forcing simple value edits through the heavier structural API while also avoiding premature abstractions over the more complex spreadsheet mutation model.

Read and write commands both requested `spreadsheets.readonly` vs. `spreadsheets` scopes respectively, preserving incremental authorization so users who only inspected spreadsheets did not grant edit access.
Superseded by ADR-0014: all Sheets commands now use a single full-access `spreadsheets` scope.
