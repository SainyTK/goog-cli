# TASK

Merge the following branches into the current branch:

{{BRANCHES}}

For each branch:

1. Run `git merge <branch> --no-edit`
2. If there are merge conflicts, resolve them intelligently by reading both sides and choosing the correct resolution
3. After resolving conflicts, run `npm run typecheck` and `npm run test` to verify everything works
4. If tests fail, fix the issues before proceeding to the next branch

After all branches are merged, make a single commit summarizing the merge.

# HAND OFF ISSUES FOR HUMAN REVIEW

Do not close issues after merging. A human will test, review, and close each issue manually if it passes.

For each issue whose branch was merged:

1. Add `need-human-review`.
2. Remove `bug` if it is present.
3. Leave the issue open.
4. Read the issue body and confirm it contains a `## Test steps` section with concrete commands and pass/fail expectations.
5. If the section is missing or vague, add a short warning to the handoff comment naming that gap. Do not invent test steps in the merge phase unless the evidence log already contains enough exact commands to do so safely.
6. If `.sandcastle/evidence/issue-<ID>-e2e.log` exists (it's now on the current branch after merging), read its contents. Issue branches are never pushed to GitHub, so a blob link to them is always dead -- embed the log's contents directly in the comment instead, inside a collapsible `<details>` block, so a reviewer sees it without leaving the issue.
7. Confirm the evidence log is reproducible: it should include setup, exact commands, expected result, observed redacted result, pass/fail status, and any skipped fixture reason.
8. Add a comment saying the Sandcastle implementation is ready for human review, with a pointer to the issue's `## Test steps` section and with the evidence embedded per step 6 if the file exists.

Use these commands:

`gh issue edit <ID> --add-label "need-human-review"`

`gh issue edit <ID> --remove-label "bug" || true`

Build the comment body so the evidence log (if present) renders as:

```
Implemented by Sandcastle and ready for human review.

Please follow the `## Test steps` section in the issue body.
Close this issue if it passes.
If it needs changes, remove `need-human-review`, add `bug`, and leave review comments with the failing command and observed output.

<details>
<summary>Reproducible E2E evidence, redacted</summary>

​```
<contents of .sandcastle/evidence/issue-<ID>-e2e.log>
​```

</details>
```

If no evidence file exists, the comment must say: "No `.sandcastle/evidence/issue-<ID>-e2e.log` was committed for this run."
If live E2E was unavailable, include the exact unavailable credential or fixture reason from the branch comments or commit message.
If the issue body lacks concrete `## Test steps`, include: "Warning: this issue still needs reviewer-runnable test steps before it should be closed."

`gh issue comment <ID> --body-file <path-to-a-temp-file-containing-the-composed-body>` -- use a temp file rather than `--body` so the log's own backticks/newlines don't need shell escaping.

Here are all the issues:

{{ISSUES}}

Once you've merged everything you can, output <promise>COMPLETE</promise>.
