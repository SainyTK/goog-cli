# Google Drive operations

Use `goog drive` for Drive browsing and file operations.
Use the installed `goog` binary outside this repository.
Inside the goog-cli repository, use `target/debug/goog` when it is current, or `cargo run --` when it is not built.

## Preflight

Run the auth and help checks before live work:

```bash
goog auth list
goog drive --help
```

Use the active account unless the user specifies another authorized account.
Pass `--account EMAIL` when account routing must remain explicit.
If a command reports missing scopes, run `goog auth login` once and retry it.

## List and browse

```bash
goog drive ls
goog drive ls --type files --limit 25
goog drive ls --type folders --all
goog drive ls --folder FOLDER_ID
goog drive ls --json
```

Without `--all`, listing defaults to 50 items unless `--limit` is supplied.
Use `--folder FOLDER_ID` to browse one folder.
Use `--type items`, `files`, or `folders` to constrain the result.
Use `--show-all` only when soft-deleted items are intentionally relevant.
Use `--json` when structured output is needed for follow-up commands.

## Download

```bash
goog drive download FILE_ID
goog drive download FILE_ID --output /absolute/path/to/file
```

Confirm the file ID and destination before downloading.
Inspect the resulting local file before reporting success.

## Upload

```bash
goog drive upload /absolute/path/to/file
goog drive upload /absolute/path/to/file --folder FOLDER_ID
```

Confirm the local path exists and is the intended file.
Use `--folder` to place the upload in a specific Drive folder.
Read the returned resource information and verify the uploaded item with `goog drive ls`.

## Convert an Office file

```bash
goog drive convert DOCX_FILE_ID --to google-doc
goog drive convert XLSX_FILE_ID --to google-sheet
```

The source must already be stored in Drive.
Office Conversion creates a new Document or Spreadsheet in the source file's parent Folder and leaves the Office source unchanged.
The command prints the new Document or Spreadsheet ID and URL separated by a tab.
Verify the result with `goog drive ls --folder PARENT_FOLDER_ID`, then read it with `goog docs get` or `goog sheets get`.

## Create a folder

```bash
goog drive mkdir "Project files" --folder PARENT_FOLDER_ID
```

The parent folder is required.
Verify the new folder under the expected parent after creation.

## Move to trash

```bash
goog drive trash FILE_ID
```

This command moves the target to Google Drive trash so it can still be recovered.
Resolve the exact file ID with a read-only listing before running it.
Do not use a name match alone as trash authority.
The CLI does not expose permanent deletion.
After moving the file to trash, verify that it no longer appears in the intended location.

## Completion gate

- The command ran against the intended account.
- The exact file or folder ID was resolved before mutation.
- Uploads, conversions, and folders were listed back from the intended parent.
- Downloads exist at the intended local destination and are readable.
- Trashed files no longer appear in their former location.
- The final response includes the useful Drive ID or URL returned by the CLI.
