# GnHF Workflow

GnHF is the default way to run substantial autonomous work in this repository.
The goal is to start with a clear objective, let the agent make and verify progress overnight, and return to a reviewable pull request branch.

## Prepare the objective

Use a repository plan when the work has multiple dependent slices or needs durable design context.
Keep architectural decisions in `CONTEXT.md` and `docs/adr/`.
The objective should describe the user outcome, relevant constraints, and the condition that means the work is complete.
It should leave the agent room to discover missing commands, awkward interfaces, and adjacent fixes that are necessary to deliver the outcome.

Before starting, make sure the base branch is clean and current.
Do not start an overnight run on top of unrelated local changes.

## Start the run

Use an isolated worktree and push successful iterations so the work survives outside the local run directory.

```sh
gnhf \
  --agent codex \
  --worktree \
  --push \
  --prevent-sleep on \
  --stop-when "The objective is complete, the full test suite passes, and the branch is ready for human review." \
  "Implement the objective in docs/<plan>.md. Work in coherent slices, test each slice, review the complete diff, and commit every completed slice."
```

Replace `docs/<plan>.md` with the real plan path, or put the complete objective directly in the prompt when a separate document would add no value.
Use limits such as `--max-iterations` or `--max-tokens` only when the run needs a deliberate budget.

GnHF stores local run state under `.gnhf/runs/`.
That directory is ignored because logs and prompts can contain local context and grow large.

## Expectations for each iteration

Each iteration should read the objective and previous run notes before choosing the next slice.
It should inspect the current implementation instead of assuming the next change from the plan is still correct.
It should complete one coherent slice, run focused checks, review the result, update durable documentation when needed, and commit the slice.

Bug fixes start with a reproduction through the real CLI.
Changes that depend on Google API behavior require live verification against a connected account when it can be done safely.
Live identifiers and account data stay out of committed files.

The agent may add commands or supporting features discovered during implementation when they are required for the objective.
Unrelated product ideas should be recorded in the run notes instead of being added to the branch.

## Morning review

Read the GnHF summary and inspect the full branch diff against its base.
Review the work against both the objective and the repository standards.
Run the full local gate:

```sh
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features
cargo test
```

Exercise the changed command through the real `goog` CLI.
For user-visible Google operations, verify the result in the relevant Google application when practical.

Open a pull request to the intended base branch after the review passes.
The pull request body should state the user outcome, important implementation decisions, automated checks, live verification performed, and anything the human reviewer still needs to inspect.

The human reviews the pull request, tests the functionality, and merges it when satisfied.
GitHub Issues, Sandcastle, and automatic use of `no-mistakes` are not part of this flow.
