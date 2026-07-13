# Upfront Full-Scope Login

Supersedes ADR-0003.

`goog auth login` and device login request every scope the CLI supports in a single consent operation.
Normal service commands never open an incremental consent flow.
If an older token lacks a required scope, the command exits with an instruction to run `goog auth login` once.
Read/write scope splits (`documents.readonly`, `spreadsheets.readonly`) are removed; each service has one full-access scope.

## Considered Options

ADR 0003 rejected this exact approach because Google's consent screen lists every scope verbatim, and a wall of permissions can cause users to abandon login. We're accepting that cost here because this CLI is used by a small number of known accounts with the OAuth app kept in Google's Testing publishing status (no verification/CASA review required for sensitive scopes at this scale). One login, one consent screen, full access, versus a second surprise prompt every time a user touches a new API for the first time.

## Consequences

`ensure_scopes` stays in place as a safety check, but it returns `MissingScopes` instead of opening a browser.
Accounts logged in before this decision may need one explicit `goog auth login` to replace their partial token with a full-scope token.
If this CLI is ever distributed beyond a handful of trusted accounts, the OAuth app will need Google verification for these sensitive scopes.
Revisit this decision first in that case.
