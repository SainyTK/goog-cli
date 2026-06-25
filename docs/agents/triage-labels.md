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

Whenever applying `ready-for-agent` to an issue, also apply the `Sandcastle` label.

## Sandcastle workflow labels

| Label               | Meaning                                                                 |
| ------------------- | ----------------------------------------------------------------------- |
| `Sandcastle`        | Issue is eligible for the Sandcastle automation loop                    |
| `need-human-review` | Sandcastle merged an implementation and a human needs to test/review it |
| `need-fix`          | Human review found required changes and Sandcastle should run again     |

Sandcastle keeps issues open after implementation. After merging, it adds `need-human-review` and removes `need-fix`. A human closes the issue if it passes. If it needs more work, the human removes `need-human-review`, adds `need-fix`, and leaves review comments for the next implementation pass.

Edit the right-hand column to match whatever vocabulary you actually use.
