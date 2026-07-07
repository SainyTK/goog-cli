# goog

![goog CLI wallpaper](./public/goog-cli-wallpaper.png)

`goog` is an Early Open-Source CLI for Google APIs.
It is built first for power users and AI agents who want terminal-native access to Google Drive, Google Docs, Google Sheets, and GoogleMail without getting forced down a browser UI path.

Human-readable terminal workflows are the default experience.
JSON is also supported for programmatic use, but it is not the primary product surface.

The CLI uses one OAuth App for all accounts, stores Accounts, the Active Account, Tokens, and Resource Account Mappings in `~/.goog/auth.json`, and requests API scopes incrementally when commands need them.

## Current API Coverage

`goog` currently includes:

- Google Drive file and folder listing, upload, and download commands.
- Google Docs document creation, mapping, text search, content lookup, high-level text/image/table/style/list edits, page and section breaks, headers, footers, footnotes, named ranges, raw document reads, and raw batch updates.
- Google Sheets spreadsheet reads, values reads and writes, appends, clears, and structural batch updates.
- GoogleMail message listing, search, raw message reads, draft creation, and attachment downloads.
- Multi-account OAuth setup, login, account listing, and active account switching.

## Installation

### Installer Script

The installer script is the default install path for macOS and Linux. It installs the latest Canonical Release from GitHub Releases rather than branch-head code.
It installs to `/usr/local/bin` when that directory is writable.
If `/usr/local/bin` is not writable, it installs to `$HOME/.local/bin` and prints a PATH warning if needed.

```sh
curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh
```

Install a specific Canonical Release with:

```sh
curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh -s -- --version v0.2.3
```

The installer supports macOS arm64, macOS x64, Linux x64, and Linux arm64 Release Assets.

### Rust-Native Fallback

Users outside the binary release support matrix can install from source with Cargo:

```sh
cargo install --git https://github.com/SainyTK/goog-cli goog
```

This path requires a local Rust toolchain.

### Uninstall

If you installed with the installer script, remove the binary from the supported install locations:

```sh
rm -f /usr/local/bin/goog "$HOME/.local/bin/goog"
```

If you installed with Cargo, uninstall the Cargo package:

```sh
cargo uninstall goog
```

Those commands remove the executable only.
To fully reset local `goog` state, delete `$HOME/.goog`.
That directory contains OAuth App setup in `config.toml` and auth state in `auth.json`.
The auth state file grants account access within authorized scopes, so do not commit it or sync it into places you do not trust.

## OAuth Setup

Create a Google OAuth client for a desktop or web application, then configure `goog` once:

```sh
goog auth setup
```

### Google Cloud Console Setup Guide

Open <https://console.cloud.google.com>, create a project or select an existing one, then complete the setup below before running `goog auth setup`.

#### 1. Enable the required APIs

Go to **APIs & Services -> Library**.

![API Library](./public/readme/01-api-library.png)

Enable these APIs:

- Google Drive API
- Google Docs API
- Google Sheets API
- Gmail API

If an API already shows **Manage** with an **API Enabled** badge, skip it.

![API already enabled](./public/readme/02-apis-enabled.png)

#### 2. Configure the OAuth consent screen

If this is a brand-new project, go to **APIs & Services -> OAuth consent screen**.

In the newer GCP layout, this may appear under **Google Auth Platform -> Branding**.

If the consent screen is already configured, skip to the next step.

Choose **External** for personal Google accounts.

Fill in the required fields:

- **App name**: any descriptive name, such as `My Office Agent`
- **User support email**: your email address
- **Developer contact information**: your email address

All other fields can stay blank.

![App information form](./public/readme/03-app-information.png)

Click **Save and Continue**.

Because the app stays in **Testing** mode by default, add your own Google account under **Test users** before finishing the flow.

![Add test users](./public/readme/04-add-test-users.png)

#### 3. Create OAuth credentials

Go to **APIs & Services -> Credentials**.

Click **+ Create credentials**.

![Credentials page](./public/readme/05-credentials-page.png)

Select **OAuth client ID**.

![Create credentials dropdown](./public/readme/06-create-credential-menu.png)

#### 4. Choose Desktop app

On the client creation form, set **Application type** to **Desktop app**.

![Application type dropdown](./public/readme/07-app-type-dropdown.png)

Enter any descriptive name, then click **Create**.

![Name filled in](./public/readme/08-name-filled.png)

#### 5. Copy the client ID and client secret

After the client is created, copy both values from the dialog:

