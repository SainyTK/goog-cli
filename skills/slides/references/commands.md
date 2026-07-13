# Slides command guide

Use `target/debug/goog` in the repository examples below.
Replace it with `cargo run --` when the debug binary has not been built.

## Create and inspect

```bash
target/debug/goog slides create "Product strategy"
target/debug/goog slides get PRESENTATION_ID --json --fields 'presentationId,title,pageSize,slides(objectId,pageElements(objectId,size,transform,shape(text)))'
target/debug/goog slides slide create PRESENTATION_ID --object-id slide-01 --layout blank
```

Capture the presentation ID and URL from create output.
Use `slides get --json` to confirm slide page IDs and object IDs.
Always provide a narrow `--fields` selector because an unrestricted presentation response includes masters, layouts, notes, and theme data.

## Add content

```bash
target/debug/goog slides text-box PRESENTATION_ID --page-id slide-01 --object-id slide-01-title --text 'A focused roadmap compounds customer value' --x 48 --y 42 --width 624 --height 54
target/debug/goog slides text-box PRESENTATION_ID --page-id slide-01 --object-id slide-01-list --text $'First line\nSecond line' --x 48 --y 126 --width 260 --height 90
target/debug/goog slides image PRESENTATION_ID --page-id slide-01 --object-id slide-01-hero --url 'https://example.com/image.png' --x 360 --y 126 --width 312 --height 300
target/debug/goog slides shape PRESENTATION_ID --page-id slide-01 --object-id slide-01-accent --type rectangle --x 48 --y 118 --width 8 --height 300
target/debug/goog slides table PRESENTATION_ID --page-id slide-02 --object-id slide-02-table --rows 4 --columns 3 --x 54 --y 120 --width 612 --height 240
```

Use `table-fill` after creating a table.
Run its nested help because its accepted row input must match the current CLI build.

## Style and adjust objects

```bash
target/debug/goog slides object style --help
target/debug/goog slides object text-style --help
target/debug/goog slides object move --help
target/debug/goog slides object alt-text --help
```

Apply object fill, outline, text style, transforms, and alt text through these high-level commands.
Inspect the object after styling rather than assuming the mutation produced the intended result.

## Replace and maintain content

```bash
target/debug/goog slides replace-text PRESENTATION_ID --find '{{quarter}}' --replace 'Q3 2026'
target/debug/goog slides object insert-text --help
target/debug/goog slides object delete-text --help
target/debug/goog slides object replace-image --help
target/debug/goog slides object delete --help
```

## Visual QA

```bash
target/debug/goog slides deck inspect PRESENTATION_ID --qa-dir /tmp/product-strategy-qa --export-pdf /tmp/product-strategy.pdf --json
```

Open the generated montage for deck-level consistency.
Open every full-size slide image to inspect text and geometry.
The montage is not sufficient evidence for overflow or small alignment problems.

## Geometry

A standard widescreen slide is 720 points wide and 405 points high.
Keep a safety margin of roughly 36 to 54 points unless the design intentionally uses full bleed.
Use a small set of repeated x positions, widths, and vertical gaps to create alignment.
Do not solve crowding by repeatedly shrinking text.
Reduce content, increase the number of slides, or change the layout first.

## Learn unfamiliar commands

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
