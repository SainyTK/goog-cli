# goog CLI

A Rust CLI for managing multiple Google accounts and interacting with Google APIs (Drive, Docs, Sheets, Slides, Gmail). Designed for developers and power users who want a scriptable, terminal-native alternative to the browser.

## Language

### Auth

**OAuth App**:
The single GCP project and OAuth 2.0 client (client ID + client secret) that `goog` uses to authorize all accounts. Configured once via `goog auth setup`.
_Avoid_: GCP project, OAuth client, credentials (overloaded)

**Account**:
A Google user identity that has been authorized through `goog auth login`. One OAuth App can have many Accounts. Identified by email address.
_Avoid_: User, profile, identity

**Active Account**:
The Account that commands target by default when no `--account` flag is provided. Stored in config. Switched explicitly via `goog auth switch`.
_Avoid_: Current account, default account, selected account

**Account Selector**:
A command argument that identifies an Account for account-management commands. It may be a full email address or a case-insensitive partial email match when the command explicitly supports partial matching; when multiple Accounts match, the first Account in list order is selected.
_Avoid_: Account query, account search, email prefix

**Token**:
The pair of (access token, refresh token) issued by Google for a specific Account and set of Scopes. Stored in the system keychain, never in config files.
_Avoid_: Credentials (overloaded), auth token, OAuth token

**Keychain Access Prompt**:
An operating-system prompt that appears when `goog` reads or writes a Token from the system keychain.
It is distinct from Google consent and does not grant new Scopes.
_Avoid_: Google password prompt, OAuth prompt, browser login

**Trusted CLI**:
The local `goog` executable that the OS user has approved to read and write Tokens without a Keychain Access Prompt on every command invocation.
Trust is local to the machine and OS user account.
_Avoid_: Trusted app, trusted account, trusted Google session

**Scope**:
A Google OAuth permission string (e.g., `https://www.googleapis.com/auth/drive`) that grants access to a specific API. Scopes are acquired incrementally -- only when a command first needs them.
_Avoid_: Permission, capability

**Incremental Authorization**:
The pattern of requesting only the Scopes a command needs, on first use, rather than all Scopes upfront at login.
_Avoid_: Lazy auth, on-demand auth, progressive scopes

### Commands

**Setup**:
The one-time command (`goog auth setup`) that records the OAuth App's client ID and client secret in config. It may collect those values directly or import them from a `client_secret_*.json` file.
_Avoid_: Init, configure, bootstrap

**Login**:
The command (`goog auth login`) that authorizes a new Account via a browser-based OAuth flow, issuing a Token for that Account.
_Avoid_: Authenticate, connect, authorize

### Drive

**Folder**:
A Google Drive resource that can contain files and other Folders.
_Avoid_: Directory, collection

### Docs

**Document**:
A Google Docs resource whose editable body, tabs, and structural elements are managed through the Docs API. A Document is stored as a Drive file, but Drive owns discovery and file lifecycle while Docs owns document content.
_Avoid_: File, doc

**Batch Update**:
The Google Docs write operation that applies an ordered list of document mutation requests to a Document. High-level editing commands may be added later, but the first write surface exposes Batch Update directly.
_Avoid_: Patch, edit, update

**Document Location**:
A user-facing reference to a position in a Document. It always includes a stable Google Docs index and may also include derived labels such as page and content line.
_Avoid_: Cursor, coordinate

**Document Range**:
A user-facing reference to a span of Document content. It may be an explicit start and end index, a whole content block selected by a Location Selector, or text spans selected by search.
_Avoid_: Selection, highlight

**Document Map**:
A navigable summary of a Document's editable content blocks and objects, with each entry carrying a Document Location and a short preview. It is the shared read model behind high-level Docs discovery commands.
_Avoid_: Outline, dump, raw document

**Document Map Entry**:
One numbered row in a Document Map. Entry numbers are for human navigation within a displayed map and are distinct from Google Docs indexes.
_Avoid_: Index, row ID

**Content Line**:
A numbered top-level content block within a derived page, such as a paragraph, heading, table, or image-bearing paragraph. It is not a rendered visual line after text wrapping.
_Avoid_: Visual line, wrapped line

**Location Confidence**:
The evidence level behind a derived Document Location, such as an explicit page break or table-of-contents heading. It tells users whether a page or content-line label is structurally exact, inferred, or unavailable.
_Avoid_: Accuracy score, certainty

**Revision Guard**:
A write precondition that requires the Document revision to match a caller-provided revision ID before applying a high-level edit or raw Batch Update.
_Avoid_: Version check, optimistic lock

**Location Selector**:
A command argument or group of arguments that resolves to one Document Location, such as an index, a page plus content line, or a heading/text anchor. High-level Docs write commands use Location Selectors instead of requiring raw Batch Update request bodies.
_Avoid_: Query, cursor selector, coordinate selector

