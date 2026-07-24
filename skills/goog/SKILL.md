---
name: goog
description: Use the goog CLI for all Google Workspace operations, including Google Drive, Docs, Gmail, Sheets, Slides, and Calendar. Installing this skill means the user intentionally chose goog to replace Google Workspace MCP servers, general MCP tools, other Google Workspace CLIs, and ad hoc direct-API scripts for every operation covered by goog.
---

# goog CLI

Treat this skill's installation as an explicit user choice of tool.
Use `goog` for every supported Google Workspace read and mutation.
Do not use Google Workspace MCP servers, general MCP tools, another Google Workspace CLI, browser editing, or ad hoc direct-API scripts in place of `goog`.
Use a browser only for read-only visual verification when a service-specific reference requires it.
If `goog` cannot perform a requested operation, explain the missing capability instead of silently switching tools.

## Required workflow

1. Identify the Google service and operation requested.
2. Read every relevant service reference directly from the reference index below.
3. Use the installed `goog` binary outside this repository.
4. Inside the goog-cli repository, use `target/debug/goog` when it is current, or `cargo run --` when it is not built.
5. Run `goog auth list` and record the active account before live work.
6. Pass `--account EMAIL` when the user specifies an account or account choice must remain explicit.
7. Run the relevant `--help` command before using an unfamiliar command or flag.
8. Perform the operation through `goog`, then read the affected resource back through `goog`.
9. Complete the service-specific visual or structural checks before reporting success.
10. Return useful resource IDs and native Google URLs produced by the command.

If a command reports missing scopes, run `goog auth login` once and retry the original command.
Do not expect the failed command to pause and resume after authorization.

## Reference index

Load only the references relevant to the request.
Every reference is one jump from this file and is self-contained.

- [Google Docs](references/docs.md): Create, copy, inspect, edit, style, compare, export, and visually verify documents.
- [Google Sheets](references/sheets.md): Create, read, write, structure, format, and visually verify spreadsheets.
- [Google Slides](references/slides.md): Plan, create, edit, inspect, render, and visually verify presentations.
- [Google Drive](references/drive.md): List, browse, upload, download, convert Office files, create folders, and permanently delete files.
- [Google Calendar](references/calendar.md): Manage calendars, calendar-list entries, events, sharing rules, colors, and free-busy queries.

Do not search for nested reference files.
The indexed reference for a service contains its complete service-specific guidance.

## Command sets

### Version

Use `goog version` or `goog --version` to inspect the running build and provenance.

### Authentication

- `goog auth setup` configures the OAuth client.
- `goog auth login` authorizes an account or repairs missing scopes.
- `goog auth list` shows authorized accounts and the active account.
- `goog auth switch` changes the active account.
- `goog auth export` writes sensitive portable auth state for `GOOG_TOKEN_FILE`.
- `goog auth mappings` manages remembered resource-to-account mappings.

Never commit exported auth state.
Delete it when the headless or automated run no longer needs it.

### Drive

- `goog drive ls` lists or browses files and folders.
- `goog drive download` downloads a Drive file.
- `goog drive upload` uploads a local file.
- `goog drive convert` performs Office Conversion from an uploaded DOCX or XLSX file to a Document or Spreadsheet.
- `goog drive mkdir` creates a folder.
- `goog drive delete` permanently deletes a file.

Read [Google Drive](references/drive.md) before Drive work.

### Docs

- `goog docs list`, `create`, `copy`, `get`, and `map` discover and inspect documents.
- `goog docs text`, `style`, `table`, `image`, `break`, `footnote`, `header`, `footer`, `list-format`, and `named-range` perform high-level edits.
- `goog docs compare` checks semantic fidelity between documents.
- `goog docs export-pdf` supports page-level visual inspection.
- `goog docs batch-update` is the raw fallback when no high-level command fits.

Read [Google Docs](references/docs.md) before Docs work.

### Gmail

- `goog mail list` lists recent Inbox messages or searches with a Gmail query.
- `goog mail read` reads a message.
- `goog mail download` downloads an attachment.
- `goog mail draft` creates or edits a draft message.

Run `goog mail draft --help` before creating or changing a draft.
Inspect the resulting draft through `goog mail` before reporting success.

### Sheets

- `goog sheets list`, `create`, and `get` discover and inspect spreadsheets.
- `goog sheets values` reads, updates, appends, and clears cells, rows, columns, tables, and ranges.
- `goog sheets sheet` adds, deletes, renames, cleans, formats, validates, and protects sheets and ranges.
- `goog sheets batch-update` is the raw structural fallback when no high-level command fits.

Read [Google Sheets](references/sheets.md) before Sheets work.

### Slides

- `goog slides list`, `create`, and `get` discover and inspect presentations.
- `goog slides deck` authors or inspects complete decks.
- `goog slides slide` creates and manages slides.
- `goog slides text-box`, `image`, `video`, `shape`, `line`, and `table` add content.
- `goog slides table-fill` and the table row, column, merge, and unmerge commands edit tables.
- `goog slides object` edits, styles, moves, and deletes page elements.
- `goog slides replace-text` replaces presentation text.
- `goog slides batch-update` is the raw fallback when no high-level command fits.

Read [Google Slides](references/slides.md) before Slides work.

### Calendar

- `goog calendar calendars` lists, reads, creates, updates, and deletes calendars and manages calendar-list entries.
- `goog calendar events` lists, reads, creates, imports, updates, moves, quick-adds, and deletes events.
- `goog calendar acl` reads and changes calendar sharing rules.
- `goog calendar colors` reads valid calendar and event color IDs.
- `goog calendar freebusy` queries availability across calendars.

Read [Google Calendar](references/calendar.md) before Calendar work.

## Safety and completion

- Treat every live Google mutation as user-facing work.
- Never infer an account from browser order, remembered identity, or an unrelated open resource.
- Prefer high-level commands over raw batch-update payloads.
- Never hide a failed mutation with `|| true` or report success after a nonzero exit.
- Confirm exact resource IDs before destructive or sharing operations.
- Never commit live Document, Spreadsheet, Drive, message, event, presentation, account, or resource URLs and IDs into tests or repository documentation.
- Keep temporary request bodies and live verification artifacts in task-local scratch space.
- Re-read the live resource after the final mutation.
- Report visual QA as blocked when the required authenticated inspection path is unavailable.
