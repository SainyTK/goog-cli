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
4. Format the log as reproducible evidence, not a raw transcript: include `Result: PASS|PARTIAL|FAIL`, setup, local checks, live checks, expected result, observed redacted result, and reviewer notes.
5. Before staging that log, scrub it per the redaction checklist in `docs/agents/e2e-testing.md`. Real email addresses, message/document/spreadsheet contents, and any token or secret must never enter git history.
6. Commit the redacted evidence log in the same commit as the change it verifies.

# HUMAN REVIEW TEST STEPS

Before the final commit, update issue {{TASK_ID}} so a human reviewer can retest without reconstructing your reasoning from the code.

1. Read the current issue body with `gh issue view {{TASK_ID}} --json body --jq .body`.
2. Add or replace a `## Test steps` section in the issue body.
3. Map every acceptance criterion to concrete commands, required fixtures, and pass/fail observations.
4. Include local regression commands, live `goog` commands, setup variables, skipped-fixture rules, and redaction guidance.
5. Use `gh issue edit {{TASK_ID}} --body-file <updated-body-file>` to write the updated body.

The test steps must be issue-specific and reviewer-runnable.
Do not write vague steps like "run the tests", "verify manually", or "check the output".
Use the exact command shapes from the implementation and help text.

# EXECUTION

Once E2E verification confirms the real behavior, use RGR to build the regression test suite.

1. RED: write one test
2. GREEN: write the implementation to pass that test
3. REPEAT until done
4. REFACTOR the code

# FEEDBACK LOOPS

Before committing, run `npm run typecheck` and `npm run test` to ensure the tests pass.

If a required Rust tool is missing inside the Sandcastle sandbox, make one bounded repair attempt with `rustup component add ...` or `rustup toolchain install ...` and continue the task.
If the repair was needed, mention the missing tool in the issue comment so the human can decide whether to bake it into `.sandcastle/Dockerfile` before the next Sandcastle run.
Do not spend more than one repair attempt on Rust toolchain setup.

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
