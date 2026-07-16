## Summary

Add first-class local-file support to `goog docs image insert` with policy-aware staging backends, guaranteed cleanup, and structured diagnostics.

The target workflow should be possible without manually uploading an image, changing Drive permissions, running a public tunnel, calculating a temporary URL, and then cleaning up every temporary resource by hand.

## User problem

Google Docs `insertInlineImage` accepts a publicly reachable HTTP or HTTPS URI.
It does not accept local file bytes.

The released CLI currently exposes only the URI form:

```text
goog docs image insert DOCUMENT_ID IMAGE_URI --at SELECTOR
```

For a local PNG, an agent or user must build a separate staging workflow.
A common implementation uploads the file to Drive, grants `anyone` reader access, inserts the image, and then revokes or deletes the staged file.

That approach fails in managed Google Workspace domains that prohibit public sharing.
The observed Drive API failure was:

```json
{
  "error": {
    "code": 400,
    "message": "Bad Request. User message: \"\"",
    "errors": [
      {
        "reason": "publishOutNotPermitted"
      }
    ]
  }
}
```

The failed upload also leaves an orphaned Drive file unless the caller implements cleanup around every partial failure.

## Proposed command surface

Support a local file as an alternative to `IMAGE_URI`:

```text
goog docs image insert DOCUMENT_ID --file ./dashboard.png --at after-heading:'Application dashboard'
```

The URI and local-file forms must be mutually exclusive.

Add an explicit staging selection:

```text
--staging auto
--staging drive-public
--staging-command PATH
```

Suggested behavior:

- `auto` tries the configured staging backend and otherwise uses `drive-public`.
- `drive-public` uploads the file to Drive, creates a temporary `anyone` reader permission, inserts the image, waits for Google Docs to copy it, and deletes the staged file or revokes the permission.
- `staging-command` runs a user-controlled adapter for environments where Workspace policy blocks public Drive sharing.

The adapter contract should be documented and machine-readable.
The CLI should pass the absolute input path and MIME type without placing file contents or credentials in command-line arguments.
The adapter should return JSON containing a public HTTPS URI and optional cleanup metadata.
The CLI remains responsible for invoking cleanup after insertion succeeds or fails.

Example adapter response:

```json
{
  "uri": "https://temporary-host.example/image-token.png",
  "cleanupToken": "opaque-token",
  "expiresAt": "2026-07-16T03:30:00Z"
}
```

The final contract may use stdin and stdout or environment variables, but it must avoid shell interpolation of untrusted file paths.

## Required behavior

- Detect the MIME type from content and validate that Google Docs supports it.
- Reject missing, unreadable, empty, or unsupported files before creating remote resources.
- Make `--dry-run --json` validate the local file and show the planned backend without uploading anything.
- Pin all Google operations to `--account EMAIL` and report the selected account in JSON output.
- Treat staging and Docs insertion as one cleanup-aware operation.
- Remove the staged file or permission after Google Docs confirms the copy.
- Clean up partial uploads when permission creation, image insertion, revision checks, or later validation fails.
- Provide `--keep-staged` only as an explicit debugging option.
- Never print OAuth tokens, signed credentials, private headers, or local file contents.
- Preserve `--required-revision-id` semantics across the complete operation.
- Return a nonzero exit status if insertion or required cleanup fails.
- If insertion succeeds but cleanup fails, return a distinct partial-success result containing the staged resource ID and a safe cleanup command.
- Confirm that the embedded image remains available after the temporary source is deleted.

## Structured errors

The JSON error should retain the underlying Google reason and add a stable CLI classification.

For the observed policy failure, return fields equivalent to:

```json
{
  "code": "DOCS_IMAGE_STAGING_POLICY_BLOCKED",
  "backend": "drive-public",
  "googleReason": "publishOutNotPermitted",
  "cleanupCompleted": true,
  "nextActions": [
    "Configure --staging-command for a permitted HTTPS staging service",
    "Use the existing IMAGE_URI form with a caller-hosted image"
  ]
}
```

Do not flatten this case into a generic `400 Bad Request` message.

## Tests

### Unit and mock API tests

- Local PNG validation succeeds and produces the correct MIME type.
- Unsupported input fails before any Drive or Docs request.
- `--dry-run` performs zero remote writes.
- Drive upload, public permission, Docs insertion, and cleanup occur in the documented order.
- A `publishOutNotPermitted` response is classified correctly.
- A policy-blocked upload is deleted and leaves no orphaned Drive file.
- A Docs insertion failure triggers staging cleanup.
- A revision mismatch triggers staging cleanup and preserves the target document.
- A cleanup failure produces the distinct partial-success result.
- Paths containing spaces, Unicode, quotes, and leading dashes are handled without shell injection.
- The staging-command response rejects non-HTTPS URLs by default.
- JSON output contains no credentials or signed request headers.

### Live acceptance tests

- Insert a local PNG through `drive-public` using an account that permits public sharing.
- Verify the image appears in `goog docs map --type images`.
- Delete the staged Drive file and export the Doc again to prove the image was copied.
- Run against a Workspace account that returns `publishOutNotPermitted` and verify no Drive orphan remains.
- Configure a test staging adapter for the restricted account and complete the same insertion successfully.
- Shut down the staging endpoint and export the Doc again to prove the embedded image remains intact.

## Success criteria

- A local PNG can be inserted with one `goog` command when an allowed staging backend is configured.
- The restricted Workspace case succeeds through a non-Drive staging adapter without manual API scripting.
- When no permitted backend is configured, the command fails with an actionable typed error.
- No temporary public file, Drive permission, or orphaned upload remains after a successful run.
- No temporary resource remains after any tested failure path where cleanup is possible.
- The inserted image remains present after staging cleanup.
- Dry-run, account selection, revision guards, and JSON output remain scriptable.

## Out of scope

- Making a managed Workspace permit public Drive sharing.
- Silently uploading private business images to an unknown third-party service.
- Bundling a mandatory external tunnel provider into the default installation.
