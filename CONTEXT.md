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

### File Transfer

**Resumable Upload**:
Google's chunked upload protocol used for files over 5 MB. Allows upload to survive interruptions. Distinct from a simple multipart upload used for small files.
_Avoid_: Chunked upload, multi-part upload (different thing)
