# Google Slides Authoring Improvement Plan

Status: Proposed

Date: 2026-07-10

Target product: `goog slides`

## Outcome

`goog-cli` should be able to create, update, verify, preview, and export a polished native Google Slides presentation from one declarative Deck Source without requiring JavaScript, Python, `jq`, `curl`, direct token access, or handwritten Google Slides Batch Update JSON.

The main workflow should be one command:

```sh
goog slides deck apply \
  --source responsible-ai.yaml \
  --title "Responsible AI Framework - Banking Virtual Assistant" \
  --account tanakorn.k@siametrics.com \
  --qa-dir ./qa \
  --export-pptx ./responsible-ai.pptx
```

The command should validate the source, create or update the presentation, reconcile managed slides and objects, wait for Google to expose the applied state, fetch live thumbnails, build a montage, run structural quality checks, export the Google-rendered PowerPoint file, and return the presentation URL and a concise report.

## Executive summary

The existing Slides commands are useful for direct object operations, but they expose too much of the authoring implementation to callers.

Creating a good deck still requires the caller to invent a layout system, calculate coordinates, generate stable object IDs, turn each visual element into several native requests, split requests into batches, reset an existing presentation, retry reads during Google consistency delays, fetch thumbnails outside the CLI, construct a montage, and export or render a second presentation format.

The responsible AI presentation demonstrates the cost of that gap.

It required a 1,002-line JavaScript authoring program, a separate 29-line reset program, 14 generated request files, 1,692 Google Slides requests, and 14 files each for local previews, layout inspection, live thumbnails, and PowerPoint renders.

The generated Google request files alone totaled about 804 KB.

The result looked good because the script created a design system and a repeatable visual grammar.

Those reusable mechanics belong inside `goog-cli`.

The recommended design is a deep Deck Authoring module with a small interface centered on `deck check`, `deck apply`, and `deck inspect`.

The module should accept a semantic Deck Source, compile it into native Google Slides operations, reconcile it idempotently against a Managed Presentation, and produce a Quality Report and exported artifacts.

Existing one-object commands and raw `batch-update` remain available as escape hatches.

## Current-state evidence

### Existing strengths

The current CLI already provides the necessary foundation:

- Multi-account authorization and explicit `--account` selection.
- Unified Access and Resource Account Mapping for existing presentations.
- Presentation creation, listing, raw reads, and raw Batch Update.
- High-level commands for slides, text boxes, images, video, tables, shapes, lines, text styling, object styling, movement, ordering, grouping, deletion, and text replacement.
- A hand-written HTTP adapter that keeps Google request support focused on the product's needs.
- Wiremock-backed command and transport tests.
- A repository rule that requires live E2E verification before regression tests for Google behavior.

These capabilities should be reused.

The plan does not replace them with a second Slides client.

### Work the caller had to implement

The responsible AI deck authoring program implemented the following behavior outside `goog-cli`:

- A shared color palette, type hierarchy, margins, rules, panels, and accent treatments.
- Reusable slide chrome with eyebrow text, title, subtitle, sources, and page number.
- Reusable numbered lists and repeated visual structures.
- A 1280 by 720 virtual canvas and a separate conversion into Google point units.
- Font-size translation between a local PowerPoint renderer and Google Slides.
- Stable page and object ID generation.
- Native request compilation for every text box and shape.
- Five Google requests for most text elements: create shape, insert text, update text style, update paragraph style, and update shape properties.
- A reset pass that inspected the live presentation, deleted current elements and slides, then created the required blank slides.
- One Batch Update file per slide to keep large request groups manageable.
- A local PowerPoint render path and a separate Google Slides request path.
- Per-slide PNG previews, layout JSON, inspection output, montages, live Google thumbnails, PowerPoint renders, and final visual review.

This behavior belongs to the product rather than presentation-specific business content.

### Current interface depth problem

The current one-object commands are intentionally direct.

For example, adding a polished title block can require creating a text box, inserting text, styling the text, styling its paragraph, setting vertical alignment, adding an accent shape, positioning every object, and tracking the generated IDs.

The caller must know almost as much as the implementation about Google request ordering and geometry.

The proposed Deck Authoring module passes the deletion test.

If the module were removed, the layout, compilation, reconciliation, retry, export, and QA logic would reappear in every serious presentation session.

## Product objective

Make a polished native Google Slides deck a normal `goog-cli` artifact rather than a custom-programming task.

The CLI should own the mechanics of presentation construction and lifecycle management.

The caller should own the narrative, slide content, selected visual patterns, theme choices, and any explicit freeform layout decisions.

### Success criteria

The implementation is successful when all of the following are true:

- A user or agent can author the responsible AI benchmark deck without writing or running an additional program.
- The only required authoring input is a Deck Source read from YAML, JSON, or stdin.
- A single `deck apply` invocation can create the Google Slides file, populate it, verify it, fetch live previews, and export it to PowerPoint.
- The command never requires the caller to read `~/.goog/auth.json` or handle an access token directly.
- Reapplying an unchanged Deck Source produces zero Slides mutations after the live state is read.
- Editing one keyed slide changes only that slide and any presentation-level metadata affected by the edit.
- Stable slide and element keys produce stable Google object IDs across sessions.
- Failed or interrupted applies can be rerun safely.
- Live preview generation works for every slide and produces a deterministic montage order.
- The Quality Report identifies out-of-bounds elements, invalid geometry, unsafe destructive changes, missing required text, missing alt text where configured, unsupported image sources, and likely text-fit problems before the command reports success.
- PowerPoint and PDF exports come from the native Google presentation, so the CLI does not maintain a second presentation renderer.
- Existing `goog slides` commands and `goog slides batch-update` continue to work without breaking changes.

### Benchmark acceptance case

The responsible AI deck should become a checked-in, sanitized E2E benchmark source with no real presentation ID, account email, local temporary path, or business secret.

The benchmark contains 14 slides and exercises cover, statement, comparison, card grid, process, timeline, evidence table, source list, and closing patterns.

Applying it to a scratch presentation should prove that the implementation can replace the former 1,692-request scripted workflow.

The test should assert semantic outcomes such as slide count, managed element count, text presence, stable IDs, successful thumbnail retrieval, and export file signatures instead of committing a live Google response or a real resource URL.

## Scope

### Included

