# Docs Writes Suggestions Through the Suggest Write Mode

`goog docs text insert` and `goog docs text replace` take `--suggest`, which sets `writeControl.writeMode` to `SUGGEST` on the Batch Update so the edit becomes a Suggestion for someone with edit access to accept or reject.
`goog docs suggestions` lists a Document's suggestion threads, and `suggestion-accept`, `suggestion-reject`, and `suggestion-delete` send the Docs API `acceptSuggestion`, `rejectSuggestion`, and `deleteSuggestion` requests.
Other kinds of suggested edits go through `docs batch-update` with the same write mode until a high-level command needs them.

## Evidence

The Docs API discovery document (revision 20261118) has `WriteControl.writeMode` with `EDIT` and `SUGGEST`, the three suggestion requests, `Document.suggestions` as `SuggestionThread`s, and `suggestionResponses` and `commentUpdateState` in the Batch Update response.
Google's Docs API guide on comments and suggestions documents the write mode and the permissions.
On 2026-11-25 each option was tried through the API on throwaway Documents owned by a personal Gmail account, one of them shared with a second Gmail account as commenter.
The Documents were then checked in the Docs editor in a browser:

| Tried | API result | What the Docs editor showed |
| --- | --- | --- |
| `insertText` and `deleteContentRange` with `writeMode: SUGGEST` | `createdSuggestionIds` for each request | Inserted text in color, deleted text struck through, an "Add" and a "Delete" card in the margin |
| `replaceAllText` with `SUGGEST` | One suggestion | "Replace: old with new" card, new text in color before the struck-out old text |
| `deleteContentRange` then `insertText` at the same start index, with `SUGGEST` | One created and one updated suggestion ID | One "Replace" card, the same as `replaceAllText` |
| `updateTextStyle`, `updateParagraphStyle`, `createParagraphBullets`, `insertPageBreak`, `insertTable`, `insertInlineImage` with `SUGGEST` | One suggestion each | "Format: bold", "Format: heading 2", "Format list", "Add page break", "Add table" cards; the image framed as an insertion |
| `insertComment` in a `SUGGEST` Batch Update, alone or after suggested text | An ordinary comment thread | Comment card anchored to the text, including text suggested in the same Batch Update |
| Suggestion typed by hand in Suggesting mode | Listed by `documents.get` with `commentsViewMode=COMMENTS_VIEW_MODE_INCLUDED` and `suggestionsViewMode=SUGGESTIONS_INLINE` | "Add" card |
| `acceptSuggestion` and `rejectSuggestion` | `acceptedSuggestionIds` or `rejectedSuggestionIds` | Accepted text became plain text; rejected text was restored; the cards left the margin |
| `deleteSuggestion` | `deletedSuggestionIds` | Change and card both gone |
| A commenter's `SUGGEST` write | Suggestion created | Card under the commenter's name, text in a second author color |
| A commenter's direct write or `acceptSuggestion` | 403 | Nothing changed |

Further results that shape the commands:

- Accepted and rejected threads stay in `Document.suggestions` with status `ACCEPTED` or `REJECTED`; deleted ones disappear, so `docs suggestions --open` filters on status.
- Accepting an already accepted suggestion returns 200 again, while an unknown ID returns 404, which the Docs client reports as a missing Document; the suggestion commands read the threads first and name the unknown ID before writing.
- Only the author can delete a suggestion, even the Document owner gets 403 on someone else's; rejecting works for editors and for the author.
- A change next to an existing suggestion by the same author merges into it, and the response lists it under `updatedSummarySuggestionIds`, as the Docs editor merges typing.
- Suggested insertions and deletions keep their place in the indexes until they are resolved, so the Document Map and index selectors see them, and later requests in one Batch Update must count them.
- Google documents that a Batch Update can commit its document changes while failing to save its comments or suggestions, reported only by `commentUpdateState: ALL_FAILED_UNKNOWN_REASON`.
- `AddDocumentTab`, `CreateNamedRange`, `DeleteFooter`, `DeleteHeader`, `DeleteNamedRange`, `DeleteTab`, `UpdateDocumentTabProperties`, and `UpdateTableColumnProperties` are rejected in `SUGGEST` mode, which rules out `--suggest` on the named range, header, and footer commands.

## Consequences

Every Batch Update now fails with an error when `commentUpdateState` is `ALL_FAILED_UNKNOWN_REASON`, because a suggested edit that silently lands as a direct edit defeats the reason for suggesting it.
The error says the changes may have been applied, since the API gives no way to tell.

The Revision Guard chaining between split Batch Updates now changes only `requiredRevisionId` and keeps the rest of `writeControl`, so the write mode survives.

`text replace --suggest` keeps the same requests as a direct replacement, which the Docs editor shows as one "Replace" suggestion with the new text before the struck-out old text, the same order `replaceAllText` produces.

An account shared a Document for comment can now edit it through `--suggest`, which matches what that account can do in the Docs editor.
