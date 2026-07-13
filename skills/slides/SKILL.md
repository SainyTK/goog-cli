---
name: slides
description: Create, edit, inspect, and visually verify native Google Slides presentations with the goog CLI. Use for production-grade pitch decks, briefings, reports, training decks, proposals, and any request involving Google Slides content, layout, or styling.
---

# Google Slides

Create and edit native presentations with `goog slides`.
Use `target/debug/goog` from the goog-cli repository, or `cargo run --`, while developing the CLI.

## Required workflow

1. Define the audience, presentation job, central takeaway, and required evidence.
2. Plan a cumulative narrative in which every slide has one primary job.
3. Choose a coherent visual system before creating slide objects.
4. Run `target/debug/goog auth list` and record the active account before any live mutation.
5. Run `target/debug/goog slides --help` and the relevant nested `--help` command before the first mutation.
6. Create the presentation and slides with stable object IDs for important elements.
7. Add content with high-level `goog slides` commands.
8. Style and position objects with `goog slides object` commands.
9. Inspect the live deck with `goog slides get` during authoring.
10. Run `goog slides deck inspect` into a task-local QA directory.
11. Open the montage and every full-size slide thumbnail, then fix overlap, clipping, awkward wrapping, alignment, density, and inconsistent styling.
12. Repeat inspection after every layout correction until the deck is clean.
13. Return the Google Slides URL and a concise summary of the finished deck.

Read [references/content-and-design.md](references/content-and-design.md) before planning or authoring a deck.
Read [references/commands.md](references/commands.md) for native command patterns and geometry.

## Authoring rules

- Use the active account shown by `goog auth list` unless the user specifies another authorized account.
- Pass `--account EMAIL` for every live command when account selection must remain explicit.
- If authorization opens an account chooser, select only the recorded active or user-specified account.
- Never infer an account from browser order, a remembered identity, or unrelated open presentations.
- If a command fails because the account is missing required scopes, run `goog auth login` once and retry; do not expect the original command to pause and resume on its own.
- Prefer `slide create`, `text-box`, `image`, `shape`, `line`, `table`, and `object` commands over `batch-update`.
- Use `batch-update` only when no high-level command exposes a required native feature.
- Use stable, descriptive object IDs so later edits are deterministic.
- Treat positions and sizes as deliberate points on a 10 by 5.625 inch widescreen canvas unless the live presentation reports another page size.
- Keep all elements inside the slide canvas.
- Use images only from publicly reachable URLs accepted by Google Slides.
- Set alt text for meaningful images and objects.
- Never place temporary instructions, source notes, or QA commentary on audience-facing slides.
- Run the exact nested `--help` command before using an unfamiliar mutation.
- Never hide a failed mutation with `|| true` or continue as if it succeeded.
- Use true newline characters for multiline text and verify the inspection report does not contain literal backslash-n sequences.
- Use `slides get --fields` to request only slide IDs, object IDs, text, transforms, and page size needed for the current check.

## Completion gate

Do not call the presentation finished until all of these are true:

- The deck has a clear opening, coherent progression, and deliberate close.
- Every slide advances the story and has one primary claim.
- Titles communicate takeaways rather than generic topics where appropriate.
- Visuals and charts support the claim and do not invent evidence.
- No text overlaps, clips, overflows, or becomes uncomfortably small.
- Object alignment, spacing, type hierarchy, colors, and page furniture are consistent.
- Tables remain readable and do not contain paragraph-length prose.
- No placeholder or production-note text remains.
- `goog slides deck inspect` completed after the last mutation.
- Every final slide image was inspected at full size.
- The final URL opens the intended native Google Slides deck.