- A versioned declarative Deck Source.
- Theme tokens and reusable slide patterns.
- A layout engine for common presentation structures.
- A freeform escape hatch for explicit positioning.
- Compilation into native Google Slides requests.
- Stable IDs and managed-object ownership.
- Create and update workflows.
- Idempotent reconciliation.
- Safe stale-object deletion after successful creation and update.
- Request chunking, retry, polling, and resumable reruns.
- Live thumbnails and montage generation.
- Native Google export to PowerPoint and PDF through Drive export.
- Structural and heuristic quality checks.
- Human-readable output and stable JSON output.
- Local checks, mock-adapter tests, and live E2E verification.
- Documentation, examples, domain vocabulary, and an ADR.

### Excluded from the first complete release

- Generating the narrative with an embedded language model.
- Replacing Google Slides as the final renderer.
- Pixel-perfect reproduction of arbitrary PowerPoint templates.
- Animations, transitions, audio, and complex embedded charts.
- Collaborative merge resolution when a person edits an object managed by `goog` while the Deck Source changes the same object.
- Silent publication of local images to the public internet.
- Automatic judgment that a narrative is persuasive, factually correct, or well sourced.

These exclusions keep the first release focused on deterministic authoring and verifiable presentation mechanics.

## Product principles

### Google Slides is the rendering source of truth

The prior workflow had one local PowerPoint renderer and another Google request compiler.

That created font scaling rules and layout deviations between outputs.

The improved workflow should build the native Google presentation first, then use Google's Drive export endpoint for PowerPoint and PDF.

There should be one visual object model and one final renderer.

### Semantic patterns are the normal interface

Most slides should describe intent such as cover, comparison, cards, process, timeline, matrix, sources, or closing.

Users should not need to specify coordinates for those patterns.

Freeform positioning remains available for exceptional slides.

### Stable keys own identity

Every managed slide and element has a caller-provided key.

The CLI converts the key into a valid deterministic Google object ID.

Content and style changes do not change identity.

### Reconciliation replaces reset scripts

The CLI should compare the desired Deck Source with the live Managed Presentation and calculate a minimal Apply Plan.

It should not delete the whole presentation and rebuild it for every edit.

### Verification is part of apply

Successful HTTP responses are insufficient evidence that a deck is ready.

The default apply path should read the final state, verify managed objects, fetch every live thumbnail, and produce a Quality Report.

### Destructive behavior is explicit

The default update mode may change or delete only objects owned by the Deck Source.

Adopting or replacing unmanaged content requires an explicit flag and a preview of the affected objects.

### Raw coverage remains available

Semantic authoring will never cover every Google Slides feature.

`batch-update` and the existing object commands remain supported for advanced cases.

The Deck Source may also include a narrowly scoped `nativeRequests` escape hatch, but use of that field must produce a warning and must not weaken ownership checks.

## Alternatives considered

### Continue adding one-object commands

This would improve coverage for individual Slides operations and should continue where users need direct editing.

It does not solve full-deck layout, shared design tokens, stable identity, request batching, reconciliation, live preview, or QA.

The responsible AI workflow would still need a caller-owned orchestration program.

### Provide reusable shell or JavaScript helpers

This would reduce repeated typing while preserving the current low-level interface.

It would also add a second runtime, expose Tokens or request files to more processes, weaken cross-platform installation, and keep correctness outside the Rust test surface.

This conflicts with the desired installed-binary workflow.

### Accept only raw Google Batch Update JSON

This is already available and remains valuable for complete native coverage.

Raw JSON cannot express semantic intent, automatic layout, stable ownership, or a minimal update without the producer rebuilding those systems.

### Generate PowerPoint locally and import it

PowerPoint libraries provide useful authoring features, but importing a generated file creates a new conversion path and can lose native Slides behavior.

Updating an existing Google presentation by stable managed identity also becomes difficult.

The safer direction is to create native Slides objects and export PowerPoint from Google after verification.

### Use a declarative Deck Source with built-in reconciliation

This is the recommended approach.

It preserves native Google Slides, gives humans and agents a compact authoring interface, centralizes layout and reliability behavior, and leaves raw requests available for unusual features.

## Proposed user interface

### Command group

Add a `deck` namespace under `goog slides`:

```text
goog slides deck check
goog slides deck apply
goog slides deck inspect
```

Keep the main interface small.

Template extraction, export, thumbnail retrieval, montage creation, and reconciliation are behavior behind these commands rather than separate steps users must compose.

### `goog slides deck check`

Purpose: Parse, normalize, lay out, compile, and quality-check a Deck Source without mutating Google.

Example:

```sh
goog slides deck check \
  --source responsible-ai.yaml \
  --report ./qa/check.json
```

Required behavior:

- Validate the source schema and report file, line, and field paths.
- Resolve theme tokens and pattern defaults.
- Calculate all element bounds.
- Detect duplicate keys and generated-ID collisions.
- Estimate text fit using the configured font and box dimensions.
- Validate image source policy.
- Compile the complete Apply Plan in memory.
- Emit a readable summary by default.
- Emit the normalized Deck Source, compiled plan, and Quality Report with `--json`.
- Return a nonzero exit code for errors and a distinct documented exit code for warning-only quality failures when `--fail-on-warning` is used.

Useful flags:

```text
--source PATH_OR_DASH
--report PATH
--json
--fail-on-warning
--strict
```

### `goog slides deck apply`

Purpose: Create or update a Managed Presentation, verify the live result, and optionally export artifacts.

Create example:

```sh
goog slides deck apply \
  --source responsible-ai.yaml \
  --title "Responsible AI Framework - Banking Virtual Assistant" \
  --account tanakorn.k@siametrics.com \
  --qa-dir ./qa \
  --export-pptx ./responsible-ai.pptx \
  --export-pdf ./responsible-ai.pdf
```

Update example:

```sh
goog slides deck apply \
  --source responsible-ai.yaml \
  --presentation PRESENTATION_ID \
  --qa-dir ./qa
```

Exactly one of `--title` or `--presentation` is required.

Required behavior:

