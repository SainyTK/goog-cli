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
4. Create or inspect the document.
5. Prefer high-level `goog docs` commands over `batch-update`.
6. Use dry runs for supported edits, then apply the confirmed operation.
7. Map the document again after structural edits because indexes and entry numbers can change.
8. Open the finished native document at 100% zoom and inspect every page from top to bottom.
9. Correct content, hierarchy, tables, spacing, page breaks, and formatting before delivery.
10. Return the Google Docs URL and a short description of the finished result.

Read [references/quality.md](references/quality.md) before creating a new document or substantially rewriting one.
Read [references/commands.md](references/commands.md) when selecting commands or location selectors.

## Command selection

Use these surfaces first:

- `goog docs create` for a blank native document.
- `goog docs get` for raw structure and revision metadata.
- `goog docs map` for readable content locations, headings, tables, and images.
- `goog docs text` for search, insertion, and replacement.
- `goog docs style apply` for text and paragraph styling.
- `goog docs table` for inserting and populating tables.
- `goog docs image`, `break`, `footnote`, `header`, `footer`, `list-format`, and `named-range` for their named document features.

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
- If no matching authenticated browser session is available, report visual QA as blocked instead of claiming the completion gate passed.
- Never expose scratch files, request bodies, or internal QA notes in the document.

## Completion gate

Do not call the document finished until all of these are true:

- The requested content is complete and factually consistent.
- Heading levels form a sensible hierarchy.
- Tables are used for real row-and-column comparisons, not as generic page layout.
- Lists use native list formatting.
- No placeholder, instruction text, or duplicated content remains.
- Styles are consistent and readable.
- The document has been visually inspected at 100% zoom, including every page edge and any requested page-count limit.
- The live document has been fetched again after the last write.
- The final URL opens the intended native Google Doc.
