# Docs Creates Anchored Comments Through the Docs API

`goog docs comment-create` creates Anchored Comments with the Docs API `insertComment` Batch Update request, which takes a Document Range.
Docs editors highlight these comments in the text and show them in the margin, like comments added in the editor.
Listing, editing, replying to, resolving, and deleting a Document's Comments stay with Drive, so `goog docs comments` and the other `docs comment-*` commands call the same Drive code as the `drive comment-*` commands.
This narrows the earlier rule that Drive owns all Comment behavior: Docs now owns the creation of Anchored Comments.

## Evidence

The Docs API discovery document added comment support in revision 20261118: `insertComment`, `addCommentReply`, `updateCommentPost`, `deleteComment`, and `deleteCommentReply` requests, and `CommentThread` plus `DocumentTab.commentAnchors` in `documents.get` when `commentsViewMode` is `COMMENTS_VIEW_MODE_INCLUDED`.
On 2026-11-25 each option was tried through the APIs on throwaway Documents owned by a Google Workspace account and by a personal Gmail account.
The Gmail account's Document was then checked in the Docs editor in a browser:

| Comment created with | What the Docs editor showed |
| --- | --- |
| Docs `insertComment` with a range | Text highlighted, card in the margin, panel header names the tab and the quoted text |
| Drive `comments.create` with an `anchor` JSON range (`{"r":"head","a":[{"txt":{"o":N,"l":N}}]}`) | No highlight or margin card; panel lists it under "Original content deleted" with no quote |
| Drive `comments.create` with `quotedFileContent` only | No highlight or margin card; "Original content deleted", with the quote behind an expander |
| Drive `comments.create` with both | Same as `quotedFileContent` only |
| Drive `comments.create` with neither | No highlight or margin card; "Original content deleted" |
| Drive `comments.create` with `anchor` set to a `kix.` anchor ID that `insertComment` created | Margin card shared with that anchor; no quote of its own |

The Drive API stores any anchor string it is given but the Docs editor ignores it, which matches Google's Drive comments guide.
The `kix.` anchor IDs that make a Drive comment attach are only created by the Docs editor or by `insertComment`, so Drive cannot anchor a comment on its own.

Further results that shape the command:

- `insertComment` rejects a request without a range, so `docs comment-create` requires an anchor selector; `drive comment-create` remains the way to add an unanchored comment.
- A zero-length range is widened to the word at that index, so deletions are marked with `--deleted` on a comment anchored to neighbouring text that remains.
- Content longer than 2048 UTF-8 bytes is rejected, so the command checks the limit before writing.
- `@email` mentions at the start of the content become mentions, as they do through Drive.
- Comments created by `insertComment` use the same IDs as Drive comments; Drive lists them with the `kix.` anchor and the quoted text, and Drive replies, resolutions, and edits leave the anchor in place.
- Replacing or deleting all of the anchored text later detaches the comment, which then appears under "Original content deleted".

## Consequences

Agents that mark their edits should comment after the final edit to each passage.
The Docs comment requests have no equivalent for Sheets or Slides, whose `comment-create` commands still create unanchored comments through Drive.
