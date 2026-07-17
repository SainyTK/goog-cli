# Local Docs image staging

Google Docs requires an HTTPS URL for `insertInlineImage`.
Local image insertion uses the built-in `drive-public` backend unless `--staging-command` selects a caller-provided HTTPS staging service.

## Drive public staging

```sh
goog --account alice@example.com docs image insert DOCUMENT_ID \
  --file './dashboard image.png' \
  --staging drive-public \
  --at 'after-heading:Application dashboard' \
  --json
```

The built-in backend uploads the validated image with its detected MIME type, grants a temporary `anyone` reader permission, inserts the public Drive URI into the document, and deletes the staged file after Google Docs confirms insertion.
The staged file is also deleted when public permission creation or Docs insertion fails.
`--keep-staged` is the explicit debugging opt-out from deletion.

If Workspace policy rejects public sharing with `publishOutNotPermitted`, JSON output uses `DOCS_IMAGE_STAGING_POLICY_BLOCKED`, reports whether cleanup completed, and suggests either a staging adapter or the existing URI form.

## Staging adapters

The `--staging-command` option lets a caller provide a temporary HTTPS staging service without exposing local paths or cleanup tokens in process arguments.

```sh
goog --account alice@example.com docs image insert DOCUMENT_ID \
  --file './dashboard image.png' \
  --staging-command ./stage-image \
  --at 'after-heading:Application dashboard' \
  --json
```

The CLI starts the adapter executable directly without a shell.
The adapter receives exactly one JSON object on stdin and returns one JSON object on stdout.
Diagnostic messages may be written to stderr, but adapters must not print credentials or file contents.

## Stage request

```json
{
  "action": "stage",
  "path": "/absolute/path/dashboard image.png",
  "mimeType": "image/png"
}
```

The CLI detects the MIME type from the file contents and supplies a canonical absolute path.
The adapter returns an absolute HTTPS URI and may include an opaque cleanup token and expiry time.

```json
{
  "uri": "https://temporary-host.example/image-token.png",
  "cleanupToken": "opaque-token",
  "expiresAt": "2026-07-18T00:00:00Z"
}
```

HTTP URLs and URLs containing user credentials are rejected before any Google Docs mutation.
The cleanup token and staged URI are omitted from the final CLI JSON output.

## Cleanup request

When `cleanupToken` is present, the CLI invokes the same executable after Docs insertion succeeds or fails.

```json
{
  "action": "cleanup",
  "cleanupToken": "opaque-token"
}
```

The adapter exits with status zero after cleanup and may return an empty JSON object.
A nonzero adapter exit makes the overall command fail.
If insertion succeeded first, JSON output reports `DOCS_IMAGE_INSERTED_CLEANUP_FAILED` as a partial success.

`--keep-staged` skips cleanup for debugging and must be selected explicitly.
Normal runs preserve `--required-revision-id` across the Docs mutation and still request cleanup when a revision check or later mutation step fails.

## Dry run

Add `--dry-run --json` to validate the file, account, selector, sizing, and planned backend without starting the adapter or performing a remote write.
