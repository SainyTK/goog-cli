# Mail Starts With Message Read JSON

The first GoogleMail surface exposes `goog mail list`, `goog mail search`, and `goog mail read` over Gmail Messages, not Threads. `read` returns the raw Gmail API Message JSON first because Message payloads can contain nested MIME parts, headers, text and HTML bodies, and attachments; a human-friendly renderer can be added later without hiding the native Gmail structure from scripts.
