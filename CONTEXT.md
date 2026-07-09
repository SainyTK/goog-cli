# goog CLI

A Rust CLI for managing multiple Google accounts and interacting with Google APIs (Drive, Docs, Sheets, Slides, Gmail).
Designed first for power users and AI agents who want a scriptable, terminal-native alternative to the browser.
Human-readable terminal workflows are the default.
JSON output is supported for programmatic integration, but it is not the primary product surface.

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
The pair of (access token, refresh token) issued by Google for a specific Account and set of Scopes.
Stored in the Token Store, never in setup config.
_Avoid_: Credentials (overloaded), auth token, OAuth token

**Token Store**:
The local secret-bearing auth state file that holds authorized Accounts, the Active Account, Tokens, and Resource Account Mappings.
The default Token Store is `~/.goog/auth.json`, a user-owned file with restricted permissions so terminal and agent workflows can read Tokens without operating-system password prompts.
_Avoid_: Keychain, credentials file, setup config

**Scope**:
A Google OAuth permission string (e.g., `https://www.googleapis.com/auth/drive`) that grants access to a specific API. `goog auth login` requests every Scope the CLI supports upfront; a command still checks the Token has the Scope it needs before calling the API, as a safety net for Tokens issued before this was true.
_Avoid_: Permission, capability

**Target Access Failure**:
A definitive Google API response showing that an Account cannot access a target resource after it has the required Scope.
It does not include malformed targets, network failures, rate limits, revoked Tokens, missing OAuth setup, or scope-check failures.
_Avoid_: Resolve failure, command failure, auth failure

**Resource Account Mapping**:
A remembered association between a target resource on a Google API surface and the Account that last accessed it successfully.
It is an optimization for future commands, not an ownership claim.
_Avoid_: Resource owner, account cache, file owner

**Account Fallback**:
The automatic attempt to access a target resource with other Accounts after the default Account receives a Target Access Failure.
Explicit Account selection disables Account Fallback.
_Avoid_: Account switching, account guessing, auto-login

**Unified Access**:
The user experience where a person logs in to multiple Accounts once, then targets Google resources without manually switching Accounts for each command.
Unified Access applies to both read and write commands that target existing resources.
_Avoid_: Multi-account mode, account pooling, global access

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

**Draft**:
An unsent GoogleMail Message stored through the Gmail drafts API.
`goog mail draft create` creates a Draft but does not send it.
_Avoid_: Outbox item, scheduled email, sent message

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

### Distribution

**Early Open-Source CLI**:
The public maturity position for `goog`: useful and installable for real Google API workflows, with command coverage and interfaces still expected to expand.
_Avoid_: Stable product, prototype, demo

**Canonical Release**:
A tagged GitHub Release created from `main` that owns the downloadable `goog` binaries and checksums for a published version.
_Avoid_: Branch head, installer version

**Preview Release**:
A GitHub pre-release created from the `preview` branch for opt-in validation before Stable LTS promotion.
Preview releases use tags such as `v0.2.4-preview.1`.
_Avoid_: Beta, unstable, branch-head binary

**Release Asset**:
A downloadable binary archive or checksum file attached to a Canonical Release for one supported platform.
_Avoid_: Build artifact, package

**Distribution Channel**:
A user-facing installation path that resolves to a GitHub Release.
The only supported distribution channel is the GitHub-hosted installer script.
Stable resolves to Canonical Releases by default.
Preview resolves to preview pre-releases only when explicitly requested.
_Avoid_: Release source, package source

**Release Automation**:
The GitHub Actions workflow that turns a stable version tag on `main` or preview version tag on `preview` into Release Assets and distribution metadata.
_Avoid_: Publish script, deploy script

**Installer Script**:
The GitHub-hosted shell entrypoint that detects the user's platform, downloads a stable or preview Release Asset, verifies it, and installs the `goog` binary.
_Avoid_: Bash release, install command
