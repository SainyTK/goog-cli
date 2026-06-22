# 09 - `goog drive upload`

Status: ready-for-agent

## Parent

`.scratch/goog-cli-mvp/PRD.md`

## What to build

Implement `goog drive upload <path>` in the `goog-drive` crate. The command streams file content from disk directly into the HTTP request body without loading the entire file into memory.

The upload protocol is chosen automatically by file size:
- Files 5 MB and under: multipart upload (single request with metadata + body).
- Files over 5 MB: Google's Resumable Upload protocol - initiate a session with a POST to get an upload URI, then PUT the file in chunks. This allows uploads to survive network interruptions.

Behavior:
- Show a progress bar (via `indicatif`) during upload, showing bytes transferred and speed. Suppressed with `--quiet`.
- On success, print the uploaded file's Drive file ID and its web view URL.
- `--folder <folder-id>`: upload into the specified Drive folder instead of the root.
- Clear errors on stderr with a non-zero exit for permission failures or invalid folder IDs.

## Acceptance criteria

- [ ] `goog drive upload <path>` uploads a small file using the multipart protocol and prints the Drive file ID and URL on success.
- [ ] Files over 5 MB automatically use the Resumable Upload protocol.
- [ ] `--folder <folder-id>` places the file in the specified folder.
- [ ] A progress bar is shown during upload and clears on completion.
- [ ] `--quiet` suppresses the progress bar.
- [ ] The upload streams from disk - peak memory usage does not scale with file size.
- [ ] `upload()` is tested with `wiremock`: a small file sends a multipart request, a large file initiates a resumable session and PUTs chunks to the upload URI, and a 403 surfaces a permission error.

## Blocked by

- `06-incremental-scope-authorization.md`