- Run every `deck check` preflight before the first mutation.
- Create a new presentation when `--title` is supplied.
- Use Unified Access when an existing presentation is supplied without `--account`.
- Read the live presentation and its management metadata.
- Refuse to mutate an unmanaged presentation unless `--adopt` or `--replace-all` is explicit.
- Show the destructive scope before adoption or replacement.
- Compile a minimal Apply Plan.
- Apply creates and updates before deleting stale managed objects.
- Split native requests without breaking request dependencies.
- Retry rate limits and transient server failures with bounded exponential backoff and jitter.
- Poll final state until expected slide and object identities are readable or the consistency timeout expires.
- Treat a duplicate object ID after a retry as a reconciliation signal, then read and compare the object instead of blindly failing.
- Fetch one live thumbnail per slide.
- Build a montage in source order.
- Verify slide count, managed keys, expected text, and object bounds against the final live read.
- Export PowerPoint or PDF from the live Google presentation when requested.
- Save a machine-readable apply journal and Quality Report under `--qa-dir`.
- Print the account used, presentation URL, mutation counts, warning counts, preview paths, export paths, and final result.
- Return a structured JSON report with `--json`.

Useful flags:

```text
--source PATH_OR_DASH
--title TITLE
--presentation ID_OR_URL
--account EMAIL
--qa-dir DIR
--export-pptx PATH
--export-pdf PATH
--dry-run
--fail-on-warning
--adopt
--replace-all
--yes
--consistency-timeout SECONDS
--json
```

`--dry-run` should fetch the target when one is supplied and print the exact semantic changes without calling Batch Update.

`--yes` should be required only for noninteractive destructive modes.

Normal managed reconciliation should remain noninteractive so agents can use it reliably.

### `goog slides deck inspect`

Purpose: Produce a complete inspection and QA bundle for any presentation without changing it.

Example:

```sh
goog slides deck inspect PRESENTATION_ID \
  --qa-dir ./qa \
  --export-pptx ./presentation.pptx
```

Required behavior:

- Read all slides and page elements required for structural inspection.
- Fetch every live thumbnail through the authorized Slides client.
- Build a numbered montage.
- Extract visible text in slide order.
- Report page and element dimensions, transforms, object IDs, managed keys, and alt text.
- Flag out-of-bounds objects, unexpected empty slides, overlapping objects that violate source constraints, missing expected text when a source is provided, missing alt text where policy requires it, and likely text overflow.
- Export PowerPoint or PDF when requested.
- Work for unmanaged presentations, with management-specific fields shown as unavailable.

Useful flags:

```text
--source PATH_OR_DASH
--qa-dir DIR
--export-pptx PATH
--export-pdf PATH
--fail-on-warning
--json
```

## Deck Source

### Format

Support YAML and JSON with the same versioned schema.

YAML is the recommended human authoring format.

JSON is useful for agents and programmatic producers.

The command accepts a path or `-` for stdin.

The CLI should detect the format from the extension or parse JSON first and YAML second for stdin.

The source is declarative data, and executable behavior is prohibited.

It cannot run shell commands, read environment variables implicitly, or evaluate expressions beyond documented token references.

### Example

```yaml
schemaVersion: 1
presentation:
  aspectRatio: wide
  language: en

theme:
  colors:
    canvas: "#FFFFFF"
    ink: "#111111"
    text: "#292929"
    muted: "#555555"
    panel: "#EDEDED"
    rule: "#B8BCC4"
    accent: "#FF6B35"
  fonts:
    heading: Arial
    body: Arial
  spacing:
    pageMargin: 42

slides:
  - key: cover
    pattern: cover
    eyebrow: BANKING VIRTUAL ASSISTANT
    title: Responsible AI measurement for banking virtual assistants
    subtitle: A practical framework for testing, release approval, and live monitoring
    footer: Risk-tiered evidence | Independent challenge | Customer outcome monitoring

  - key: why-measurement-changes
    pattern: statement-and-list
    title: Generic model scores miss banking harm
    statement: A correct sentence can still create an unsafe banking outcome.
    items:
      - title: Wrong information
        body: Fees, limits, rates, or product terms conflict with the approved source.
      - title: Unauthorized action
        body: A tool changes money, access, or obligations without valid confirmation.
    takeaway: Measure the conversation, the action, and the customer outcome.
```

### Schema structure

The first schema version should include:

- `presentation`: aspect ratio, language, default speaker-note policy, and optional metadata.
- `theme`: named colors, fonts, type styles, spacing, lines, fills, and pattern defaults.
- `slides`: ordered slide definitions with stable keys.
- `assets`: named image sources with URL, checksum, alt text, and placement policy.
- `layouts`: optional caller-defined reusable layouts composed from safe primitives.
- `quality`: minimum font sizes, contrast rules, safe areas, allowed overlap groups, and required alt-text policy.

Unknown fields should be rejected by default so misspellings do not silently change a deck.

A future schema version may add fields without changing how version 1 is interpreted.

### Theme tokens

Theme values should be referenced by name rather than copied into each slide.

Required token groups:

- Colors.
- Font families and fallback families.
- Type styles with size, weight, line spacing, alignment, and color.
- Spacing values.
- Shape fills and outlines.
- Rules and dividers.
- Safe-area and footer geometry.

The compiler should resolve all tokens before layout.

The normalized source emitted by `deck check --json` should contain resolved values for debugging.

### Built-in slide patterns

The first complete release should support the patterns needed by the benchmark and common business decks:

- `cover`.
- `section`.
- `statement`.
- `statement-and-list`.
- `comparison` with two or more columns.
- `cards` with automatic rows and columns.
- `process` with ordered stages.
- `timeline` with ordered milestones.
- `matrix` with row and column labels.
- `evidence-table` with semantic headers and body rows.
- `sources` with multi-column flow.
- `closing`.
- `freeform`.

Patterns should accept content and a limited set of meaningful layout options.

They should not expose every internal coordinate.

For example, a card grid accepts card content, preferred column count, emphasis, and density.

The layout engine owns card width, gutters, wrapping, and vertical placement.

### Freeform slide and custom layouts

Some presentations need a unique visual composition.

The `freeform` pattern should support text, rectangle, ellipse, line, image, and table elements with explicit bounds.

Bounds may use presentation points or percentages.

Every element still requires a stable key and uses theme tokens by default.

Caller-defined layouts may combine these primitives with parameters and repeat blocks.

The layout definition language must remain declarative and must not become a general-purpose programming language.

### Rich text

Text should support plain strings and optional runs.

Each run may set bold, italic, color, font family, font size, link, and baseline offset.

Paragraphs should support alignment, line spacing, space before, space after, indentation, and list levels.

