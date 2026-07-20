---
name: implement
description: "Implement a piece of work from a user objective, repository plan, or specification."
disable-model-invocation: true
---

Implement the work described by the user's objective or the referenced repository plan.

For a large GnHF objective, choose one coherent, verifiable slice per iteration.
Read the existing plan and prior run notes before choosing the slice.
Update durable project documentation when the implementation changes an architectural decision or public workflow.

Use /tdd where possible, at pre-agreed seams.

Run `cargo check` and targeted tests regularly.
Run formatting, Clippy, and the full test suite before declaring the complete objective ready for human review.

Once done, use /review to review the work.

Commit each completed slice to the current branch with a message that explains the user-visible or architectural outcome.
