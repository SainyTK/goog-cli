## Agent skills

### Issue tracker

Issues live in GitHub Issues (`gh` CLI). External PRs are not a triage surface. See `docs/agents/issue-tracker.md`.

### Triage labels

Default label vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

`ready-for-agent` or `bug` makes an issue eligible for the Sandcastle automation loop -- there is no separate `Sandcastle` label.

### Domain docs

Single-context repo: one `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.

### End-to-end testing

Sandcastle's implementer verifies real behavior against a live Google account before writing regression unit tests. See `docs/agents/e2e-testing.md`.

Real Document/Spreadsheet/Drive/message IDs and URLs discovered during that live verification are for local, throwaway use only -- never commit them into unit test source. Use a placeholder ID instead once the test is written. See `docs/agents/e2e-testing.md`.
