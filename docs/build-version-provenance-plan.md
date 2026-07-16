## Summary

Development binaries and released binaries can report the same semantic version while exposing materially different command surfaces.
Add build provenance so users and support tooling can identify exactly which binary is running.

## Observed behavior

Two binaries both reported:

```text
goog 0.2.5
```

The installed release binary at `~/.local/bin/goog` did not support:

```text
goog docs image insert ... --width ... --height ...
goog docs export-pdf ...
```

The repository build from the `develop` branch supported both features while still reporting `goog 0.2.5`.

The binaries had different SHA-256 hashes, but `goog --version` did not expose a commit, channel, dirty state, or release provenance.

This made it appear that a released feature was inconsistent across installations when the actual difference was released source versus unreleased development source.

## Expected behavior

`goog --version` should distinguish a clean tagged release from any build that is ahead of the tag or built from a dirty tree.

Suggested human-readable forms:

```text
goog 0.2.5
goog 0.2.5-dev.148+8879bc2
goog 0.2.5-dev.148+8879bc2.dirty
```

The exact format may differ, but it must be unambiguous and semver-compatible where practical.

Add a structured command:

```text
goog version --json
```

Suggested fields:

```json
{
  "semanticVersion": "0.2.5",
  "displayVersion": "0.2.5-dev.148+8879bc2",
  "gitCommit": "8879bc2...",
  "dirty": false,
  "distanceFromTag": 148,
  "sourceTag": "v0.2.5",
  "releaseChannel": "development",
  "target": "aarch64-apple-darwin"
}
```

Do not include the developer's username, workspace path, hostname, or other machine-specific private data.

## Release artifact requirements

- A clean build from tag `vX.Y.Z` reports exactly the release version and channel.
- An ahead-of-tag build reports development provenance.
- A dirty build reports dirty provenance.
- Release workflows verify that uploaded binaries were built from the tagged commit.
- Release workflows run command-surface smoke tests against the packaged binary, not only a workspace build.
- Published checksums are generated from the same assets that passed smoke tests.
- Installation documentation provides a command for comparing the installed binary with release metadata.

## Tests

### Build metadata tests

- Clean tagged checkout produces the clean release version.
- One commit after the tag produces a development version with distance and short commit.
- Dirty tracked changes add dirty provenance.
- Untracked build output does not accidentally mark the tree dirty if the documented policy excludes it.
- A source archive without `.git` uses deterministic fallback metadata supplied by the build environment.
- Reproducible builds do not depend on the local wall-clock time.
- JSON output is stable and contains no private local paths or host information.

### Packaged artifact tests

- Download or install the release asset in CI.
- Assert `goog --version` matches the release tag.
- Assert `goog version --json` reports the tagged commit and release channel.
- Run representative `--help` checks on the packaged binary.
- For the next release containing image sizing and PDF export, assert the packaged binary exposes `docs image insert --width`, `--height`, and `docs export-pdf`.
- Compare the packaged binary checksum with the published checksum.

## Success criteria

- Support can distinguish release and development binaries from one command output.
- Two binaries with materially different command surfaces cannot both present only `goog 0.2.5` without additional provenance.
- Release assets identify the exact tagged source commit.
- Packaged-binary smoke tests prevent a stale or incorrectly assembled asset from being published.
- Structured version output is safe to attach to bug reports.
- No private build-machine data appears in version output.
