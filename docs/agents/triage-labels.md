# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

`ready-for-agent` and `bug` are the eligibility gates for the Sandcastle automation loop -- either one makes an issue eligible. There is no separate `Sandcastle` label. `.sandcastle/plan-prompt.md` selects issues via `gh issue list --search "label:ready-for-agent,bug"`. A `bug` report is presumed ready for an agent to fix without the extra `ready-for-agent` triage step.

## Sandcastle workflow labels

| Label               | Meaning                                                                 |
| ------------------- | ----------------------------------------------------------------------- |
| `need-human-review` | Sandcastle merged an implementation and a human needs to test/review it |
| `bug`                | Reported bug ready for an agent to fix, or human review found required changes and Sandcastle should run again |

Sandcastle keeps issues open after implementation. After merging, it adds `need-human-review` and removes `bug`. A human closes the issue if it passes. If it needs more work, the human removes `need-human-review`, adds `bug`, and leaves review comments for the next implementation pass.

Edit the right-hand column to match whatever vocabulary you actually use.