- **Client ID**: a long value ending in `.apps.googleusercontent.com`
- **Client secret**: a shorter value usually starting with `GOCSPX-`

![OAuth client created](./public/readme/09-oauth-client-created.png)

Copy both values before closing the dialog.

You can also use **Download JSON** if you prefer the file-based `--client-secret-file` setup path.

You can also import a downloaded Google client secret file:

```sh
goog auth setup --client-secret-file client_secret_123.apps.googleusercontent.com.json
```

Authorize a Google Account:

```sh
goog auth login
```

List authorized Accounts:

```sh
goog auth list
goog auth list --json
```

Switch the Active Account:

```sh
goog auth switch alice@example.com
```

Run one command against a different Account without switching:

```sh
goog --account bob@example.com drive ls
```

## Examples

### Drive

```sh
goog drive ls --limit 20
goog drive folder list --parent FOLDER_ID --json
goog drive upload ./report.pdf --folder FOLDER_ID
goog drive download FILE_ID --output ./report.pdf
```

### Docs

```sh
goog docs create "Q3 Report"
goog docs map DOCUMENT_ID
goog docs search-text DOCUMENT_ID "quarterly plan"
goog docs get-content DOCUMENT_ID --heading "Summary"
goog docs insert-page-break DOCUMENT_ID --after-heading "Summary"
goog docs insert-section-break DOCUMENT_ID --section-type next-page --after-heading "Appendix"
goog docs create-header DOCUMENT_ID
goog docs create-footer DOCUMENT_ID
goog docs create-footnote DOCUMENT_ID --after-text "quarterly plan"
goog docs create-named-range DOCUMENT_ID "highlights" --text "quarterly plan"
goog docs delete-named-range DOCUMENT_ID --name "highlights"
goog docs batch-update DOCUMENT_ID --requests ./requests.json
```

### Sheets