**Ambiguous Location**:
A Location Selector result that matches more than one Document Location. High-level Docs write commands must reject Ambiguous Locations and return the candidate locations for disambiguation.
_Avoid_: Best match, fuzzy target

**Dry Run Preview**:
A local simulation of a high-level Docs write command showing the resolved Document Location, the native Batch Update request, and the affected content before and after the edit. It does not promise Google-rendered page layout.
_Avoid_: Rendered preview, final layout preview

**Human Preview**:
The default Dry Run Preview format for people, focused on before/after content around the resolved Document Location. Machine-readable Dry Run Preview details are emitted with `--json`.
_Avoid_: Pretty JSON, rendered preview

**High-Level Docs Command**:
A Docs command that performs a common read or edit operation through Document Maps, Location Selectors, ambiguity checks, and Dry Run Previews. High-Level Docs Commands coexist with Batch Update rather than replacing it.
_Avoid_: Convenience wrapper, shortcut command

**Table Handle**:
A user-facing identifier assigned to a table entry in a Document Map, such as `table-3`. It is assigned by current document order and re-resolved from the latest Document Map on each command.
_Avoid_: Table ID, raw index

**Image Handle**:
A user-facing identifier assigned to an image entry in a Document Map, such as `image-7`. It may refer to an inline image with a Document Location or a positioned image with an object ID and layout metadata.
_Avoid_: Image ID, raw object ID

**Inline Image**:
An image that lives in the Document text flow and has an index-bearing paragraph element. High-level image insertion creates Inline Images.
_Avoid_: Embedded image

**Positioned Image**:
An image represented as a positioned object with layout metadata rather than a normal text-flow insertion point. Positioned Images are listed by High-Level Docs Commands but are not the first target of high-level image edits.
_Avoid_: Floating image

**Table Data**:
Rectangular CSV or TSV content used by high-level table commands. Full-table edits replace cell text from Table Data and require matching dimensions unless resizing is explicitly requested.
_Avoid_: Range payload, table range

**Style Payload**:
Style settings accepted by high-level style commands, either as named shorthand flags or as raw Google Docs style JSON for full API coverage.
_Avoid_: Formatting blob, style patch

**List Style**:
A high-level list type such as bullet, numbered, dash, or checkbox that maps to a Google Docs bullet preset. Raw Google presets remain available for less common list formats.
_Avoid_: Bullet preset, glyph type

### Mail

**GoogleMail**:
The Gmail-backed Google API surface exposed through the `goog mail` command namespace.
_Avoid_: Gmail command, Google Mail command, email client

**Message**:
A single email item in GoogleMail, identified by a Gmail message ID and retrieved through the Gmail API.
_Avoid_: Email, mail item, thread item

**Message Summary**:
A lightweight GoogleMail view of a Message for list and search output, containing the message ID and selected metadata headers without the full body.
_Avoid_: Message preview, email row, search result

**Thread**:
A Gmail conversation that groups related Messages. Threads are not part of the first GoogleMail command slice.
_Avoid_: Conversation, chain

**Mailbox Query**:
A Gmail search expression used to find Messages through GoogleMail.
_Avoid_: Search term, filter, query string

**Inbox**:
The Gmail label used by `goog mail list` to show current inbox Messages by default.
_Avoid_: Default mailbox, mail list, received mail

**Attachment**:
A file-like payload part associated with a Message and retrieved separately from the Message body.
_Avoid_: File, mail file, payload

### Sheets

**Spreadsheet**:
A Google Sheets file-level resource containing Spreadsheet properties, Sheets, grid metadata, cell data when explicitly requested, named ranges, and other workbook-level structure.
_Avoid_: Workbook, sheet file

**Sheet**:
An individual tab within a Spreadsheet. A Sheet has its own properties, grid dimensions, and optional grid data.
_Avoid_: Tab, worksheet

**Range**:
A Google Sheets A1 notation selector that identifies cells within a Spreadsheet, optionally scoped to a Sheet name such as `Sheet1!A1:B2`.
_Avoid_: Cell selector, address

**ValueRange**:
The native Google Sheets values API JSON shape for reading or writing cell values in one Range, including `range`, `majorDimension`, and `values`.
_Avoid_: Rows payload, cells payload

**Structural Batch Update**:
The Google Sheets `spreadsheets.batchUpdate` operation that applies ordered structural mutation requests to a Spreadsheet, such as adding Sheets, formatting cells, resizing dimensions, filters, merges, and protected ranges.
_Avoid_: Values batch update, patch, edit

### File Transfer

**Resumable Upload**:
Google's chunked upload protocol used for files over 5 MB. Allows upload to survive interruptions. Distinct from a simple multipart upload used for small files.
_Avoid_: Chunked upload, multi-part upload (different thing)
