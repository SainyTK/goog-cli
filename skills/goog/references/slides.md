# Google Slides operations

## Contents

- Required workflow
- Content and narrative
- Design and layout
- Authoring safety
- Completion gate
- Command guide
  - Create and inspect
  - Add content
  - Style and adjust objects
  - Replace and maintain content
  - Visual QA
  - Geometry
  - Learn unfamiliar commands

## Required workflow

1. Define the audience, presentation job, central takeaway, and required evidence.
2. Plan a cumulative narrative in which every slide has one primary job.
3. Choose a coherent visual system before creating slide objects.
4. Run `goog auth list` and record the active account before any live mutation.
5. Run `goog slides --help` and the relevant nested `--help` command before the first mutation.
6. Create the presentation and slides with stable object IDs for important elements.
7. Add content with high-level `goog slides` commands.
8. Style and position objects with `goog slides object` commands.
9. Inspect the live deck with `goog slides get` during authoring.
10. Run `goog slides deck inspect` into a task-local QA directory.
11. Open the montage and every full-size slide thumbnail, then fix overlap, clipping, awkward wrapping, alignment, density, and inconsistent styling.
12. Repeat inspection after every layout correction until the deck is clean.
13. Return the Google Slides URL and a concise summary of the finished deck.

## Content and narrative

Infer the intended audience rather than assuming it is the user.
Define what the audience should understand, believe, choose, approve, discuss, or do by the end.
Identify the central takeaway and the evidence required to support it.
For a neutral or exploratory deck, define the questions the audience should leave able to answer.

Choose an arc suited to the job, such as context to stakes to evidence to action, question to analysis to answer, or current state to change to future state.
An agenda alone is not a narrative.
Make the sequence cumulative so each slide creates the need for the next.
Open with the context, purpose, question, or tension that makes the presentation worth attention.
Close by resolving the opening with a decision, action, synthesis, application, or productive next question.

Give each slide one narrative job and one primary claim.
Prefer takeaway titles that state the point over labels that merely name the topic.
Remove repeated beats and content that does not change understanding or action.
Show what important evidence means and why it matters to this audience.
Never invent people, quotations, facts, metrics, sources, or outcomes.

## Design and layout

Choose a restrained palette, a clear type hierarchy, and a small set of reusable spacing values.
Use a minimum body size that remains comfortable in the rendered full-size slide.
Increase 10 or 11 point audience-facing labels unless they are truly secondary metadata and remain legible in the full-size render.
Prefer short audience-facing copy over dense paragraphs.
Use visual variety when the information calls for it, while keeping the deck recognizably coherent.
Avoid repeating the same card grid or left-text-right-image composition on every slide.
Use diagrams only when relationships are materially clearer than prose or a simple table.

Align objects to shared edges and baselines.
Use consistent margins and visible breathing room between unrelated elements.
Crop images intentionally and preserve their aspect ratio.
Do not stretch raster images.
Keep charts honest, legible, and labeled with units and time periods.
Keep tables compact, highlight the comparison that matters, and move detailed backup tables to an appendix when needed.

Inspect the deck-level montage for rhythm, repetition, and abrupt style changes.
Inspect every slide at full size for clipping, overlap, wrapping, alignment, image quality, and unreadably small text.
Check that titles, footers, page markers, and repeated components sit in consistent positions.
Keep decision lines and recommendations in a dedicated footer band with clear separation from body content and the canvas edge.
Fix the actual layout rather than accepting defects because the content is technically present.
Run a fresh inspection after the final correction.

## Authoring safety

- Use the active account shown by `goog auth list` unless the user specifies another authorized account.
- Pass `--account EMAIL` for every live command when account selection must remain explicit.
- If authorization opens an account chooser, select only the recorded active or user-specified account.
- Never infer an account from browser order, a remembered identity, or unrelated open presentations.
- If a command fails because the account is missing required scopes, run `goog auth login` once and retry.
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

## Command guide

Use `target/debug/goog` in the repository examples below.
Replace it with `cargo run --` when the debug binary has not been built.

### Create and inspect

```bash
target/debug/goog slides create "Product strategy"
target/debug/goog slides get PRESENTATION_ID --json --fields 'presentationId,title,pageSize,slides(objectId,pageElements(objectId,size,transform,shape(text)))'
target/debug/goog slides slide create PRESENTATION_ID --object-id slide-01 --layout blank
```

Capture the presentation ID and URL from create output.
Use `slides get --json` to confirm slide page IDs and object IDs.
Always provide a narrow `--fields` selector because an unrestricted presentation response includes masters, layouts, notes, and theme data.

### Add content

```bash
target/debug/goog slides text-box PRESENTATION_ID --page-id slide-01 --object-id slide-01-title --text 'A focused roadmap compounds customer value' --x 48 --y 42 --width 624 --height 54
target/debug/goog slides text-box PRESENTATION_ID --page-id slide-01 --object-id slide-01-list --text $'First line\nSecond line' --x 48 --y 126 --width 260 --height 90
target/debug/goog slides image PRESENTATION_ID --page-id slide-01 --object-id slide-01-hero --url 'https://example.com/image.png' --x 360 --y 126 --width 312 --height 300
target/debug/goog slides shape PRESENTATION_ID --page-id slide-01 --object-id slide-01-accent --type rectangle --x 48 --y 118 --width 8 --height 300
target/debug/goog slides table PRESENTATION_ID --page-id slide-02 --object-id slide-02-table --rows 4 --columns 3 --x 54 --y 120 --width 612 --height 240
```

Use `table-fill` after creating a table.
Run its nested help because its accepted row input must match the current CLI build.

### Style and adjust objects

```bash
target/debug/goog slides object style --help
target/debug/goog slides object text-style --help
target/debug/goog slides object move --help
target/debug/goog slides object alt-text --help
```

Apply object fill, outline, text style, transforms, and alt text through these high-level commands.
Inspect the object after styling rather than assuming the mutation produced the intended result.

### Replace and maintain content

```bash
target/debug/goog slides replace-text PRESENTATION_ID --find '{{quarter}}' --replace 'Q3 2026'
target/debug/goog slides object insert-text --help
target/debug/goog slides object delete-text --help
target/debug/goog slides object replace-image --help
target/debug/goog slides object delete --help
```

### Visual QA

```bash
target/debug/goog slides deck inspect PRESENTATION_ID --qa-dir /tmp/product-strategy-qa --export-pdf /tmp/product-strategy.pdf --json
```

Open the generated montage for deck-level consistency.
Open every full-size slide image to inspect text and geometry.
The montage is not sufficient evidence for overflow or small alignment problems.

### Geometry

A standard widescreen slide is 720 points wide and 405 points high.
Keep a safety margin of roughly 36 to 54 points unless the design intentionally uses full bleed.
Use a small set of repeated x positions, widths, and vertical gaps to create alignment.
Do not solve crowding by repeatedly shrinking text.
Reduce content, increase the number of slides, or change the layout first.

### Learn unfamiliar commands

```bash
target/debug/goog slides --help
target/debug/goog slides slide --help
target/debug/goog slides object --help
target/debug/goog slides deck inspect --help
```

Pass each command segment as its own shell argument.
Do not quote an entire command path.
Do not append `|| true` to mutations.
Treat every nonzero exit as a failed operation that must be understood before continuing.