The compiler should collapse adjacent runs with identical style and minimize Google requests.

### Images and asset policy

Google Slides inserts images from URLs that Google can fetch.

The first release should support HTTPS image URLs and require alt text in strict mode.

Local paths should fail with an explanation unless the user selects an explicit asset-staging mode.

A later built-in staging flow may upload an image to a dedicated Drive folder, grant temporary link access, wait for Slides to copy the image, and remove that permission immediately.

That flow must require `--allow-temporary-public-assets`, report the permission lifetime, remove access in a finally-style cleanup path, and record cleanup failures prominently.

The default must never publish a local asset or change Drive permissions silently.

### Native request escape hatch

Slides may include optional native requests for features not covered by semantic patterns.

The escape hatch should be scoped to a keyed slide and inserted at a documented compile phase.

The CLI should validate that custom requests target only the current Managed Presentation and do not delete unmanaged objects unless destructive mode is active.

Native requests should appear in the Quality Report so their presence is never hidden.

## Managed Presentation model

### Ownership

A Managed Presentation is a Google presentation whose authoring identity and source version are recorded by `goog-cli`.

Use Drive `appProperties` for presentation-level metadata such as:

```text
googManaged=true
googDeckSchema=1
googSourceHash=<sha256>
googApplyState=complete
```

Use deterministic Google object IDs for slides and page elements.

IDs should include a reserved `goog_` prefix plus a compact hash derived from the stable source key.

The full source key to object ID mapping should be included in the local Apply Report.

If Drive `appProperties` limits prevent storing the full mapping, stable derivation remains authoritative and the report carries the readable keys.

### Adoption modes

The normal create path creates a Managed Presentation.

Updating an existing Managed Presentation requires no special flag.

Updating an unmanaged presentation has two explicit choices:

- `--adopt` adds managed slides and objects while preserving all unmanaged content.
- `--replace-all` declares that the source owns the complete presentation and may delete current content.

Both modes require a dry-run summary before mutation.

Interactive use asks for confirmation.

Noninteractive use requires `--yes`.

### Reconciliation rules

Reconciliation compares stable identity, content, style, geometry, order, and presentation metadata.

The Apply Plan classifies changes as:

- Create slide.
- Move slide.
- Create element.
- Update element content.
- Update element style.
- Update element geometry.
- Delete stale managed element.
- Delete stale managed slide.
- No change.

The compiler should avoid replacing an object when Google supports an in-place update.

Unchanged objects produce no request.

Deletes run after creates and updates have succeeded and the created state is readable.

### First blank slide

Google presentation creation returns a presentation containing an initial slide.

The create workflow should absorb or replace that slide inside reconciliation.

Callers should never need a separate reset script or know that the initial slide exists.

### Concurrent edits

The first release should detect conflicts on managed objects and leave merging to the caller.

Store the last successfully applied source hash and a compact live-state fingerprint.

If a managed object differs from both the previous applied fingerprint and the new desired state, report a concurrent-edit conflict.

Allow `--prefer-source` to overwrite managed conflicts after listing them.

Never overwrite unmanaged objects through this flag.

## Layout and compilation design

### Coordinate system

Use Google presentation points as the internal coordinate system.

Resolve named aspect ratios into exact page dimensions before layout.

Pattern implementations work inside a safe content rectangle derived from the theme's margins, header, and footer.

Freeform percentage bounds convert to points during normalization.

Do not maintain a separate 1280 by 720 to Google scale factor in user code.

### Layout engine responsibilities

The layout engine should:

- Measure available regions.
- Allocate rows and columns from content count and pattern constraints.
- Apply minimum and preferred gaps.
- Wrap text using font metrics and explicit line-break rules.
- Select a documented density variant when content does not fit the preferred variant.
- Refuse to shrink below the configured minimum font size.
- Produce actionable fit errors that identify the slide key, element key, required height, and available height.
- Track intentional overlap groups so quality checks can distinguish layering from collisions.

Layout should be deterministic across machines when the same font metrics are available.

The CLI should ship metrics for its recommended built-in font set and report when a requested font uses fallback metrics.

### Text fit

Google Slides does not expose a direct text-overflow result in the presentation read model.

The CLI should combine three checks:

- Preflight measurement using font metrics, box width, paragraph spacing, and line wrapping.
- Post-apply verification that all expected text is present in the live object model.
- Live thumbnail generation for visual review.

Text-fit warnings should be honest about their heuristic nature.

The command must not claim that a slide has no overflow based only on a successful Batch Update response.

### Request compiler

The compiler translates a Presentation Plan into ordered native requests.

It should own:

- Google color conversion.
- Unit conversion.
- Object ID generation and collision handling.
- Shape, line, table, image, and text request construction.
- Rich-text run coalescing.
- Paragraph-style request coalescing.
- Field-mask calculation.
- Dependency ordering.
- Request-size estimation.
- Chunk construction.

Existing request builders in `src/commands/slides.rs` should move behind this module and be reused by both one-object commands and deck compilation.

The command layer should stop owning request JSON construction.

### Request chunking

Chunking must preserve dependencies.

A request that inserts or styles text cannot be sent before its shape exists.

The compiler should group each new element's create and initialization operations into the same dependency group.

Chunks should honor both a configurable request-count ceiling and a serialized-byte ceiling.

The initial defaults should be based on live E2E evidence rather than guessed limits.

The responsible AI benchmark, with 1,692 native operations, is the stress case for chunk planning.

### Request minimization

The first benchmark generated 300 text boxes and used 1,500 requests for their creation, content, text style, paragraph style, and vertical alignment.

The compiler should merge compatible operations when Google supports it and omit updates that match defaults.

Request minimization is valuable, but correctness and idempotency take priority over a lower raw request count.

The Apply Report should show native request counts by type so future optimization is evidence-based.

## Module design

### External seam

Add a Deck Authoring module under `src/slides/authoring/`.

Its main interface should be conceptually equivalent to:

```rust
pub struct DeckAuthor<P> {
    presentation_port: P,
}

impl<P: PresentationPort> DeckAuthor<P> {
    pub fn check(&self, request: CheckDeckRequest) -> Result<CheckDeckReport, DeckError>;

    pub async fn apply(
        &self,
        request: ApplyDeckRequest,
    ) -> Result<ApplyDeckReport, DeckError>;

    pub async fn inspect(
        &self,
        request: InspectDeckRequest,
    ) -> Result<InspectDeckReport, DeckError>;
}
```

