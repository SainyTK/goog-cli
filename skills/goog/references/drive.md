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
Native Google Docs download as editable `.docx` files, Google Sheets as `.xlsx`, and Google Slides as `.pptx`.
When `--output` is omitted, `goog` adds the matching Office extension to the Drive file name.
An explicit `--output` path is used exactly as provided.
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

## Comments and replies

```bash
goog drive comments FILE_ID
goog drive comments FILE_ID --open
goog drive comment-create FILE_ID --text "Please review." --mention reviewer@example.com
goog drive comment-edit FILE_ID --comment-id COMMENT_ID --text "Updated comment."
goog drive comment-reply FILE_ID --comment-id COMMENT_ID --text "Updated as requested." --mention owner@example.com
goog drive comment-resolve FILE_ID --comment-id COMMENT_ID
goog drive comment-resolve FILE_ID --comment-id COMMENT_ID --text "Addressed."
goog drive comment-delete FILE_ID --comment-id COMMENT_ID
```

`comments` emits one JSON object containing every non-deleted comment and its nested replies.
It follows Drive pagination automatically.
Use `--open` to include only unresolved comments.
The command works with Google Docs, Sheets, Slides, and other Drive files that support comments.
`comment-create` creates an unanchored file comment.
Use the exact comment ID returned by `comments` when editing, replying, resolving, or deleting.
Repeat `--mention EMAIL` to prefix email mentions.
Include ordinary emoji directly in `--text` when desired.
`comment-resolve` marks the Comment resolved and can create a final Reply in the same request.
`comment-delete` permanently removes the target comment.
List the comments again after every mutation.
Use `comments --open` to verify that a resolved comment is absent.

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
- Comment reads include the expected comment and nested replies.
- Comment mutations were read back from the same file.
- Resolved comments are absent from `comments --open`.
- Deleted comments no longer appear in the default comment listing.
- Trashed files no longer appear in their former location.
- The final response includes the useful Drive ID or URL returned by the CLI.
