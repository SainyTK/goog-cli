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
4. If `.sandcastle/evidence/issue-<ID>-e2e.log` exists (it's now on the current branch after merging), read its contents. Issue branches are never pushed to GitHub, so a blob link to them is always dead -- embed the log's contents directly in the comment instead, inside a collapsible `<details>` block, so a reviewer sees it without leaving the issue.
5. Add a comment saying the Sandcastle implementation is ready for human review, with the evidence embedded per step 4 if the file exists.

Use these commands:

`gh issue edit <ID> --add-label "need-human-review"`

`gh issue edit <ID> --remove-label "bug" || true`

Build the comment body so the evidence log (if present) renders as:

```
<details>
<summary>E2E evidence</summary>

​```
<contents of .sandcastle/evidence/issue-<ID>-e2e.log>
​```

</details>
```

`gh issue comment <ID> --body-file <path-to-a-temp-file-containing-the-composed-body>` -- use a temp file rather than `--body` so the log's own backticks/newlines don't need shell escaping. The body should read: "Implemented by Sandcastle and ready for human review." followed by the evidence block (or, if no evidence file exists, a line noting E2E testing wasn't available for this run), followed by "Please test and close this issue if it passes. If it needs changes, remove need-human-review, add bug, and leave review comments."

Here are all the issues:

{{ISSUES}}

Once you've merged everything you can, output <promise>COMPLETE</promise>.
