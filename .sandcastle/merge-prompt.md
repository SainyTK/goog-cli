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
2. Remove `need-fix` if it is present.
3. Leave the issue open.
4. Add a short comment saying the Sandcastle implementation is ready for human review.

Use these commands:

`gh issue edit <ID> --add-label "need-human-review"`

`gh issue edit <ID> --remove-label "need-fix" || true`

`gh issue comment <ID> --body "Implemented by Sandcastle and ready for human review. Please test and close this issue if it passes. If it needs changes, remove need-human-review, add need-fix, and leave review comments."`

Here are all the issues:

{{ISSUES}}

Once you've merged everything you can, output <promise>COMPLETE</promise>.
