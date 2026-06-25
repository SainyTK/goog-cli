## Parent

Parent issue: #34

## What to build

Add image discovery and inline image insertion: `goog docs list-images` and `goog docs insert-image`. Image discovery should include both Inline Images and Positioned Images, while high-level insertion should create Inline Images in the document text flow.

`list-images` should expose Image Handles, image kind, Document Locations for Inline Images, object IDs and layout metadata for Positioned Images, and readable previews. `insert-image` should use the same Location Selector, ambiguity handling, dry-run, JSON dry-run, and Revision Guard behavior as the text write commands.

## Acceptance criteria

- [ ] `goog docs list-images DOC_ID` prints a human-readable list of Inline Images and Positioned Images.
- [ ] `goog docs list-images DOC_ID --json` emits structured Image Handles, kinds, locations, object IDs, and layout metadata.
- [ ] Inline Images include Document Locations with index, page, content line, and confidence where available.
- [ ] Positioned Images include object IDs and layout metadata without pretending they have normal text-flow insertion points.
- [ ] `goog docs insert-image DOC_ID PATH --page PAGE --line LINE` creates an Inline Image at the resolved Document Location.
- [ ] `insert-image` supports exact `--index` targeting.
- [ ] `insert-image --dry-run` shows a Human Preview with an image placeholder in context and does not call the mutation endpoint.
- [ ] `insert-image --dry-run --json` emits the resolved target, revision ID, and native Batch Update request body.
- [ ] `--required-revision-id` adds a Revision Guard to the native request body.
- [ ] Tests cover documents containing both Inline Images and Positioned Images.
- [ ] Existing raw `docs get` and `docs batch-update` behavior remains unchanged.

## Blocked by

- #35
- #36
- #37