Callers cross one interface and receive results rather than orchestrating side effects themselves.

Parsing, normalization, layout, compilation, diffing, request batching, polling, thumbnails, montage generation, export, and report writing remain inside the module.

### External Google dependency

Google Slides and Drive are true external dependencies.

Define one internal `PresentationPort` at that seam.

Provide a production Google adapter and an in-memory adapter for module tests.

The port needs operations for create, read, batch update, thumbnail fetch, Drive export, and Drive `appProperties` updates.

Do not expose this port through CLI types or Deck Source types.

### Suggested file structure

```text
src/slides/
  mod.rs
  error.rs
  transport.rs
  types.rs
  requests.rs
  authoring/
    mod.rs
    source.rs
    normalize.rs
    theme.rs
    patterns.rs
    layout.rs
    compile.rs
    reconcile.rs
    quality.rs
    artifacts.rs
    journal.rs
```

The exact file split may change while implementing, but the external interface should stay small.

The large `src/commands/slides.rs` file should become command routing and output formatting rather than the home of every Slides behavior.

### Core types

Recommended internal types:

```text
DeckSource
NormalizedDeck
Theme
SlideDefinition
SlidePattern
ElementDefinition
PresentationPlan
PlannedSlide
PlannedElement
LivePresentation
ApplyPlan
ApplyChunk
ApplyJournal
QualityFinding
QualityReport
ApplyDeckReport
```

Use typed request structures with `serde` instead of assembling large request bodies through ad hoc `serde_json::json!` values.

Keep a raw `serde_json::Value` only at the native-request escape hatch and raw response edges.

## Artifact and QA behavior

### Live thumbnails

Add authorized support for the Slides thumbnail endpoint.

The caller supplies only the presentation ID or URL.

The CLI resolves the Account, sends the authenticated request, downloads the image bytes, and never prints or exposes the Token.

Thumbnail retrieval should retry until the expected slide is available or the consistency timeout expires.

### Montage

Build the montage inside the Rust binary.

Use a deterministic grid based on slide count, preserve source order, add a small slide-number label, and avoid resampling more than necessary.

Write PNG by default so inspection does not require another tool.

The montage path belongs in human and JSON reports.

### Export

Extend the Drive adapter with native Google file export.

Supported deck formats:

```text
application/vnd.openxmlformats-officedocument.presentationml.presentation
application/pdf
```

Export should stream to a temporary file, verify the expected file signature and nonzero size, then atomically rename to the requested output path.

The command should not leave a partially written final file after interruption.

### QA directory

When `--qa-dir` is supplied, write:

```text
qa/
  apply-report.json
  quality-report.json
  normalized-source.json
  apply-plan.json
  apply-journal.json
  montage.png
  thumbnails/
    slide-01.png
    slide-02.png
```

The directory must contain no Token, refresh token, OAuth secret, or raw auth-state data.

Presentation IDs may appear in local reports but must never be copied into committed fixtures.

### Quality findings

Every finding should include:

- Severity: error, warning, or information.
- Stable code.
- Slide key and slide number when applicable.
- Element key when applicable.
- Concise explanation.
- Measured values.
- A concrete corrective action.

Initial finding codes should cover:

```text
SOURCE_SCHEMA_INVALID
DUPLICATE_KEY
OBJECT_ID_COLLISION
UNSUPPORTED_FONT
UNSUPPORTED_IMAGE_SOURCE
IMAGE_ALT_TEXT_MISSING
ELEMENT_OUT_OF_BOUNDS
ELEMENT_NEGATIVE_SIZE
UNEXPECTED_OVERLAP
TEXT_LIKELY_OVERFLOW
TEXT_BELOW_MINIMUM_SIZE
LOW_TEXT_CONTRAST
EMPTY_SLIDE
EXPECTED_TEXT_MISSING
UNMANAGED_TARGET
CONCURRENT_MANAGED_EDIT
LIVE_STATE_TIMEOUT
THUMBNAIL_MISSING
EXPORT_INVALID
```

Quality codes are part of the machine interface and should remain stable once released.

## Failure handling

### Preflight before mutation

Schema, token, geometry, key, image policy, layout, and request-size errors must fail before creating or changing a presentation when possible.

For create mode, the CLI may delay creating the Google file until compilation succeeds.

### Multi-batch applies

Google provides atomicity within one Batch Update call only.

The complete multi-batch deck operation has no cross-batch atomicity.

The CLI should not imply transaction semantics it cannot provide.

Instead, it should:

- Write an Apply Journal after each successful chunk.
- Mark Drive `appProperties.googApplyState` as `applying` before the first mutation.
- Make every chunk safe to reconcile after a retry.
- Create and update desired objects before deleting stale objects.
- Verify final state before marking the apply as `complete`.
- Return the presentation URL and journal path on partial failure.
- Resume through normal `deck apply` reconciliation rather than a separate recovery command.

### Retry policy

Retry only errors that are plausibly transient:

- HTTP 429.
- HTTP 500, 502, 503, and 504.
- Network timeouts and connection resets.
- Temporary live-read absence during the configured consistency window.

Do not retry invalid requests, permission failures, missing Scopes, unmanaged-target refusal, or source errors.

Honor `Retry-After` when Google supplies it.

Include attempt count and final status in the Apply Report without logging request bodies that may contain sensitive presentation text.

### Cleanup

If create mode fails after making a new presentation, report the created URL clearly.

Add `--cleanup-on-failure` only after the CLI has a safe Drive trash operation and can prove the resource was created by the same invocation.

Do not implement cleanup by calling raw Drive endpoints from shell code.

Temporary export files and montage work files should always be removed locally.

Temporary asset permissions must always enter the cleanup report, whether cleanup succeeds or fails.

## Output contracts

### Human output

The default successful apply output should resemble:

```text
Applied deck successfully.
Account: tanakorn.k@siametrics.com
Presentation: Responsible AI Framework - Banking Virtual Assistant
Slides: 14 managed, 0 unmanaged
Elements: 396 managed
Changes: 14 slides created, 396 elements created
Quality: 0 errors, 0 warnings
Live previews: qa/thumbnails
Montage: qa/montage.png
PowerPoint: responsible-ai.pptx
Google Slides: https://docs.google.com/presentation/d/PRESENTATION_ID/edit
```

