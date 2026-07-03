# Upfront Full-Scope Login

Supersedes ADR-0003.

`goog auth login` (and device login) now requests every scope the CLI supports -- Drive, Docs, Sheets, and Gmail (`gmail.modify`) -- in a single consent screen, instead of acquiring scopes incrementally per command. Read/write scope splits (`documents.readonly`, `spreadsheets.readonly`) are removed; each service has one full-access scope.

## Considered Options

ADR 0003 rejected this exact approach because Google's consent screen lists every scope verbatim, and a wall of permissions can cause users to abandon login. We're accepting that cost here because this CLI is used by a small number of known accounts with the OAuth app kept in Google's Testing publishing status (no verification/CASA review required for sensitive scopes at this scale). One login, one consent screen, full access, versus a second surprise prompt every time a user touches a new API for the first time.

## Consequences

`ensure_scopes` (the per-request scope check) stays in place as a safety net: it still detects and incrementally requests any scope missing from a token (e.g. accounts logged in before this change), but is expected to no-op for new logins since login already grants everything. If this CLI is ever distributed beyond a handful of trusted accounts, the OAuth app will need Google verification for these sensitive scopes -- revisit this decision first.
