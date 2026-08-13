# Mail Starts With Message Read JSON and Attachment Download

The first Gmail surface exposes message listing, message search, raw message reads, and attachment downloads over Gmail messages, not Threads.
`read` returns the raw Gmail API Message JSON first because Message payloads can contain nested MIME parts, headers, text and HTML bodies, and attachments; a human-friendly renderer can be added later without hiding the native Gmail structure from scripts.
Attachment download is included because raw Message JSON can expose attachment IDs without giving users a way to retrieve the payload.
Downloads fail when the destination file already exists, matching the CLI's safe default for local file writes.
Mail commands request Gmail scope only when needed.
Mail list and search stay bounded with `--limit`; they do not expose `--all` because each listed Message requires an additional metadata fetch.
