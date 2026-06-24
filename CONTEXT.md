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
