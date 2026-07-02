# TASK

Fix issue {{TASK_ID}}: {{ISSUE_TITLE}}

Pull in the issue using `gh issue view <ID>`. If it has a parent PRD, pull that in too.

Only work on the issue specified.

Work on branch {{BRANCH}}. Make commits and run tests.

This branch may already exist from a previous run and be stale relative to `{{TARGET_BRANCH}}` -- it won't have picked up any infra, dependency, or doc changes merged since. Before doing anything else, sync it: `git merge {{TARGET_BRANCH}} --no-edit`. If there are conflicts, resolve them by reading both sides and keeping the intent of each; if the merge is a no-op, that's fine too.

# CONTEXT

Here are the last 10 commits:

<recent-commits>

!`git log -n 10 --format="%H%n%ad%n%B---" --date=short`

</recent-commits>

# EXPLORATION

Explore the repo and fill your context window with relevant information that will allow you to complete the task.

Pay extra attention to test files that touch the relevant parts of the code.

# E2E VERIFICATION

Before writing any unit tests, verify the change by actually running the `goog` CLI against a real, already-connected Google account. Follow `docs/agents/e2e-testing.md` for which account to use, which commands are read-only vs. mutating, and the evidence/redaction rules -- do not improvise around it.

1. Run the `goog` command(s) this issue touches and observe the real output, not a mocked one.
2. If this surfaces a bug that is out of scope for this issue and too large to fix here, do not attempt it. Open a new GitHub issue describing it (`gh issue create`), comment on issue {{TASK_ID}} linking the new issue, and continue with {{TASK_ID}}'s own scope.
3. Save every `goog` invocation and its output to `.sandcastle/evidence/issue-{{TASK_ID}}-e2e.log`.
4. Before staging that log, scrub it per the redaction checklist in `docs/agents/e2e-testing.md`. Real email addresses, message/document/spreadsheet contents, and any token or secret must never enter git history.
5. Commit the redacted evidence log in the same commit as the change it verifies.

# EXECUTION

Once E2E verification confirms the real behavior, use RGR to build the regression test suite.

1. RED: write one test
2. GREEN: write the implementation to pass that test
3. REPEAT until done
4. REFACTOR the code

# FEEDBACK LOOPS

Before committing, run `npm run typecheck` and `npm run test` to ensure the tests pass.

# COMMIT

Make a git commit. The commit message must:

1. Start with `RALPH:` prefix
2. Include task completed + PRD reference
3. Key decisions made
4. Files changed
5. Blockers or notes for next iteration

Keep it concise.

# THE ISSUE

If the task is not complete, leave a comment on the issue with what was done.

Do not close the issue - this will be done later.

Once complete, output <promise>COMPLETE</promise>.

# FINAL RULES

ONLY WORK ON A SINGLE TASK.