An unchanged reapply should say that no Slides mutation was needed while still allowing verification and export.

### JSON output

The JSON report should include:

```json
{
  "result": "success",
  "account": "tanakorn.k@siametrics.com",
  "presentationId": "PRESENTATION_ID",
  "presentationUrl": "https://docs.google.com/presentation/d/PRESENTATION_ID/edit",
  "sourceHash": "sha256-value",
  "managedSlides": 14,
  "managedElements": 396,
  "changes": {
    "slidesCreated": 14,
    "slidesMoved": 0,
    "slidesDeleted": 0,
    "elementsCreated": 396,
    "elementsUpdated": 0,
    "elementsDeleted": 0,
    "nativeRequests": 1692
  },
  "quality": {
    "errors": 0,
    "warnings": 0,
    "report": "qa/quality-report.json"
  },
  "artifacts": {
    "montage": "qa/montage.png",
    "pptx": "responsible-ai.pptx"
  }
}
```

The exact native request count may improve after request coalescing.

The benchmark value above records the current scripted baseline rather than a required final count.

## Security and privacy

- All Google requests must flow through the existing authenticated client and Scope checks.
- Never read or print the Token outside the auth module.
- Never embed a Token in an image URL, report, error, or temporary file.
- Do not include presentation text in retry logs by default.
- Treat Deck Sources, thumbnails, exports, and QA reports as potentially sensitive local files.
- Create QA directories and exports with user-only permissions where the platform supports it.
- Refuse non-HTTPS remote image URLs.
- Validate redirects for image and export downloads.
- Bound download sizes and image dimensions before decoding.
- Prevent path traversal when deriving artifact names from slide keys.
- Reject duplicate or malicious object IDs before sending requests.
- Never grant public Drive access without the explicit temporary-asset flag.
- Redact live presentation IDs and URLs before any E2E evidence is committed unless the test resource rules explicitly permit a scratch ID.

### Authorization impact

The planned features require the existing full Google Slides and Google Drive Scopes.

`DEFAULT_LOGIN_SCOPES` already requests both `https://www.googleapis.com/auth/presentations` and `https://www.googleapis.com/auth/drive`.

No new OAuth Scope should be necessary for presentation reads and writes, thumbnail requests, Drive export, or Drive `appProperties` updates.

The production adapter must still call `send_with_scopes` for each operation so Tokens issued before the current upfront-scope policy fail with the normal missing-Scope guidance.

Create mode should record the Account that created the presentation through the existing Resource Account Mapping behavior.

Existing presentation mode should preserve Unified Access and disable fallback when `--account` is explicit.

### Dependency policy

The implementation will probably need local capabilities that the current dependency set does not provide: YAML parsing, SHA-256 hashing, image decoding and PNG encoding, montage composition, and font measurement.

Select Rust crates only after checking maintenance activity, license compatibility, transitive dependency size, supported image limits, and behavior on the release platforms.

Keep these dependencies inside the single existing crate unless one capability creates a clear independent publishing or build concern.

Do not add Node, Python, LibreOffice, a browser runtime, or a generated Google client as a required runtime dependency.

Image decoders must support configured pixel and byte limits before allocation to reduce decompression-bomb risk.

Font measurement should use a bounded recommended font set and a documented fallback rather than scanning arbitrary system font directories during every command.

## Testing strategy

### Test through the Deck Authoring interface

The interface is the primary test surface.

New behavior tests should call `check`, `apply`, or `inspect` and assert returned plans, reports, and observable adapter calls.

Do not add a parallel set of tests that knows every private layout or compiler function.

Old command tests that only duplicate behavior covered through the deeper interface should be deleted as their behavior moves.

Keep focused tests for command parsing and human or JSON rendering.

### Pure source and layout tests

Cover:

- YAML and JSON parity.
- Strict unknown-field rejection.
- Version handling.
- Theme resolution and fallback.
- Every built-in pattern at minimum, normal, and high content density.
- Stable key to object ID derivation.
- Collision detection.
- Rich-text normalization.
- Deterministic geometry.
- Text-fit errors and fallback warnings.
- Intentional and accidental overlaps.
- Image policy.
- Native request validation.

Use semantic assertions on normalized plans.

Avoid enormous raw JSON snapshots that hide the reason for a failure.

### Compiler tests

Cover:

- Request dependency ordering.
- Field masks.
- Unit and color conversion.
- Text and paragraph style coalescing.
- Chunk count and byte limits.
- Objects that fit within one chunk.
- Objects that force a chunk boundary.
- The 14-slide responsible AI benchmark.
- Minimal changes for a one-element content update.
- Zero requests for an unchanged live model.

### Reconciliation tests with an in-memory adapter

Cover:

- New presentation creation and initial-slide handling.
- Unchanged reapply.
- Slide insertion, movement, and deletion.
- Element creation, update, movement, and deletion.
- Preservation of unmanaged content.
- Adoption and replace-all safety checks.
- Concurrent managed edits.
- Duplicate object IDs after an ambiguous retry.
- Partial journal resume.
- Stale deletion only after successful create and update phases.

### Google adapter tests

Use Wiremock fixtures for:

- Presentation create and get.
- Batch Update success and Google error payloads.
- Thumbnail response metadata and image download.
- Drive `appProperties` reads and writes.
- Drive export streaming.
- `Retry-After` handling.
- Rate limits, transient server errors, permissions, missing resources, and malformed responses.

### Live E2E sequence

Follow the repository E2E policy and mutate only a presentation created by the same run.

The E2E sequence should:

1. Run `goog slides deck check` on the sanitized benchmark.
2. Create `goog-e2e-slides-authoring-<timestamp>` with `deck apply`.
3. Assert that the returned presentation has 14 slides and the expected managed identities.
4. Fetch all 14 live thumbnails through `deck inspect`.
5. Verify that the montage is a valid PNG and includes 14 ordered cells.
6. Export PowerPoint and PDF, then verify their file signatures and nonzero sizes.
7. Reapply the unchanged source and assert zero Slides mutations.
8. Change one keyed text element in a temporary source copy, apply it, and assert that only the affected managed object changes.
9. Add one unmanaged object with an existing high-level command, reapply, and prove that normal reconciliation preserves it.
10. Exercise a warning or failure case and verify the exit code and Quality Report.