```sh
goog sheets create "Quarterly Plan"
goog sheets get SPREADSHEET_ID --fields 'properties.title,sheets.properties'
goog sheets sheet add SPREADSHEET_ID "Raw Data"
goog sheets sheet rename SPREADSHEET_ID 123456789 "Archive"
goog sheets sheet move SPREADSHEET_ID 123456789 0
goog sheets sheet duplicate SPREADSHEET_ID 123456789 "Archive Copy"
goog sheets sheet freeze SPREADSHEET_ID 123456789 --rows 1 --columns 2
goog sheets sheet resize SPREADSHEET_ID 123456789 --rows 200 --columns 12
goog sheets sheet auto-resize SPREADSHEET_ID 123456789 --dimension columns --start-index 0 --end-index 5
goog sheets sheet set-dimension-size SPREADSHEET_ID 123456789 --dimension rows --start-index 1 --end-index 3 --pixel-size 28
goog sheets sheet hide-dimension SPREADSHEET_ID 123456789 --dimension columns --start-index 1 --end-index 3
goog sheets sheet unhide-dimension SPREADSHEET_ID 123456789 --dimension columns --start-index 1 --end-index 3
goog sheets sheet group-dimension SPREADSHEET_ID 123456789 --dimension rows --start-index 1 --end-index 10
goog sheets sheet ungroup-dimension SPREADSHEET_ID 123456789 --dimension rows --start-index 1 --end-index 10
goog sheets sheet collapse-dimension-group SPREADSHEET_ID 123456789 --dimension rows --start-index 1 --end-index 10
goog sheets sheet expand-dimension-group SPREADSHEET_ID 123456789 --dimension rows --start-index 1 --end-index 10
goog sheets sheet insert-dimension SPREADSHEET_ID 123456789 --dimension rows --start-index 2 --end-index 4 --inherit-from-before
goog sheets sheet delete-dimension SPREADSHEET_ID 123456789 --dimension columns --start-index 3 --end-index 6
goog sheets sheet basic-filter SPREADSHEET_ID 123456789 --start-row 0 --end-row 100 --start-column 0 --end-column 5
goog sheets sheet clear-basic-filter SPREADSHEET_ID 123456789
goog sheets sheet merge SPREADSHEET_ID 123456789 --start-row 0 --end-row 2 --start-column 0 --end-column 4 --merge-type all
goog sheets sheet unmerge SPREADSHEET_ID 123456789 --start-row 0 --end-row 2 --start-column 0 --end-column 4
goog sheets sheet sort-range SPREADSHEET_ID 123456789 --start-row 1 --end-row 100 --start-column 0 --end-column 5 --sort-column 3 --order descending
goog sheets sheet delete-duplicates SPREADSHEET_ID 123456789 --start-row 1 --end-row 100 --start-column 0 --end-column 5 --comparison-column 1 --comparison-column 3
goog sheets sheet trim-whitespace SPREADSHEET_ID 123456789 --start-row 1 --end-row 100 --start-column 0 --end-column 5
goog sheets sheet find-replace SPREADSHEET_ID "draft" "final" --sheet-id 123456789 --match-case
goog sheets sheet copy-paste SPREADSHEET_ID 123456789 --source-start-row 1 --source-end-row 4 --source-start-column 0 --source-end-column 3 --destination-sheet-id 987654321 --destination-start-row 10 --destination-end-row 13 --destination-start-column 0 --destination-end-column 3 --paste-type values
goog sheets sheet cut-paste SPREADSHEET_ID 123456789 --source-start-row 1 --source-end-row 4 --source-start-column 0 --source-end-column 3 --destination-sheet-id 987654321 --destination-row 10 --destination-column 0 --paste-type values
goog sheets sheet background-color SPREADSHEET_ID 123456789 --start-row 0 --end-row 1 --start-column 0 --end-column 5 "#ffcc00"
goog sheets sheet text-color SPREADSHEET_ID 123456789 --start-row 0 --end-row 1 --start-column 0 --end-column 5 "#3366cc"
goog sheets sheet font-size SPREADSHEET_ID 123456789 --start-row 0 --end-row 1 --start-column 0 --end-column 5 --size 14
goog sheets sheet font-family SPREADSHEET_ID 123456789 --start-row 0 --end-row 1 --start-column 0 --end-column 5 --family Roboto
goog sheets sheet number-format SPREADSHEET_ID 123456789 --start-row 1 --end-row 100 --start-column 3 --end-column 4 --type currency --pattern '$#,##0.00'
goog sheets sheet borders SPREADSHEET_ID 123456789 --start-row 0 --end-row 10 --start-column 0 --end-column 5 --edge outer --style solid-thick --color "#3366cc"
goog sheets sheet clear-format SPREADSHEET_ID 123456789 --start-row 0 --end-row 10 --start-column 0 --end-column 5
goog sheets sheet bold SPREADSHEET_ID 123456789 --start-row 0 --end-row 1 --start-column 0 --end-column 5
goog sheets sheet italic SPREADSHEET_ID 123456789 --start-row 0 --end-row 1 --start-column 0 --end-column 5
goog sheets sheet underline SPREADSHEET_ID 123456789 --start-row 0 --end-row 1 --start-column 0 --end-column 5
goog sheets sheet strikethrough SPREADSHEET_ID 123456789 --start-row 0 --end-row 1 --start-column 0 --end-column 5
goog sheets sheet horizontal-align SPREADSHEET_ID 123456789 --start-row 0 --end-row 1 --start-column 0 --end-column 5 --alignment center
goog sheets sheet vertical-align SPREADSHEET_ID 123456789 --start-row 0 --end-row 1 --start-column 0 --end-column 5 --alignment middle
goog sheets sheet text-wrap SPREADSHEET_ID 123456789 --start-row 0 --end-row 10 --start-column 0 --end-column 5 --strategy wrap
goog sheets sheet text-rotation SPREADSHEET_ID 123456789 --start-row 0 --end-row 1 --start-column 0 --end-column 5 --angle 45
goog sheets sheet text-direction SPREADSHEET_ID 123456789 --start-row 0 --end-row 10 --start-column 0 --end-column 5 --direction right-to-left
goog sheets sheet note SPREADSHEET_ID 123456789 --start-row 1 --end-row 2 --start-column 3 --end-column 4 "Check source data"
goog sheets sheet note SPREADSHEET_ID 123456789 --start-row 1 --end-row 2 --start-column 3 --end-column 4 --clear
goog sheets sheet data-validation-list SPREADSHEET_ID 123456789 --start-row 1 --end-row 100 --start-column 2 --end-column 3 --value Open --value Closed --input-message "Pick a status"
goog sheets sheet data-validation-list SPREADSHEET_ID 123456789 --start-row 1 --end-row 100 --start-column 2 --end-column 3 --clear
goog sheets sheet data-validation-checkbox SPREADSHEET_ID 123456789 --start-row 1 --end-row 100 --start-column 0 --end-column 1 --input-message "Mark done"
goog sheets sheet data-validation-checkbox SPREADSHEET_ID 123456789 --start-row 1 --end-row 100 --start-column 0 --end-column 1 --checked-value Done --unchecked-value Todo
goog sheets sheet data-validation-checkbox SPREADSHEET_ID 123456789 --start-row 1 --end-row 100 --start-column 0 --end-column 1 --clear
goog sheets sheet conditional-format-color SPREADSHEET_ID 123456789 --start-row 1 --end-row 100 --start-column 3 --end-column 4 --condition number-greater --value 100 --background-color "#ffcccc"
goog sheets sheet conditional-format-update SPREADSHEET_ID 123456789 0 --start-row 1 --end-row 100 --start-column 3 --end-column 4 --condition text-contains --value Blocked --background-color "#ffeeee"
goog sheets sheet conditional-format-delete SPREADSHEET_ID 123456789 0
goog sheets sheet conditional-format-move SPREADSHEET_ID 123456789 2 0
goog sheets sheet protect-range SPREADSHEET_ID 123456789 --start-row 0 --end-row 1 --start-column 0 --end-column 5 --description "Lock headers"
goog sheets sheet protect-range SPREADSHEET_ID 123456789 --start-row 1 --end-row 100 --start-column 0 --end-column 5 --warning-only
goog sheets sheet add-named-range SPREADSHEET_ID 123456789 HeaderCells --start-row 0 --end-row 1 --start-column 0 --end-column 5
goog sheets sheet delete-named-range SPREADSHEET_ID header_cells
goog sheets sheet update-named-range SPREADSHEET_ID header_cells --name HeaderRows --sheet-id 123456789 --start-row 0 --end-row 2 --start-column 0 --end-column 5
goog sheets sheet update-protected-range SPREADSHEET_ID 7 --description "Warn before editing" --warning-only
goog sheets sheet update-protected-range SPREADSHEET_ID 7 --enforce
goog sheets sheet unprotect-range SPREADSHEET_ID 7
goog sheets sheet tab-color SPREADSHEET_ID 123456789 "#3366cc"
goog sheets sheet clear-tab-color SPREADSHEET_ID 123456789
goog sheets sheet hide SPREADSHEET_ID 123456789
goog sheets sheet unhide SPREADSHEET_ID 123456789
goog sheets sheet delete SPREADSHEET_ID 123456789
goog sheets values get SPREADSHEET_ID 'Sheet1!A1:D10'
goog sheets values update SPREADSHEET_ID 'Sheet1!A1' --values ./value-range.json
goog sheets values update-row SPREADSHEET_ID 'Sheet1!A2:C2' --value Ada --value Lovelace --value '=SUM(C2:C10)'
goog sheets values update-table SPREADSHEET_ID 'Sheet1!A1:D10' --data ./rows.csv
goog sheets values append SPREADSHEET_ID 'Sheet1!A:D' --values ./rows.json
goog sheets values append-row SPREADSHEET_ID 'Sheet1!A:D' --value Ada --value Lovelace --value '=SUM(C2:C10)'
goog sheets values append-table SPREADSHEET_ID 'Sheet1!A:D' --data ./rows.csv
```

