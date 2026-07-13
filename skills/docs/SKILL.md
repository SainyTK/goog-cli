---
name: docs
description: Create, read, edit, structure, and visually verify native Google Docs with the goog CLI. Use for professional reports, memos, proposals, guides, document templates, and any request involving Google Docs content or formatting.
---

# Google Docs

Create and edit native Google Docs with `goog docs`.
Use `target/debug/goog` from the goog-cli repository, or `cargo run --`, because the development binary is the authoritative command surface.

## Required workflow

1. Decide the document's purpose, audience, content structure, and visual hierarchy before writing.
2. Run `target/debug/goog auth list` and record the active account before any live mutation.
3. Run `target/debug/goog docs --help` and the relevant nested `--help` command before the first mutation.
4. Choose blank creation for a new design or template copying when editor-only components must be preserved.
5. Create or inspect the document.
6. Prefer high-level `goog docs` commands over `batch-update`.
7. Use dry runs for supported edits, then apply the confirmed operation.
8. Map the document again after structural edits because indexes and entry numbers can change.
9. Export the finished document as PDF and inspect every page from top to bottom at 100% zoom, or use an authenticated browser when export is unavailable.
10. Correct content, hierarchy, tables, spacing, page breaks, and formatting before delivery.
11. Repeat the available visual inspection path after the last layout-affecting edit.
12. Return the Google Docs URL and a short description of the finished result.

Read [references/quality.md](references/quality.md) before creating a new document or substantially rewriting one.
Read [references/commands.md](references/commands.md) when selecting commands or location selectors.

## Command selection

Use these surfaces first:

- `goog docs create` for a blank native document.
- `goog docs copy` when a source document contains native tables of contents, page-number auto text, positioned images, first-page headers, or other components that the Docs API cannot create.
- `goog docs get` for raw structure and revision metadata.
- `goog docs map` for readable content locations, headings, tables, and images.
- `goog docs text` for search, insertion, and replacement.
- `goog docs style apply` for text and paragraph styling.
- `goog docs style named` for defining a blank document's native style system.
- `goog docs style copy-named` and `copy-page` for transferring an existing document's visual system to a blank target.
- `goog docs table` for inserting and populating tables.
- `goog docs image`, `break`, `footnote`, `header`, `footer`, `list-format`, and `named-range` for their named document features.
- `goog docs export-pdf` for page-level visual QA.

Use `goog docs batch-update` only when no high-level command supports the required operation.
Keep any temporary JSON request body in task-local scratch space and remove it after verification.

## Editing safety

- Accept a document ID or full Google Docs URL wherever the command supports either.
- Use the active account shown by `goog auth list` unless the user specifies another authorized account.
- Pass `--account EMAIL` when account selection must remain explicit across a multi-step task.
- If authorization opens an account chooser, select only the recorded active or user-specified account.
- Never infer an account from browser order, a remembered identity, or unrelated open documents.
- If a command fails because the account is missing required scopes, run `goog auth login` once and retry; do not expect the original command to pause and resume on its own.
- Use `goog auth list` for the auth preflight and do not list unrelated Drive resources.
- Inspect with `docs map` before targeting content.
- Prefer semantic selectors such as `heading:`, `after-heading:`, `before-text:`, or `--text` over raw indexes.
- Run `--dry-run --json` when the command supports it.
- Use `--required-revision-id` for multi-step or potentially concurrent edits.
- Capture revision IDs directly from command output and never retype or shorten them manually.
- Refresh the Document Map after any failed guarded write before retrying.
- Re-fetch the revision and remap after each structural change.
- Preserve the existing design during local edits unless the user asks for a redesign.
- Treat browser inspection as read-only QA and avoid editor keyboard shortcuts that could mutate content.
- Use an already authenticated native browser session for the recorded account when one is available.
- If no matching authenticated browser session is available, export the document as PDF and inspect the rendered pages instead.
- Treat a denied PDF export as a Workspace or file-policy limitation and retry with `--account EMAIL` only when account selection may be the cause.
- If both browser inspection and PDF export are unavailable, report visual QA as blocked instead of claiming the completion gate passed.
- Never expose scratch files, request bodies, or internal QA notes in the document.

## Completion gate

Do not call the document finished until all of these are true:

- The requested content is complete and factually consistent.
- Heading levels form a sensible hierarchy.
- Tables are used for real row-and-column comparisons, not as generic page layout.
- Lists use native list formatting.
- No placeholder, instruction text, or duplicated content remains.
- Styles are consistent and readable.
- The native document or its exported PDF has been visually inspected at 100% zoom, including every page edge and any requested page-count limit.
- When reproducing a source document, source and target component inventories have been compared and every intentional difference is understood.
- When reproducing a source document, named styles and page styles have been compared after generated IDs are removed.
- When reproducing a source document, mapped content and component properties have been compared after generated IDs are removed.
- The live document has been fetched again after the last write.
- The final URL opens the intended native Google Doc.
