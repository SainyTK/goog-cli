# Local Docs image staging

Google Docs requires an HTTPS URL for `insertInlineImage`.
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
