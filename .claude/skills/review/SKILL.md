---
name: review
description: Review changes since a fixed point along Standards and Spec axes using parallel sub-agents.
---

Two-axis review of the diff between `HEAD` and a fixed point the user supplies:

- **Standards** - does the code conform to this repo's documented coding standards?
- **Spec** - does the code faithfully implement the GnHF objective, repository plan, or other specification?

Both axes run as **parallel sub-agents** so they don't pollute each other's context, then this skill aggregates their findings.

## Process

### 1. Pin the fixed point

Whatever the user said is the fixed point, such as a commit SHA, branch name, tag, `main`, or `HEAD~5`.
If they did not specify one, ask for it.

Capture the diff command once: `git diff <fixed-point>...HEAD`.
The three-dot form compares against the merge base.
Also note the list of commits via `git log <fixed-point>..HEAD --oneline`.

Before going further, confirm the fixed point resolves (`git rev-parse <fixed-point>`) and the diff is non-empty.
A bad ref or empty diff should fail here before starting the parallel sub-agents.

### 2. Identify the spec source

Look for the originating spec in this order:

1. The objective or plan path supplied by the user or GnHF run.
2. A repository plan or specification under `docs/` that matches the branch name or changed feature.
3. The pull request body when reviewing a published branch.
4. The commit messages and GnHF run notes for the branch.
5. If nothing is found, ask the user where the spec is.
   If there is no spec, skip the Spec sub-agent and report "no spec available".

### 3. Identify the standards sources

Anything in the repo that documents how code should be written, such as `CODING_STANDARDS.md` or `CONTRIBUTING.md`.

### 4. Spawn both sub-agents in parallel

Send a single message with two `Agent` tool calls.
Use the `general-purpose` subagent for both.

**Standards sub-agent prompt** - include:

- The full diff command and commit list.
- The list of standards-source files you found in step 3.
- Ask it to report every place the diff violates a documented standard, organized by file and hunk where relevant.
- Ask it to cite the standard by file and rule.
- Ask it to distinguish hard violations from judgment calls, skip anything tooling enforces, and stay under 400 words.

**Spec sub-agent prompt** - include:

- The diff command and commit list.
- The path or fetched contents of the spec.
- Ask it to report missing or partial requirements, scope creep, and requirements whose implementation appears incorrect.
- Ask it to quote the relevant spec line for each finding and stay under 400 words.

If the spec is missing, skip the Spec sub-agent and note this in the final report.

### 5. Aggregate

Present the two reports under `## Standards` and `## Spec` headings, verbatim or lightly cleaned.
Do **not** merge or rerank findings because the two axes are deliberately separate.

End with a one-line summary containing the total findings per axis and the worst finding within each axis, if any.
Do not pick a single winner across axes because that would collapse the intended separation.

## Why two axes

A change can pass one axis and fail the other:

- Code that follows every standard but implements the wrong thing results in **Standards pass, Spec fail.**
- Code that satisfies the objective but breaks the project's conventions results in **Spec pass, Standards fail.**

Reporting them separately stops one axis from masking the other.