### GoogleMail

```sh
goog mail list --limit 10
goog mail search 'from:alerts@example.com newer_than:7d'
goog mail read MESSAGE_ID
goog mail draft create --to teammate@example.com --subject 'Status update' --body-file ./message.txt --attachment ./report.pdf
goog mail attachment download MESSAGE_ID ATTACHMENT_ID --output invoice.pdf
```

Use `goog help`, `goog <command> --help`, and nested command help for the full command reference.

### Limitations

- **`goog` cannot write Office files (.xlsx, .docx) in Drive.** Writing to an Excel-format spreadsheet (`values update`, `values batch-update`, `values append`, `batch-update`) or a Word-format document (`batch-update`) is not supported. This is a Google Sheets/Docs API restriction, not a `goog` gap: neither API can write to `.xlsx` or `.docx` files at all. Convert the file to a native Google Sheet or Google Doc first (Drive UI: File > Save as Google Sheets/Docs) to edit it with `goog`.

## Contributor Workflow

Install local dependencies:

```sh
cargo fetch
npm install
```

Run checks before opening a pull request:

```sh
cargo fmt --check
cargo test
npm run typecheck
```

Issues live in GitHub Issues. Pick work labeled `ready-for-agent` or `bug` (there is no separate `Sandcastle` label), keep changes scoped to the issue, and link the issue from the pull request.

Pull requests should include:

- A concise summary of user-facing behavior.
- Tests or verification notes covering the changed behavior.
- Documentation updates when commands, OAuth behavior, or distribution paths change.
- Release-document updates when Canonical Release assets, installer behavior, Homebrew tap behavior, or operator steps change.

Distribution changes must keep GitHub Releases as the only Canonical Release authority.