Capture concise redacted evidence with command, expected result, observed result, and pass or fail.

Never commit the live presentation ID, URL, account data, thumbnail, or exported business content.

### Visual review

Visual review should use the live Google montage, because Google is the final renderer.

The reviewer should inspect title hierarchy, clipping, whitespace, alignment, source readability, repeated-pattern consistency, and the first and last slides at full resolution.

Pixel snapshots should not be the only regression gate because Google rendering and fonts may change outside the repository.

Structural plan assertions and live visual evidence should be used together.

## Documentation and domain changes

### Add domain terms to `CONTEXT.md`

The implementation should define these terms before code lands:

**Deck Source**:
The versioned YAML or JSON description of a presentation's narrative, theme, slide patterns, assets, and stable keys.

**Slide Pattern**:
A semantic layout that converts structured slide content into planned elements and geometry.

**Managed Presentation**:
A Google Slides presentation whose managed slides and objects are reconciled from a Deck Source by `goog-cli`.

**Presentation Plan**:
The normalized, fully laid-out desired state compiled from a Deck Source before comparison with Google.

**Apply Plan**:
The ordered semantic difference between a Presentation Plan and the current live presentation.

**Apply Journal**:
The local record of completed request chunks and final verification state for one apply invocation.

**Quality Report**:
The structured findings from source, layout, live-state, thumbnail, and export verification.

**Managed Object**:
A slide or page element whose stable identity is owned by a Deck Source.

The final wording should follow the repo glossary's term and avoid-term format.

### Add an ADR

Add an ADR titled `Slides Adds Declarative Deck Authoring`.

The ADR should record:

- Semantic Deck Source plus freeform escape hatch.
- Google Slides as the rendering source of truth.
- Stable keys and Drive `appProperties` for ownership.
- Idempotent managed reconciliation instead of reset-and-rebuild.
- Built-in live preview and Drive export.
- Continued support for high-level object commands and raw Batch Update.
- Detection rather than automatic merge for concurrent managed edits.

This decision extends the direction of ADR-0011 for high-level Docs commands without copying Docs location semantics into Slides.

### README updates

Add a short end-to-end example near the start of the Slides section.

Keep the one-object commands under an advanced or direct-edit subsection.

Document Deck Source schema versioning, destructive modes, image policy, QA artifacts, exit codes, and JSON output.

Provide one minimal deck and one polished multi-slide example under `examples/slides/`.

## Implementation sequence

The sequence below is ordered so each slice produces an end-user capability and leaves the CLI in a working state.

No slice should rely on an external helper script as its shipped implementation.

### Slice 1: Record the interface and vocabulary

Deliverables:

- ADR for declarative deck authoring.
- Slides authoring terms in `CONTEXT.md`.
- Command help text and source schema examples reviewed before implementation.
- Sanitized responsible AI benchmark source shape, without live identifiers.

Acceptance:

- The proposed interface can express every slide in the benchmark.
- Destructive ownership rules and image policy are explicit.
- No implementation begins with unresolved names for source, plan, ownership, or quality output.

### Slice 2: Add thumbnails and native export

Deliverables:

- Slides thumbnail transport.
- Drive native export transport.
- `deck inspect` with thumbnails, montage, PowerPoint export, PDF export, and JSON report.
- In-binary PNG montage generation.

Acceptance:

- One CLI invocation inspects a live presentation and creates a complete QA bundle.
- No direct Token access, `curl`, LibreOffice, Node, or Python is needed.
- E2E proves thumbnail count and export signatures against a run-created scratch presentation.

This slice removes the most sensitive and repetitive external orchestration immediately.

### Slice 3: Add the Deck Source and local compiler

Deliverables:

- Versioned YAML and JSON schema.
- Theme resolution.
- Stable key and object ID rules.
- Core visual primitives.
- Built-in patterns required by the benchmark.
- Layout and text-fit checks.
- Typed native request compiler.
- `deck check` with human and JSON output.

Acceptance:

- The sanitized benchmark compiles without an external program.
- The plan is deterministic across repeated runs.
- All generated IDs are stable and valid.
- Quality errors identify exact source paths and slide or element keys.

### Slice 4: Create a Managed Presentation

Deliverables:

- `deck apply --title` create path.
- Initial blank slide handling.
- Presentation metadata in Drive `appProperties`.
- Dependency-aware request chunks.
- Apply Journal.
- Retry and consistency polling.
- Automatic post-apply inspection.

Acceptance:

- One CLI command creates the complete 14-slide benchmark and returns the live URL, thumbnails, montage, and requested exports.
- A partial failure reports a resumable state and never hides the created URL.
- The final live read contains every managed slide, element, and expected text value.

### Slice 5: Reconcile existing Managed Presentations

Deliverables:

- Live-to-planned comparison.
- Minimal create, update, move, and delete operations.
- Unchanged no-op behavior.
- Managed-only deletion.
- Conflict detection.
- `--dry-run` semantic diff.
- `--prefer-source` for explicit managed conflict resolution.

Acceptance:

- Unchanged reapply produces zero mutation requests.
- A single text edit changes only the intended object.
- Reordering one slide does not rebuild the deck.
- Unmanaged objects survive normal reconciliation.
- Stale managed objects are deleted only after desired objects are successfully readable.

### Slice 6: Add adoption and complete replacement

Deliverables:

- `--adopt`.
- `--replace-all`.
- Interactive confirmation and noninteractive `--yes`.
- Clear destructive dry-run output.
- Tests for mixed managed and unmanaged content.

Acceptance:

- The CLI refuses an unmanaged target by default.
- Adoption preserves every existing unmanaged object.
- Replace-all lists the exact slides and objects it will remove before mutation.

### Slice 7: Harden quality and asset behavior

Deliverables:

- Contrast checks.
- Minimum font-size policy.
- Alt-text policy.
- Better font metrics and fallback reporting.
- Image download bounds and redirect validation.
- Optional temporary Drive asset staging with explicit consent, if its security review passes.
- Stable quality codes and documented exit behavior.

Acceptance:

- Known-bad fixture decks fail with actionable findings.
- Strict mode blocks missing alt text and unsafe image sources.
- Temporary public permissions are never created without the explicit flag and are always included in cleanup evidence.

### Slice 8: Refactor direct Slides commands onto shared primitives

Deliverables:

