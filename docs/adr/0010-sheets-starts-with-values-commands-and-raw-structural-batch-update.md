# Sheets Starts With Values Commands and Raw Structural Batch Update

The first Google Sheets surface separates cell value operations from structural spreadsheet operations. `goog sheets values get`, `batch-get`, `update`, `batch-update`, `append`, `clear`, and `batch-clear` use the Google Sheets values endpoints because reading and editing cell values is the common scripting path, while `goog sheets batch-update` accepts the full native `spreadsheets.batchUpdate` request body for structural changes such as adding sheets, formatting cells, changing dimensions, creating filters, merges, and protected ranges. This avoids forcing simple value edits through the heavier structural API while also avoiding premature abstractions over the more complex spreadsheet mutation model.

Read commands request `spreadsheets.readonly`; write commands request `spreadsheets`, preserving incremental authorization so users who only inspect spreadsheets do not grant edit access.
