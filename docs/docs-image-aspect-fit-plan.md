## Summary

Add aspect-ratio-preserving image sizing based on maximum dimensions so users can fit screenshots and diagrams into Google Docs pages without pre-calculating width and height.

## User problem

The current development command requires both exact dimensions:

```text
goog docs image insert DOCUMENT_ID IMAGE_URI --width 468 --height 590 --at SELECTOR
```

This forces the caller to inspect the source image, calculate its aspect ratio, determine the available page area, and choose dimensions that do not distort or spill onto another page.

In a real proposal workflow, 15 application screenshots ranged from 1440 by 1047 pixels to 1440 by 2534 pixels.
Using a shared height of 590 points caused six caption spillovers and six nearly blank pages.
Those six images had to be deleted and reinserted at 500 points high with individually calculated widths.

This calculation belongs in the CLI.

## Proposed command surface

Support aspect-fit constraints:

```text
goog docs image insert DOCUMENT_ID IMAGE_URI \
  --max-width 468 \
  --max-height 500 \
  --preserve-aspect-ratio \
  --at SELECTOR
```

For local-file insertion, the same flags should work with `--file` after local-file support exists.

`--preserve-aspect-ratio` should be the default whenever only maximum dimensions are provided.

Keep exact sizing available:

```text
--width 468 --height 500
```

If exact width and height would distort the source, require an explicit `--allow-distortion` flag or emit a clear warning in human and JSON output.

Consider a page-aware convenience mode:

```text
--fit-page
--reserve-height 72
```

`--fit-page` should derive maximum width from the active section's page size and margins.
`--reserve-height` should reserve space for a heading, caption, or other content on the same page.

## Dimension resolution requirements

- Read PNG, JPEG, GIF, and WebP dimensions without decoding the entire image into an unbounded buffer.
- Use one documented unit conversion policy between source pixels and Google Docs points.
- Preserve the source aspect ratio unless distortion is explicitly allowed.
- Scale down to satisfy both maximum constraints.
- Do not upscale by default.
- Add `--allow-upscale` for callers that need it.
- Reject zero, negative, NaN, infinite, or unreasonably large dimensions.
- Report the source dimensions, scale factor, and final point dimensions in `--dry-run --json`.
- Use deterministic rounding so repeated runs produce the same object size.
- Keep existing exact `--width` and `--height` behavior compatible.
- Document how EXIF orientation affects JPEG dimension calculation.

## Page-layout requirements

- `--fit-page` must use the page size and margins from the correct tab and section.
- Pageless documents must return an actionable error or use explicit maximum dimensions.
- Header and footer insertion must use the containing segment's available width rather than body margins.
- The command must not claim that an image fits when its paragraph spacing and requested reserve height exceed the available area.
- Dry-run output must show the page geometry used in the calculation.

## Tests

### Dimension tests

- A 1440 by 2534 image with `--max-width 468 --max-height 500` resolves to approximately 284 by 500 points without distortion.
- A 1440 by 1047 image with the same constraints resolves to approximately 468 by 340 points.
- A small image is not upscaled by default.
- `--allow-upscale` permits deterministic enlargement.
- One maximum dimension constrains the other dimension through the aspect ratio.
- Exact width and height remain supported.
- Distorting exact dimensions require `--allow-distortion` or produce a documented warning.
- Invalid numeric values fail before the Docs mutation.
- EXIF-rotated JPEG input uses its displayed orientation.

### Document tests

- `--fit-page` reads Letter and A4 page geometry correctly.
- Section-specific margins are honored.
- Pageless documents produce the documented diagnostic.
- Header and footer image constraints use segment geometry.
- `--dry-run --json` includes native dimensions, calculated dimensions, scale, page geometry, and whether upscaling occurred.

### Live acceptance test

- Insert a set of portrait and landscape screenshots into separate page-break-before sections.
- Export the document to PDF.
- Verify that each heading, image, and caption remains on one page.
- Verify that the export contains no blank spillover pages.
- Verify `goog docs map --type images --json` reports the calculated geometry.

## Success criteria

- Users can fit mixed-aspect images into a bounded rectangle without external image inspection tools or manual arithmetic.
- No inserted image is distorted unless the caller explicitly permits distortion.
- The 1440 by 2534 acceptance fixture fits within 468 by 500 points and remains on one page with reserved caption space.
- Dry-run explains every sizing decision in stable JSON.
- Existing exact sizing commands continue to work.
- Page-aware fitting handles section geometry and fails safely for pageless documents.