- Move request construction out of `src/commands/slides.rs`.
- Reuse typed request builders, color conversion, units, IDs, text ranges, and validation from the Slides module.
- Keep current command syntax and output compatible.
- Delete redundant shallow tests after equivalent interface-level coverage exists.

Acceptance:

- Existing command tests and live smoke tests pass.
- `src/commands/slides.rs` primarily routes commands and formats results.
- Direct commands and deck authoring produce the same native request shapes for shared operations.

### Slice 9: Documentation, examples, and release validation

Deliverables:

- README workflow.
- Full Deck Source reference.
- Minimal and polished example sources.
- Migration guide from raw request scripts.
- E2E evidence for create, no-op reapply, one-object update, unmanaged preservation, preview, and export.
- Installed-binary smoke test from the release artifact.

Acceptance:

- A fresh user can install `goog`, authorize an Account, and create the example deck using only documented CLI commands.
- The release binary completes the benchmark without repository source code or development dependencies.

## Compatibility and migration

### Existing commands

Do not deprecate current Slides commands during this work.

They remain useful for direct edits, debugging, and unsupported features.

The new authoring path should call shared Rust modules directly rather than invoking `goog` subprocesses.

### Raw Batch Update

Keep `goog slides batch-update` as the last-mile coverage path.

Document that Batch Update does not participate in Managed Presentation ownership unless it targets only unmanaged objects or the caller accepts that the next reconciliation may overwrite managed objects.

### Script migration

Provide a guide that maps common script responsibilities to Deck Source or built-in behavior:

```text
Color and typography constants -> theme tokens
Reusable helper functions -> slide patterns or custom layouts
Coordinate conversion -> internal point layout
Object ID helper -> stable keys
Per-slide request files -> request compiler and chunker
Reset script -> managed reconciliation
Token extraction and curl -> authenticated thumbnail adapter
Local PowerPoint renderer -> Drive export from Google Slides
Montage script -> deck inspect artifact generation
Manual object-count polling -> final-state verification
```

The CLI does not need an automatic JavaScript-to-Deck-Source converter.

The migration guide should make a manual conversion predictable and one-time.

## Performance expectations

Performance goals should be measured against live Google behavior and the 14-slide benchmark.

Track:

- Source parse and local compile duration.
- Native request count and serialized bytes.
- Number of Batch Update calls.
- Retry count and wait duration.
- Time until the final state is readable.
- Thumbnail download duration.
- Montage generation duration.
- Export duration and output size.

Do not hide network wait time inside a generic success message.

Human output may show a progress bar by phase unless `--quiet` is set.

JSON output should include phase timings.

Local checking should remain fast enough to use before every apply, while live verification time will depend on Google.

## Observability and supportability

Use phase-oriented diagnostics:

```text
parse
normalize
layout
compile
read-live
diff
apply
poll
inspect
export
```

Errors should name the phase, slide key, element key, request chunk, Google status, and whether rerunning is safe.

`--verbose` may print request type counts and retry decisions, but must not print Tokens or complete sensitive text bodies.

The Apply Journal should make support possible without reproducing the whole deck.

Add a `reportVersion` to JSON reports so tooling can consume them safely.

## Risks and mitigations

### Google rendering differs from local text metrics

Mitigation: Treat font measurement as preflight guidance, verify expected live text, always fetch Google thumbnails, and describe overflow results as likely rather than exact.

### Large decks require multiple Batch Update calls

Mitigation: Compile dependency-aware chunks, journal progress, make reruns idempotent, and delete stale content last.

### Human edits conflict with managed source

Mitigation: Preserve unmanaged objects, fingerprint managed state, detect concurrent managed edits, and require an explicit preference before overwrite.

### Object ID restrictions or collisions

Mitigation: Use a reserved prefix, sanitized compact hashes, deterministic collision detection, and preflight validation.

### Image insertion requires fetchable URLs

Mitigation: Support HTTPS sources first, reject silent local publishing, and gate any temporary Drive permission flow behind explicit informed consent and cleanup evidence.

### QA output leaks sensitive content

Mitigation: Use user-only file permissions, never include Tokens, keep verbose text extraction optional, document artifact sensitivity, and redact E2E evidence.

### Quality rules become too rigid for creative slides

Mitigation: Allow per-element intentional-overlap declarations, per-slide quality overrides with reasons, warning severity controls, and a freeform pattern without bypassing bounds or identity checks.

### The authoring schema becomes another programming language

Mitigation: Keep patterns semantic, keep layout definitions declarative, reject executable expressions, and add new built-in patterns when repeated source boilerplate appears.

### The command module continues growing

Mitigation: Put behavior behind the Deck Authoring interface, share request primitives through `src/slides`, and keep `src/commands/slides.rs` limited to argument mapping and output.

## Definition of done

The overall plan is complete when:

- The responsible AI benchmark is authored and maintained without custom executable scripts.
- `deck check`, `deck apply`, and `deck inspect` are documented and available in a released binary.
- One apply command can create or update the deck, verify the live Google result, generate thumbnails and a montage, and export PowerPoint and PDF.
- Unchanged reapply is a verified no-op.
- Managed identity survives content and layout changes.
- Unmanaged content is preserved by default.
- Interrupted applies are safe to rerun.
- Quality findings are structured, actionable, and stable for automation.
- All Google calls use the existing Account and Unified Access behavior.
- Live E2E evidence covers create, verify, export, no-op, targeted update, and unmanaged preservation.
- The installed release binary passes the same workflow without Node, Python, LibreOffice, `jq`, `curl`, or repository-only helpers.
- Existing direct Slides commands and raw Batch Update remain compatible.

## Recommended first milestone

Start with Slice 2 and Slice 3 after the vocabulary and ADR are accepted.

Thumbnail, montage, and export support immediately removes the unsafe token-and-HTTP workarounds from future sessions.

The local Deck Source compiler then captures the reusable design and layout behavior before reconciliation adds live mutation complexity.

The first public milestone should let this command succeed against a new scratch presentation:

```sh
goog slides deck apply \
  --source examples/slides/responsible-ai-framework.yaml \
  --title "goog-e2e-slides-authoring" \
  --qa-dir ./qa \
  --export-pptx ./responsible-ai-framework.pptx
```

At that point, future presentation sessions can focus on content and visual judgment while `goog-cli` owns the repetitive authoring machinery.
