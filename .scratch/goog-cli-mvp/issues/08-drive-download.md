# 08 - `goog drive download`

Status: ready-for-agent

## Parent

`.scratch/goog-cli-mvp/PRD.md`

## What to build

Implement `goog drive download <file-id>` in the `goog-drive` crate. The command streams the file content directly from the Drive API response to disk without buffering the entire file in memory, so arbitrarily large files work without OOM risk.

Behavior:
- Default destination: current working directory, using the file's name from the Drive API.
- `--output <path>`: save to the specified path instead.
- Show a progress bar (via `indicatif`) during download, showing bytes transferred and speed. The progress bar is suppressed with `--quiet`.
- `DriveError::NotFound` on a 404 response (file does not exist).
- `DriveError::PermissionDenied` on a 403 response (no access).
- Both errors surface on stderr with a non-zero exit code.

## Acceptance criteria

- [ ] `goog drive download <id>` saves the file to the current directory using the Drive file's name.
- [ ] `--output <path>` saves to the specified path.
- [ ] A progress bar is shown during download and clears on completion.
- [ ] `--quiet` suppresses the progress bar.
- [ ] The download streams to disk - peak memory usage does not scale with file size (verified by the streaming implementation, not a runtime memory test).
- [ ] `download()` is tested with `wiremock`: a mocked binary response is streamed correctly to a temp directory, a 404 surfaces `DriveError::NotFound`, and a 403 surfaces `DriveError::PermissionDenied`.

## Blocked by

- `06-incremental-scope-authorization.md`
