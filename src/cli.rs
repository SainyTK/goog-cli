use clap::{Parser, Subcommand, ValueEnum};

use crate::auth::config::OAuthAppType;

#[derive(Debug, Parser)]
#[command(
    name = "goog",
    about = "A terminal-native Google APIs CLI for power users and AI agents",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Override the active account for this invocation
    #[arg(long, global = true)]
    pub account: Option<String>,

    /// Suppress progress bars and informational output
    #[arg(long, global = true)]
    pub quiet: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage Google account authentication
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// Interact with Google Drive
    Drive {
        #[command(subcommand)]
        command: DriveCommand,
    },
    /// Interact with Google Docs
    Docs {
        #[command(subcommand)]
        command: DocsCommand,
    },
    /// Interact with GoogleMail
    Mail {
        #[command(subcommand)]
        command: MailCommand,
    },
    /// Interact with Google Sheets
    Sheets {
        #[command(subcommand)]
        command: SheetsCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Set up the OAuth App client ID and secret
    Setup {
        /// Import OAuth App values from a client_secret_*.json file
        #[arg(long)]
        client_secret_file: Option<String>,
        /// OAuth App type to store when the JSON shape is not specific enough
        #[arg(long, value_enum)]
        app_type: Option<OAuthAppType>,
    },
    /// Authorize a Google Account via a browser-based OAuth flow
    Login {
        /// Use device authorization grant instead of browser redirect
        #[arg(long)]
        no_browser: bool,
    },
    /// List all authorized accounts
    List {
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Switch the active account
    Switch {
        /// Email address or partial email of the account to activate
        email: String,
    },
    /// Export full auth state to a file for use with GOOG_TOKEN_FILE in
    /// headless environments such as Sandcastle. The output file grants
    /// access to every account it contains, within their authorized scopes --
    /// never commit it, and delete it once the headless environment no longer
    /// needs it.
    Export {
        /// Email address or partial email of one account to export. Omit to
        /// export every authorized account.
        email: Option<String>,
        /// Path to write the auth state JSON to. Overwrites any existing file.
        #[arg(long)]
        out: String,
    },
    /// Manage remembered Resource Account Mappings
    Mappings {
        #[command(subcommand)]
        command: AuthMappingsCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthMappingsCommand {
    /// List remembered Resource Account Mappings and their Account
    List {
        /// Emit newline-delimited JSON
        #[arg(long)]
        json: bool,
    },
    /// Clear Resource Account Mappings from runtime state
    Clear {
        /// Google API surface to clear, such as docs. Use with --resource-id.
        #[arg(long)]
        surface: Option<String>,
        /// Resource ID to clear within the Google API surface. Use with --surface.
        #[arg(long)]
        resource_id: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum DriveCommand {
    /// Browse files and folders in Google Drive
    Ls {
        /// Maximum number of items to return (default: 50)
        #[arg(long)]
        limit: Option<u32>,
        /// Fetch all items across all pages
        #[arg(long)]
        all: bool,
        /// Drive folder ID to browse
        #[arg(long)]
        folder: Option<String>,
        /// Emit newline-delimited JSON
        #[arg(long)]
        json: bool,
    },
    /// List files in Google Drive
    List {
        /// Maximum number of files to return (default: 50)
        #[arg(long)]
        limit: Option<u32>,
        /// Fetch all files across all pages
        #[arg(long)]
        all: bool,
        /// Drive folder ID to list files from
        #[arg(long)]
        folder: Option<String>,
        /// Emit newline-delimited JSON
        #[arg(long)]
        json: bool,
    },
    /// Manage Google Drive folders
    Folder {
        #[command(subcommand)]
        command: DriveFolderCommand,
    },
    /// Download a file from Google Drive
    Download {
        /// Drive file ID to download
        file_id: String,
        /// Destination path (defaults to current directory)
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Upload a local file to Google Drive
    Upload {
        /// Local file path to upload
        path: String,
        /// Drive folder ID to upload into
        #[arg(long)]
        folder: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum DriveFolderCommand {
    /// List folders in Google Drive
    List {
        /// Maximum number of folders to return (default: 50)
        #[arg(long)]
        limit: Option<u32>,
        /// Fetch all folders across all pages
        #[arg(long)]
        all: bool,
        /// Drive parent folder ID to list folders from
        #[arg(long)]
        parent: Option<String>,
        /// Emit newline-delimited JSON
        #[arg(long)]
        json: bool,
    },
}

impl DocsCommand {
    /// Resolves the `document_id` field of any variant to a bare Document ID,
    /// extracting it first if a Google Docs/Drive URL was passed instead.
    pub fn normalize_document_id(&mut self) {
        let document_id = match self {
            DocsCommand::Create { .. } => return,
            DocsCommand::Map { document_id, .. }
            | DocsCommand::SearchText { document_id, .. }
            | DocsCommand::GetContent { document_id, .. }
            | DocsCommand::InsertText { document_id, .. }
            | DocsCommand::ReplaceText { document_id, .. }
            | DocsCommand::ListImages { document_id, .. }
            | DocsCommand::ListTables { document_id, .. }
            | DocsCommand::InsertImage { document_id, .. }
            | DocsCommand::InsertPageBreak { document_id, .. }
            | DocsCommand::InsertSectionBreak { document_id, .. }
            | DocsCommand::CreateHeader { document_id, .. }
            | DocsCommand::CreateFooter { document_id, .. }
            | DocsCommand::CreateFootnote { document_id, .. }
            | DocsCommand::InsertTable { document_id, .. }
            | DocsCommand::EditTable { document_id, .. }
            | DocsCommand::ApplyStyles { document_id, .. }
            | DocsCommand::ApplyList { document_id, .. }
            | DocsCommand::CreateNamedRange { document_id, .. }
            | DocsCommand::DeleteNamedRange { document_id, .. }
            | DocsCommand::Get { document_id, .. }
            | DocsCommand::BatchUpdate { document_id, .. }
            | DocsCommand::ShowStyleTemplate { document_id, .. } => document_id,
        };
        *document_id = crate::docs::extract_document_id(document_id);
    }
}

#[derive(Debug, Subcommand)]
pub enum DocsCommand {
    /// Create a new, blank Google Docs Document
    #[command(after_long_help = "Output shape:
  Prints the created Document ID and its Google Docs edit URL, tab-separated.

Notes:
  The Document is always created at the root of My Drive; there is no --folder option today.
  Move it afterward with the Google Drive web UI, or via a future `goog drive` move command.
  Follow up with `goog docs batch-update` or the other `goog docs` editing commands to add content.")]
    Create {
        /// Title for the new Google Docs Document
        title: String,
    },
    /// Print a high-level map of editable Google Docs content
    Map {
        /// Google Docs Document ID or URL to map
        document_id: String,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
    },
    /// Search editable Google Docs content through the Document Map
    SearchText {
        /// Google Docs Document ID or URL to search
        document_id: String,
        /// Text to find
        text: String,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
    },
    /// Retrieve one content block through a Document Map location selector
    GetContent {
        /// Google Docs Document ID or URL to inspect
        document_id: String,
        /// Raw Google Docs UTF-16 index
        #[arg(long)]
        index: Option<i64>,
        /// Document Map Entry number
        #[arg(long)]
        entry: Option<usize>,
        /// Derived page label
        #[arg(long)]
        page: Option<usize>,
        /// Content line within the derived page
        #[arg(long)]
        line: Option<usize>,
        /// Heading text anchor
        #[arg(long)]
        heading: Option<String>,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
    },
    /// Insert text through a high-level Document Map location selector
    InsertText {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// Text to insert
        text: String,
        /// Raw Google Docs UTF-16 index
        #[arg(long)]
        index: Option<i64>,
        /// Document Map Entry number
        #[arg(long)]
        entry: Option<usize>,
        /// Derived page label
        #[arg(long)]
        page: Option<usize>,
        /// Content line within the derived page
        #[arg(long)]
        line: Option<usize>,
        /// Insert after the matching heading text
        #[arg(long)]
        after_heading: Option<String>,
        /// Insert before the matching heading text
        #[arg(long)]
        before_heading: Option<String>,
        /// Insert after the matching text span
        #[arg(long)]
        after_text: Option<String>,
        /// Insert before the matching text span
        #[arg(long)]
        before_text: Option<String>,
        /// Preview the edit without calling documents.batchUpdate
        #[arg(long)]
        dry_run: bool,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
        /// Require the document to still be at this revision before applying the edit
        #[arg(long)]
        required_revision_id: Option<String>,
    },
    /// Replace text through a high-level Document Map text match
    ReplaceText {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// Existing text to replace
        old_text: String,
        /// Replacement text
        new_text: String,
        /// Replace the Nth text match
        #[arg(long = "match")]
        match_number: Option<usize>,
        /// Replace every text match
        #[arg(long)]
        all: bool,
        /// Preview the edit without calling documents.batchUpdate
        #[arg(long)]
        dry_run: bool,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
        /// Require the document to still be at this revision before applying the edit
        #[arg(long)]
        required_revision_id: Option<String>,
    },
    /// List image-like objects through the Document Map
    ListImages {
        /// Google Docs Document ID or URL to inspect
        document_id: String,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
    },
    /// List tables through the Document Map
    ListTables {
        /// Google Docs Document ID or URL to inspect
        document_id: String,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
    },
    /// Insert an Inline Image through a high-level Document Map location selector
    InsertImage {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// Publicly reachable image URI for Google Docs insertInlineImage
        image_uri: String,
        /// Raw Google Docs UTF-16 index
        #[arg(long)]
        index: Option<i64>,
        /// Document Map Entry number
        #[arg(long)]
        entry: Option<usize>,
        /// Derived page label
        #[arg(long)]
        page: Option<usize>,
        /// Content line within the derived page
        #[arg(long)]
        line: Option<usize>,
        /// Insert after the matching heading text
        #[arg(long)]
        after_heading: Option<String>,
        /// Insert before the matching heading text
        #[arg(long)]
        before_heading: Option<String>,
        /// Insert after the matching text span
        #[arg(long)]
        after_text: Option<String>,
        /// Insert before the matching text span
        #[arg(long)]
        before_text: Option<String>,
        /// Preview the edit without calling documents.batchUpdate
        #[arg(long)]
        dry_run: bool,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
        /// Require the document to still be at this revision before applying the edit
        #[arg(long)]
        required_revision_id: Option<String>,
    },
    /// Insert a page break through a high-level Document Map location selector
    InsertPageBreak {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// Raw Google Docs UTF-16 index
        #[arg(long)]
        index: Option<i64>,
        /// Document Map Entry number
        #[arg(long)]
        entry: Option<usize>,
        /// Derived page label
        #[arg(long)]
        page: Option<usize>,
        /// Content line within the derived page
        #[arg(long)]
        line: Option<usize>,
        /// Insert after the matching heading text
        #[arg(long)]
        after_heading: Option<String>,
        /// Insert before the matching heading text
        #[arg(long)]
        before_heading: Option<String>,
        /// Insert after the matching text span
        #[arg(long)]
        after_text: Option<String>,
        /// Insert before the matching text span
        #[arg(long)]
        before_text: Option<String>,
        /// Preview the edit without calling documents.batchUpdate
        #[arg(long)]
        dry_run: bool,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
        /// Require the document to still be at this revision before applying the edit
        #[arg(long)]
        required_revision_id: Option<String>,
    },
    /// Insert a section break through a high-level Document Map location selector
    InsertSectionBreak {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// Section break type
        #[arg(long, value_enum, default_value_t = DocsSectionBreakType::NextPage)]
        section_type: DocsSectionBreakType,
        /// Raw Google Docs UTF-16 index
        #[arg(long)]
        index: Option<i64>,
        /// Document Map Entry number
        #[arg(long)]
        entry: Option<usize>,
        /// Derived page label
        #[arg(long)]
        page: Option<usize>,
        /// Content line within the derived page
        #[arg(long)]
        line: Option<usize>,
        /// Insert after the matching heading text
        #[arg(long)]
        after_heading: Option<String>,
        /// Insert before the matching heading text
        #[arg(long)]
        before_heading: Option<String>,
        /// Insert after the matching text span
        #[arg(long)]
        after_text: Option<String>,
        /// Insert before the matching text span
        #[arg(long)]
        before_text: Option<String>,
        /// Preview the edit without calling documents.batchUpdate
        #[arg(long)]
        dry_run: bool,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
        /// Require the document to still be at this revision before applying the edit
        #[arg(long)]
        required_revision_id: Option<String>,
    },
    /// Create the document's default header, returning its headerId
    #[command(after_long_help = "Output shape:
  Prints the raw documents.batchUpdate response JSON, which includes the new headerId under replies[0].createHeader.headerId.

Notes:
  Always creates the DEFAULT header for the document's first section; there is no per-section header support today.
  Edit the header's own content with `goog docs insert-text`/`goog docs batch-update`, targeting a location inside the returned headerId segment.")]
    CreateHeader {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// Preview the edit without calling documents.batchUpdate
        #[arg(long)]
        dry_run: bool,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
        /// Require the document to still be at this revision before applying the edit
        #[arg(long)]
        required_revision_id: Option<String>,
    },
    /// Create the document's default footer, returning its footerId
    #[command(after_long_help = "Output shape:
  Prints the raw documents.batchUpdate response JSON, which includes the new footerId under replies[0].createFooter.footerId.

Notes:
  Always creates the DEFAULT footer for the document's first section; there is no per-section footer support today.
  Edit the footer's own content with `goog docs insert-text`/`goog docs batch-update`, targeting a location inside the returned footerId segment.")]
    CreateFooter {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// Preview the edit without calling documents.batchUpdate
        #[arg(long)]
        dry_run: bool,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
        /// Require the document to still be at this revision before applying the edit
        #[arg(long)]
        required_revision_id: Option<String>,
    },
    /// Create a footnote at a high-level Document Map location, returning its footnoteId
    #[command(after_long_help = "Output shape:
  Prints the raw documents.batchUpdate response JSON, which includes the new footnoteId under replies[0].createFootnote.footnoteId.

Notes:
  The footnote reference is inserted at the resolved location; the footnote's own body starts empty.
  Edit the footnote's own content with `goog docs insert-text`/`goog docs batch-update`, targeting a location inside the returned footnoteId segment.")]
    CreateFootnote {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// Raw Google Docs UTF-16 index
        #[arg(long)]
        index: Option<i64>,
        /// Document Map Entry number
        #[arg(long)]
        entry: Option<usize>,
        /// Derived page label
        #[arg(long)]
        page: Option<usize>,
        /// Content line within the derived page
        #[arg(long)]
        line: Option<usize>,
        /// Insert after the matching heading text
        #[arg(long)]
        after_heading: Option<String>,
        /// Insert before the matching heading text
        #[arg(long)]
        before_heading: Option<String>,
        /// Insert after the matching text span
        #[arg(long)]
        after_text: Option<String>,
        /// Insert before the matching text span
        #[arg(long)]
        before_text: Option<String>,
        /// Preview the edit without calling documents.batchUpdate
        #[arg(long)]
        dry_run: bool,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
        /// Require the document to still be at this revision before applying the edit
        #[arg(long)]
        required_revision_id: Option<String>,
    },
    /// Insert a table through a high-level Document Map location selector
    InsertTable {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// CSV or TSV data file to populate the inserted table
        #[arg(long)]
        data: Option<String>,
        /// Number of table rows
        #[arg(long)]
        rows: Option<usize>,
        /// Number of table columns
        #[arg(long)]
        columns: Option<usize>,
        /// Raw Google Docs UTF-16 index
        #[arg(long)]
        index: Option<i64>,
        /// Document Map Entry number
        #[arg(long)]
        entry: Option<usize>,
        /// Derived page label
        #[arg(long)]
        page: Option<usize>,
        /// Content line within the derived page
        #[arg(long)]
        line: Option<usize>,
        /// Insert after the matching heading text
        #[arg(long)]
        after_heading: Option<String>,
        /// Insert before the matching heading text
        #[arg(long)]
        before_heading: Option<String>,
        /// Insert after the matching text span
        #[arg(long)]
        after_text: Option<String>,
        /// Insert before the matching text span
        #[arg(long)]
        before_text: Option<String>,
        /// Preview the edit without calling documents.batchUpdate
        #[arg(long)]
        dry_run: bool,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
        /// Require the document to still be at this revision before applying the edit
        #[arg(long)]
        required_revision_id: Option<String>,
        /// Skip applying the cached style template's header styling to the new table
        #[arg(long)]
        no_auto_style: bool,
    },
    /// Replace table cell text from CSV or TSV data
    EditTable {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// Table handle from list-tables, such as table-3
        #[arg(long)]
        table_id: String,
        /// CSV or TSV data file with replacement cell text
        #[arg(long)]
        data: String,
        /// Allow future structural table resizing
        #[arg(long)]
        resize: bool,
        /// Preview the edit without calling documents.batchUpdate
        #[arg(long)]
        dry_run: bool,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
        /// Require the document to still be at this revision before applying the edit
        #[arg(long)]
        required_revision_id: Option<String>,
    },
    /// Apply common text styles through a high-level Document Range
    ApplyStyles {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// Raw Google Docs UTF-16 range start
        #[arg(long)]
        from_index: Option<i64>,
        /// Raw Google Docs UTF-16 range end
        #[arg(long)]
        to_index: Option<i64>,
        /// Document Map Entry number
        #[arg(long)]
        entry: Option<usize>,
        /// Derived page label
        #[arg(long)]
        page: Option<usize>,
        /// Content line within the derived page
        #[arg(long)]
        line: Option<usize>,
        /// Matched text span to style
        #[arg(long)]
        text: Option<String>,
        /// Style the Nth text match
        #[arg(long = "match")]
        match_number: Option<usize>,
        /// Apply bold text style
        #[arg(long)]
        bold: bool,
        /// Apply italic text style
        #[arg(long)]
        italic: bool,
        /// Font size in points
        #[arg(long)]
        font_size: Option<f64>,
        /// Foreground color as #RRGGBB
        #[arg(long)]
        foreground_color: Option<String>,
        /// Named paragraph style such as HEADING_1
        #[arg(long)]
        heading: Option<String>,
        /// Raw Google Docs style JSON with optional textStyle and paragraphStyle objects
        #[arg(long)]
        style_json: Option<String>,
        /// Preview the edit without calling documents.batchUpdate
        #[arg(long)]
        dry_run: bool,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
        /// Require the document to still be at this revision before applying the edit
        #[arg(long)]
        required_revision_id: Option<String>,
        /// Ignore the cached style template for this document
        #[arg(long)]
        no_auto_style: bool,
    },
    /// Apply a common list preset through a high-level Document Range
    ApplyList {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// Raw Google Docs UTF-16 range start
        #[arg(long)]
        from_index: Option<i64>,
        /// Raw Google Docs UTF-16 range end
        #[arg(long)]
        to_index: Option<i64>,
        /// Document Map Entry number
        #[arg(long)]
        entry: Option<usize>,
        /// Derived page label
        #[arg(long)]
        page: Option<usize>,
        /// Content line within the derived page
        #[arg(long)]
        line: Option<usize>,
        /// List type shorthand
        #[arg(long = "type", value_enum)]
        list_type: Option<DocsListType>,
        /// Raw Google Docs bullet preset
        #[arg(long)]
        preset: Option<String>,
        /// Preview the edit without calling documents.batchUpdate
        #[arg(long)]
        dry_run: bool,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
        /// Require the document to still be at this revision before applying the edit
        #[arg(long)]
        required_revision_id: Option<String>,
        /// Ignore the cached style template for this document
        #[arg(long)]
        no_auto_style: bool,
    },
    /// Create a named range over a high-level Document Range, returning its namedRangeId
    #[command(after_long_help = "Output shape:
  Prints the raw documents.batchUpdate response JSON, which includes the new namedRangeId under replies[0].createNamedRange.namedRangeId.

Notes:
  Named ranges do not track edits after creation; a later insert/delete before the range can leave it pointing at the wrong text.
  Re-create the named range after content in or before it changes.")]
    CreateNamedRange {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// Name for the new named range (need not be unique)
        name: String,
        /// Raw Google Docs UTF-16 range start
        #[arg(long)]
        from_index: Option<i64>,
        /// Raw Google Docs UTF-16 range end
        #[arg(long)]
        to_index: Option<i64>,
        /// Document Map Entry number
        #[arg(long)]
        entry: Option<usize>,
        /// Derived page label
        #[arg(long)]
        page: Option<usize>,
        /// Content line within the derived page
        #[arg(long)]
        line: Option<usize>,
        /// Matched text span to cover
        #[arg(long)]
        text: Option<String>,
        /// Cover the Nth text match
        #[arg(long = "match")]
        match_number: Option<usize>,
        /// Preview the edit without calling documents.batchUpdate
        #[arg(long)]
        dry_run: bool,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
        /// Require the document to still be at this revision before applying the edit
        #[arg(long)]
        required_revision_id: Option<String>,
    },
    /// Delete a named range by ID or name
    #[command(after_long_help = "Output shape:
  Prints the raw documents.batchUpdate response JSON (an empty replies array on success).

Notes:
  Provide exactly one of --named-range-id or --name; --name deletes every named range sharing that name.")]
    DeleteNamedRange {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// Exact namedRangeId returned by create-named-range
        #[arg(long)]
        named_range_id: Option<String>,
        /// Name shared by the named range(s) to delete
        #[arg(long)]
        name: Option<String>,
        /// Preview the edit without calling documents.batchUpdate
        #[arg(long)]
        dry_run: bool,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
        /// Require the document to still be at this revision before applying the edit
        #[arg(long)]
        required_revision_id: Option<String>,
    },
    /// Fetch a raw Google Docs Document
    #[command(after_long_help = "Output shape:
  Emits the Google Docs API Document JSON unchanged.
  Top-level metadata includes documentId, title, revisionId, and documentStyle.
  Body content is under body.content as ordered structural elements.
  Common structural elements include paragraph, table, sectionBreak, and tableOfContents.
  Paragraph text is split across paragraph.elements[].textRun.content; indexes are UTF-16 positions used by batch-update requests.
  Tab-aware documents can also return tabs[].documentTab.body.content when --include-tabs-content is set.
  DOCUMENT_ID also accepts a full Google Docs or Drive URL; the Document ID is extracted automatically.
  Word (.docx) files stored on Drive are read too: they are converted to a temporary native Google Doc, read, and the temporary copy is deleted, all transparently.

Tips:
  Use --fields to fetch only the paths you need, for example:
    goog docs get DOCUMENT_ID --fields 'title,body(content(paragraph(elements(textRun(content)))))'
  Use jq to inspect text runs:
    goog docs get DOCUMENT_ID | jq -r '.body.content[]?.paragraph?.elements[]?.textRun?.content // empty'")]
    Get {
        /// Google Docs Document ID or URL to fetch
        document_id: String,
        /// Google partial response field selector
        #[arg(long)]
        fields: Option<String>,
        /// Include tab-aware content in the returned Document
        #[arg(long)]
        include_tabs_content: bool,
    },
    /// Apply a raw Google Docs Batch Update request body
    #[command(after_long_help = "Request shape:
  --requests reads the full Google Docs documents.batchUpdate JSON body, not only the requests array.
  The body usually contains requests: an ordered array of mutation objects.
  It may also contain writeControl when you need revision-aware writes.
  Locations and ranges use the UTF-16 indexes returned by docs get.

Common request types:
  Text: insertText, replaceAllText, deleteContentRange
  Text and paragraph style: updateTextStyle, updateParagraphStyle, createParagraphBullets, deleteParagraphBullets
  Tables and images: insertTable, insertTableRow, insertTableColumn, deleteTableRow, deleteTableColumn, insertInlineImage, replaceImage
  Document structure: insertPageBreak, insertSectionBreak, updateDocumentStyle, updateSectionStyle, createHeader, createFooter, createFootnote
  Tabs: addDocumentTab, deleteTab, updateDocumentTabProperties

Full request type reference:
  https://developers.google.com/workspace/docs/api/reference/rest/v1/documents/request

Example:
  goog docs batch-update DOCUMENT_ID --requests - <<'JSON'
  {
    \"requests\": [
      {
        \"insertText\": {
          \"location\": { \"index\": 1 },
          \"text\": \"Hello from goog-cli\\n\"
        }
      }
    ]
  }
  JSON")]
    BatchUpdate {
        /// Google Docs Document ID or URL to update
        document_id: String,
        /// Path to a full documents.batchUpdate JSON request body, or - for stdin
        #[arg(long)]
        requests: String,
    },
    /// Show the locally cached style template for a Google Doc
    ShowStyleTemplate {
        /// Google Docs Document ID whose cached style template to show
        document_id: String,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DocsSectionBreakType {
    Continuous,
    NextPage,
}

impl DocsSectionBreakType {
    pub fn api_value(self) -> &'static str {
        match self {
            DocsSectionBreakType::Continuous => "CONTINUOUS",
            DocsSectionBreakType::NextPage => "NEXT_PAGE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DocsListType {
    Bullet,
    Numbered,
    Dash,
    Checkbox,
}

#[derive(Debug, Subcommand)]
pub enum MailCommand {
    /// List recent Inbox GoogleMail Messages
    List {
        /// Maximum number of messages to return (default: 10)
        #[arg(long)]
        limit: Option<u32>,
        /// Emit newline-delimited JSON
        #[arg(long)]
        json: bool,
    },
    /// Search GoogleMail Messages with a Gmail Mailbox Query
    Search {
        /// Gmail Mailbox Query to pass through to GoogleMail
        query: String,
        /// Maximum number of messages to return (default: 10)
        #[arg(long)]
        limit: Option<u32>,
        /// Emit newline-delimited JSON
        #[arg(long)]
        json: bool,
    },
    /// Fetch a GoogleMail Message
    Read {
        /// GoogleMail Message ID to fetch
        message_id: String,
        /// Emit the raw GoogleMail Message as JSON instead of Markdown
        #[arg(long)]
        json: bool,
    },
    /// Manage GoogleMail Attachments
    Attachment {
        #[command(subcommand)]
        command: MailAttachmentCommand,
    },
    /// Manage GoogleMail Drafts
    Draft {
        #[command(subcommand)]
        command: MailDraftCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum MailAttachmentCommand {
    /// Download a GoogleMail Attachment
    Download {
        /// GoogleMail Message ID containing the Attachment
        message_id: String,
        /// GoogleMail Attachment ID to download
        attachment_id: String,
        /// Destination path (defaults to Attachment filename)
        #[arg(long, short)]
        output: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum MailDraftCommand {
    /// Create a GoogleMail Draft Message
    Create {
        /// Recipient email address. Repeat for multiple To recipients.
        #[arg(long, required = true)]
        to: Vec<String>,
        /// Cc recipient email address. Repeat for multiple Cc recipients.
        #[arg(long)]
        cc: Vec<String>,
        /// Bcc recipient email address. Repeat for multiple Bcc recipients.
        #[arg(long)]
        bcc: Vec<String>,
        /// Draft subject
        #[arg(long)]
        subject: String,
        /// Plain text draft body
        #[arg(long, conflicts_with = "body_file")]
        body: Option<String>,
        /// Path to a plain text draft body file
        #[arg(long, conflicts_with = "body")]
        body_file: Option<String>,
        /// Local file to attach to the Draft. Repeat for multiple Attachments.
        #[arg(long)]
        attachment: Vec<String>,
        /// Emit the raw GoogleMail Draft as JSON
        #[arg(long)]
        json: bool,
    },
    /// Edit a GoogleMail Draft Message
    Edit {
        /// GoogleMail Draft ID to update
        draft_id: String,
        /// Recipient email address. Repeat for multiple To recipients.
        #[arg(long, required = true)]
        to: Vec<String>,
        /// Cc recipient email address. Repeat for multiple Cc recipients.
        #[arg(long)]
        cc: Vec<String>,
        /// Bcc recipient email address. Repeat for multiple Bcc recipients.
        #[arg(long)]
        bcc: Vec<String>,
        /// Draft subject
        #[arg(long)]
        subject: String,
        /// Plain text draft body
        #[arg(long, conflicts_with = "body_file")]
        body: Option<String>,
        /// Path to a plain text draft body file
        #[arg(long, conflicts_with = "body")]
        body_file: Option<String>,
        /// Local file to attach to the Draft. Repeat for multiple Attachments.
        #[arg(long)]
        attachment: Vec<String>,
        /// Emit the raw GoogleMail Draft as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsValueRenderOption {
    /// Return values formatted as displayed in Google Sheets
    FormattedValue,
    /// Return underlying unformatted values
    UnformattedValue,
    /// Return formulas instead of calculated values
    Formula,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsValueInputOption {
    /// Store values exactly as provided
    Raw,
    /// Parse values as if entered by a user
    UserEntered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsInsertDataOption {
    /// Insert new rows for appended values
    InsertRows,
    /// Overwrite existing data where possible
    Overwrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsDimension {
    /// Sheet rows
    Rows,
    /// Sheet columns
    Columns,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsMergeType {
    /// Merge the full range into one cell
    All,
    /// Merge each row across the selected columns
    Rows,
    /// Merge each column across the selected rows
    Columns,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsSortOrder {
    /// Sort smallest to largest or A to Z
    Ascending,
    /// Sort largest to smallest or Z to A
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsPasteType {
    /// Paste values, formulas, formats, and other cell data
    Normal,
    /// Paste values only
    Values,
    /// Paste formats only
    Format,
    /// Paste formulas only
    Formula,
    /// Paste everything except borders
    NoBorders,
    /// Paste data validation only
    DataValidation,
    /// Paste conditional formatting only
    ConditionalFormatting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsPasteOrientation {
    /// Keep the copied row and column orientation
    Normal,
    /// Transpose rows and columns while pasting
    Transposed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsHorizontalAlignment {
    /// Align cell content to the left
    Left,
    /// Align cell content in the center
    Center,
    /// Align cell content to the right
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsVerticalAlignment {
    /// Align cell content to the top
    Top,
    /// Align cell content in the middle
    Middle,
    /// Align cell content to the bottom
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsWrapStrategy {
    /// Let text overflow into the next empty cell
    Overflow,
    /// Wrap text onto multiple lines within the cell
    Wrap,
    /// Clip text at the cell boundary
    Clip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsTextDirection {
    /// Display cell text left to right
    LeftToRight,
    /// Display cell text right to left
    RightToLeft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsNumberFormatType {
    /// Plain text
    Text,
    /// General number formatting
    Number,
    /// Percent formatting
    Percent,
    /// Currency formatting
    Currency,
    /// Date formatting
    Date,
    /// Time formatting
    Time,
    /// Date and time formatting
    DateTime,
    /// Scientific notation formatting
    Scientific,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsBorderEdge {
    /// Apply to every outside and inside edge in the selected range
    All,
    /// Apply to the outside edges of the selected range
    Outer,
    /// Apply to the inside edges of the selected range
    Inner,
    /// Apply to the top edge of the selected range
    Top,
    /// Apply to the bottom edge of the selected range
    Bottom,
    /// Apply to the left edge of the selected range
    Left,
    /// Apply to the right edge of the selected range
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsBorderStyle {
    /// Remove the selected border edges
    None,
    /// Thin solid border
    Solid,
    /// Medium solid border
    SolidMedium,
    /// Thick solid border
    SolidThick,
    /// Dashed border
    Dashed,
    /// Dotted border
    Dotted,
    /// Double-line border
    Double,
}

#[derive(Debug, Subcommand)]
pub enum SheetsCommand {
    /// Create a new, blank Google Sheets Spreadsheet
    #[command(after_long_help = "Output shape:
  Prints the created Spreadsheet ID and its Google Sheets edit URL, tab-separated.

Notes:
  The Spreadsheet is always created at the root of My Drive; there is no --folder option today.
  Move it afterward with the Google Drive web UI, or via a future `goog drive` move command.
  Follow up with `goog sheets values append-row` or `goog sheets values append-table` to add rows.")]
    Create {
        /// Title for the new Google Sheets Spreadsheet
        title: String,
    },
    /// Fetch raw Google Sheets Spreadsheet metadata
    Get {
        /// Google Sheets Spreadsheet ID to fetch
        spreadsheet_id: String,
        /// Google partial response field selector
        #[arg(long)]
        fields: Option<String>,
        /// Include grid data in the returned Spreadsheet
        #[arg(long)]
        include_grid_data: bool,
        /// Limit returned grid data to a Google Sheets A1 Range
        #[arg(long = "ranges")]
        ranges: Vec<String>,
    },
    /// Read and write Google Sheets cell values
    Values {
        #[command(subcommand)]
        command: SheetsValuesCommand,
    },
    /// Manage individual sheets inside a Spreadsheet
    Sheet {
        #[command(subcommand)]
        command: SheetsSheetCommand,
    },
    /// Apply a raw Google Sheets structural Batch Update request body
    #[command(after_long_help = "Request shape:
  --requests reads the full Google Sheets spreadsheets.batchUpdate JSON body, not only the requests array.
  The body usually contains requests: an ordered array of structural mutation objects.
  It may also contain includeSpreadsheetInResponse, responseRanges, and responseIncludeGridData.

Common request types:
  Sheets: addSheet, duplicateSheet, deleteSheet, updateSheetProperties
  Formatting: repeatCell, updateCells, updateBorders, mergeCells, unmergeCells
  Structure: updateDimensionProperties, addFilterView, setBasicFilter, addProtectedRange

Full request type reference:
  https://developers.google.com/workspace/sheets/api/reference/rest/v4/spreadsheets/request

Example:
  goog sheets batch-update SPREADSHEET_ID --requests - <<'JSON'
  {
    \"requests\": [
      {
        \"addSheet\": {
          \"properties\": { \"title\": \"New sheet\" }
        }
      }
    ],
    \"includeSpreadsheetInResponse\": false
  }
  JSON")]
    BatchUpdate {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Path to a full spreadsheets.batchUpdate JSON request body, or - for stdin
        #[arg(long)]
        requests: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum SheetsSheetCommand {
    /// Add a new sheet tab without writing a Batch Update JSON body
    Add {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Title for the new sheet tab
        title: String,
        /// Optional Google Sheets numeric sheetId for the new tab
        #[arg(long)]
        sheet_id: Option<i64>,
        /// Zero-based index where Google Sheets should place the new tab
        #[arg(long)]
        index: Option<i64>,
    },
    /// Delete a sheet tab without writing a Batch Update JSON body
    Delete {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to delete
        sheet_id: i64,
    },
    /// Rename a sheet tab without writing a Batch Update JSON body
    Rename {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to rename
        sheet_id: i64,
        /// New title for the sheet tab
        title: String,
    },
    /// Move a sheet tab to a zero-based index without writing a Batch Update JSON body
    Move {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to move
        sheet_id: i64,
        /// Zero-based index where Google Sheets should place the tab
        index: i64,
    },
    /// Duplicate a sheet tab without writing a Batch Update JSON body
    Duplicate {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to duplicate
        source_sheet_id: i64,
        /// Title for the duplicated sheet tab
        title: String,
        /// Optional Google Sheets numeric sheetId for the duplicated tab
        #[arg(long)]
        sheet_id: Option<i64>,
        /// Zero-based index where Google Sheets should place the duplicated tab
        #[arg(long)]
        index: Option<i64>,
    },
    /// Freeze rows or columns without writing a Batch Update JSON body
    Freeze {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Number of frozen rows, or 0 to unfreeze rows
        #[arg(long, required_unless_present = "columns", value_parser = clap::value_parser!(i64).range(0..))]
        rows: Option<i64>,
        /// Number of frozen columns, or 0 to unfreeze columns
        #[arg(long, required_unless_present = "rows", value_parser = clap::value_parser!(i64).range(0..))]
        columns: Option<i64>,
    },
    /// Resize a sheet grid without writing a Batch Update JSON body
    Resize {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Total row count for the sheet grid
        #[arg(long, required_unless_present = "columns", value_parser = clap::value_parser!(i64).range(1..))]
        rows: Option<i64>,
        /// Total column count for the sheet grid
        #[arg(long, required_unless_present = "rows", value_parser = clap::value_parser!(i64).range(1..))]
        columns: Option<i64>,
    },
    /// Auto-resize rows or columns without writing a Batch Update JSON body
    AutoResize {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Dimension to auto-resize
        #[arg(long)]
        dimension: SheetsDimension,
        /// Zero-based inclusive start index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_index: i64,
        /// Zero-based exclusive end index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_index: i64,
    },
    /// Set row height or column width without writing a Batch Update JSON body
    SetDimensionSize {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Dimension to resize
        #[arg(long)]
        dimension: SheetsDimension,
        /// Zero-based inclusive start index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_index: i64,
        /// Zero-based exclusive end index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_index: i64,
        /// Pixel size for each selected row or column
        #[arg(long, value_parser = clap::value_parser!(i64).range(1..))]
        pixel_size: i64,
    },
    /// Insert rows or columns without writing a Batch Update JSON body
    InsertDimension {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Dimension to insert
        #[arg(long)]
        dimension: SheetsDimension,
        /// Zero-based inclusive start index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_index: i64,
        /// Zero-based exclusive end index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_index: i64,
        /// Inherit formatting from the row or column before the inserted range
        #[arg(long)]
        inherit_from_before: bool,
    },
    /// Delete rows or columns without writing a Batch Update JSON body
    DeleteDimension {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Dimension to delete
        #[arg(long)]
        dimension: SheetsDimension,
        /// Zero-based inclusive start index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_index: i64,
        /// Zero-based exclusive end index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_index: i64,
    },
    /// Set a basic filter over a grid range without writing a Batch Update JSON body
    BasicFilter {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
    },
    /// Merge cells over a grid range without writing a Batch Update JSON body
    Merge {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Merge behavior for the selected range
        #[arg(long, value_enum, default_value = "all")]
        merge_type: SheetsMergeType,
    },
    /// Unmerge cells over a grid range without writing a Batch Update JSON body
    Unmerge {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
    },
    /// Sort rows over a grid range without writing a Batch Update JSON body
    SortRange {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Zero-based column index to sort by
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        sort_column: i64,
        /// Sort direction
        #[arg(long, value_enum, default_value = "ascending")]
        order: SheetsSortOrder,
    },
    /// Find and replace text without writing a Batch Update JSON body
    FindReplace {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Text or regex pattern to find
        find: String,
        /// Replacement text
        replacement: String,
        /// Limit replacement to one Google Sheets numeric sheetId
        #[arg(long)]
        sheet_id: Option<i64>,
        /// Match case exactly
        #[arg(long)]
        match_case: bool,
        /// Match the entire cell value
        #[arg(long)]
        match_entire_cell: bool,
        /// Interpret the find text as a regular expression
        #[arg(long = "regex")]
        search_by_regex: bool,
        /// Search formulas in addition to displayed values
        #[arg(long)]
        include_formulas: bool,
    },
    /// Copy cells from one grid range to another without writing a Batch Update JSON body
    CopyPaste {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the source tab
        source_sheet_id: i64,
        /// Zero-based inclusive source start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        source_start_row: i64,
        /// Zero-based exclusive source end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        source_end_row: i64,
        /// Zero-based inclusive source start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        source_start_column: i64,
        /// Zero-based exclusive source end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        source_end_column: i64,
        /// Google Sheets numeric sheetId for the destination tab
        #[arg(long)]
        destination_sheet_id: i64,
        /// Zero-based inclusive destination start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        destination_start_row: i64,
        /// Zero-based exclusive destination end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        destination_end_row: i64,
        /// Zero-based inclusive destination start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        destination_start_column: i64,
        /// Zero-based exclusive destination end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        destination_end_column: i64,
        /// What to paste from the source range
        #[arg(long, value_enum, default_value = "normal")]
        paste_type: SheetsPasteType,
        /// Whether to transpose rows and columns while pasting
        #[arg(long, value_enum, default_value = "normal")]
        paste_orientation: SheetsPasteOrientation,
    },
    /// Move cells from one grid range to a top-left coordinate without writing a Batch Update JSON body
    CutPaste {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the source tab
        source_sheet_id: i64,
        /// Zero-based inclusive source start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        source_start_row: i64,
        /// Zero-based exclusive source end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        source_end_row: i64,
        /// Zero-based inclusive source start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        source_start_column: i64,
        /// Zero-based exclusive source end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        source_end_column: i64,
        /// Google Sheets numeric sheetId for the destination tab
        #[arg(long)]
        destination_sheet_id: i64,
        /// Zero-based destination row for the top-left pasted cell
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        destination_row: i64,
        /// Zero-based destination column for the top-left pasted cell
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        destination_column: i64,
        /// What to paste from the source range
        #[arg(long, value_enum, default_value = "normal")]
        paste_type: SheetsPasteType,
    },
    /// Set a cell range background color without writing a Batch Update JSON body
    BackgroundColor {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Hex background color for the selected cells, as RRGGBB or #RRGGBB
        color: String,
    },
    /// Set a cell range text color without writing a Batch Update JSON body
    TextColor {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Hex text color for the selected cells, as RRGGBB or #RRGGBB
        color: String,
    },
    /// Set font size over a cell range without writing a Batch Update JSON body
    FontSize {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Font size in points
        #[arg(long, value_parser = clap::value_parser!(i64).range(1..))]
        size: i64,
    },
    /// Set font family over a cell range without writing a Batch Update JSON body
    FontFamily {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Font family name, such as Arial or Roboto
        #[arg(long)]
        family: String,
    },
    /// Set number formatting over a cell range without writing a Batch Update JSON body
    NumberFormat {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Number format type to apply
        #[arg(long = "type", value_enum)]
        format_type: SheetsNumberFormatType,
        /// Google Sheets number format pattern, such as #,##0.00 or m/d/yyyy
        #[arg(long)]
        pattern: Option<String>,
    },
    /// Set cell borders over a range without writing a Batch Update JSON body
    Borders {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Border edge group to update. Repeat for multiple edges.
        #[arg(long, value_enum, default_value = "all")]
        edge: Vec<SheetsBorderEdge>,
        /// Border line style to apply
        #[arg(long, value_enum, default_value = "solid")]
        style: SheetsBorderStyle,
        /// Optional hex border color, as RRGGBB or #RRGGBB
        #[arg(long)]
        color: Option<String>,
    },
    /// Clear cell formatting over a range without writing a Batch Update JSON body
    ClearFormat {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
    },
    /// Set bold text style over a cell range without writing a Batch Update JSON body
    Bold {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Clear bold style instead of applying it
        #[arg(long)]
        off: bool,
    },
    /// Set italic text style over a cell range without writing a Batch Update JSON body
    Italic {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Clear italic style instead of applying it
        #[arg(long)]
        off: bool,
    },
    /// Set underline text style over a cell range without writing a Batch Update JSON body
    Underline {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Clear underline style instead of applying it
        #[arg(long)]
        off: bool,
    },
    /// Set strikethrough text style over a cell range without writing a Batch Update JSON body
    Strikethrough {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Clear strikethrough style instead of applying it
        #[arg(long)]
        off: bool,
    },
    /// Set horizontal alignment over a cell range without writing a Batch Update JSON body
    HorizontalAlign {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Horizontal alignment to apply
        #[arg(long, value_enum)]
        alignment: SheetsHorizontalAlignment,
    },
    /// Set vertical alignment over a cell range without writing a Batch Update JSON body
    VerticalAlign {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Vertical alignment to apply
        #[arg(long, value_enum)]
        alignment: SheetsVerticalAlignment,
    },
    /// Set text wrapping over a cell range without writing a Batch Update JSON body
    TextWrap {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Text wrapping behavior to apply
        #[arg(long, value_enum)]
        strategy: SheetsWrapStrategy,
    },
    /// Set text rotation over a cell range without writing a Batch Update JSON body
    TextRotation {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Text rotation angle from -90 to 90 degrees
        #[arg(long, value_parser = clap::value_parser!(i64).range(-90..=90))]
        angle: i64,
    },
    /// Set text direction over a cell range without writing a Batch Update JSON body
    TextDirection {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Text direction to apply
        #[arg(long, value_enum)]
        direction: SheetsTextDirection,
    },
    /// Set a cell note over a range without writing a Batch Update JSON body
    Note {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Note text to set on each selected cell
        #[arg(required_unless_present = "clear", conflicts_with = "clear")]
        note: Option<String>,
        /// Clear notes from the selected cells instead of setting note text
        #[arg(long)]
        clear: bool,
    },
    /// Set or clear dropdown list data validation over a range without writing a Batch Update JSON body
    DataValidationList {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based inclusive start row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: i64,
        /// Zero-based exclusive end row
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: i64,
        /// Zero-based inclusive start column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: i64,
        /// Zero-based exclusive end column
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: i64,
        /// Allowed dropdown value. Repeat for multiple values.
        #[arg(
            long = "value",
            required_unless_present = "clear",
            conflicts_with = "clear"
        )]
        values: Vec<String>,
        /// Show a warning instead of rejecting invalid values
        #[arg(long)]
        allow_invalid: bool,
        /// Hide the dropdown picker in the Google Sheets UI
        #[arg(long)]
        hide_dropdown: bool,
        /// Optional validation help text shown in Google Sheets
        #[arg(long)]
        input_message: Option<String>,
        /// Clear data validation from the selected cells instead of setting a dropdown list
        #[arg(long)]
        clear: bool,
    },
    /// Clear the basic filter from a sheet without writing a Batch Update JSON body
    ClearBasicFilter {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
    },
    /// Set a sheet tab color without writing a Batch Update JSON body
    TabColor {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Hex color for the sheet tab, as RRGGBB or #RRGGBB
        color: String,
    },
    /// Clear a sheet tab color without writing a Batch Update JSON body
    ClearTabColor {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
    },
    /// Hide a sheet tab without writing a Batch Update JSON body
    Hide {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to hide
        sheet_id: i64,
    },
    /// Unhide a sheet tab without writing a Batch Update JSON body
    Unhide {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to unhide
        sheet_id: i64,
    },
}

#[derive(Debug, Subcommand)]
pub enum SheetsValuesCommand {
    /// Fetch a raw Google Sheets ValueRange
    Get {
        /// Google Sheets Spreadsheet ID to fetch
        spreadsheet_id: String,
        /// Google Sheets A1 Range to fetch
        range: String,
        /// How values should be represented in the response
        #[arg(long, value_enum, default_value = "formatted-value")]
        value_render_option: SheetsValueRenderOption,
    },
    /// Fetch raw Google Sheets values from multiple ranges
    BatchGet {
        /// Google Sheets Spreadsheet ID to fetch
        spreadsheet_id: String,
        /// Google Sheets A1 Range to fetch
        #[arg(long = "range", required = true)]
        ranges: Vec<String>,
        /// How values should be represented in the response
        #[arg(long, value_enum, default_value = "formatted-value")]
        value_render_option: SheetsValueRenderOption,
    },
    /// Update a Google Sheets ValueRange
    Update {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets A1 Range to update
        range: String,
        /// Path to a Google ValueRange JSON request body, or - for stdin
        #[arg(long)]
        values: String,
        /// How input values should be interpreted
        #[arg(long, value_enum, default_value = "user-entered")]
        value_input_option: SheetsValueInputOption,
    },
    /// Update a Google Sheets Range from CSV or TSV without writing a ValueRange JSON body
    UpdateTable {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets A1 Range to update
        range: String,
        /// CSV or TSV data file to write
        #[arg(long)]
        data: String,
        /// How input values should be interpreted
        #[arg(long, value_enum, default_value = "user-entered")]
        value_input_option: SheetsValueInputOption,
    },
    /// Update one row without writing a ValueRange JSON body
    UpdateRow {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets A1 Range to update
        range: String,
        /// Cell value to write. Repeat once per column.
        #[arg(long = "value", required = true)]
        values: Vec<String>,
        /// How input values should be interpreted
        #[arg(long, value_enum, default_value = "user-entered")]
        value_input_option: SheetsValueInputOption,
    },
    /// Batch update Google Sheets values
    BatchUpdate {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Path to a full spreadsheets.values.batchUpdate JSON request body, or - for stdin
        #[arg(long)]
        values: String,
    },
    /// Append values to a Google Sheets Range
    Append {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets A1 Range to append into
        range: String,
        /// Path to a Google ValueRange JSON request body, or - for stdin
        #[arg(long)]
        values: String,
        /// How input values should be interpreted
        #[arg(long, value_enum, default_value = "user-entered")]
        value_input_option: SheetsValueInputOption,
        /// How Google Sheets should insert appended data
        #[arg(long, value_enum, default_value = "insert-rows")]
        insert_data_option: SheetsInsertDataOption,
    },
    /// Append one row without writing a ValueRange JSON body
    AppendRow {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets A1 Range to append into
        range: String,
        /// Cell value to append. Repeat once per column.
        #[arg(long = "value", required = true)]
        values: Vec<String>,
        /// How input values should be interpreted
        #[arg(long, value_enum, default_value = "user-entered")]
        value_input_option: SheetsValueInputOption,
        /// How Google Sheets should insert appended data
        #[arg(long, value_enum, default_value = "insert-rows")]
        insert_data_option: SheetsInsertDataOption,
    },
    /// Append CSV or TSV rows without writing a ValueRange JSON body
    AppendTable {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets A1 Range to append into
        range: String,
        /// CSV or TSV data file to append
        #[arg(long)]
        data: String,
        /// How input values should be interpreted
        #[arg(long, value_enum, default_value = "user-entered")]
        value_input_option: SheetsValueInputOption,
        /// How Google Sheets should insert appended data
        #[arg(long, value_enum, default_value = "insert-rows")]
        insert_data_option: SheetsInsertDataOption,
    },
    /// Clear values from a Google Sheets Range
    Clear {
        /// Google Sheets Spreadsheet ID to clear
        spreadsheet_id: String,
        /// Google Sheets A1 Range to clear
        range: String,
    },
    /// Clear values from multiple Google Sheets Ranges
    BatchClear {
        /// Google Sheets Spreadsheet ID to clear
        spreadsheet_id: String,
        /// Google Sheets A1 Range to clear
        #[arg(long = "range", required = true)]
        ranges: Vec<String>,
    },
}
