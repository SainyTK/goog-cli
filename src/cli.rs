use clap::{ArgGroup, Parser, Subcommand, ValueEnum};

use crate::auth::config::OAuthAppType;

const DOCS_CONTENT_SELECTOR_HELP: &str = "Selector rules:
  Provide exactly one content selector.
  Use --index N, --entry N, --page P --line L, or --heading TEXT.";

const DOCS_INSERT_SELECTOR_HELP: &str = "Selector rules:
  Provide exactly one insert location selector with --at.
  Use --at index:N, --at entry:N, --at page:P,line:L, --at heading:TEXT, --at after-heading:TEXT, --at before-heading:TEXT, --at after-text:TEXT, or --at before-text:TEXT.

Write safety:
  Use --dry-run to preview without calling documents.batchUpdate.
  Use --required-revision-id REVISION_ID to reject writes against a changed document.";

const DOCS_RANGE_SELECTOR_HELP: &str = "Selector rules:
  Provide exactly one range selector.
  Use --from-index START --to-index END, --entry N, --page P --line L, or --text TEXT with optional --match N.

Write safety:
  Use --dry-run to preview without calling documents.batchUpdate.
  Use --required-revision-id REVISION_ID to reject writes against a changed document.";

const DOCS_CREATE_NAMED_RANGE_HELP: &str = "Output shape:
  Prints the raw documents.batchUpdate response JSON, which includes the new namedRangeId under replies[0].createNamedRange.namedRangeId.

Selector rules:
  Provide exactly one range selector.
  Use --from-index START --to-index END, --entry N, --page P --line L, or --text TEXT with optional --match N.

Notes:
  Named ranges do not track edits after creation; a later insert/delete before the range can leave it pointing at the wrong text.
  Re-create the named range after content in or before it changes.";

fn parse_mail_draft_id(value: &str) -> Result<String, String> {
    match value {
        "create" | "edit" => Err(format!(
            "`mail draft {value}` was removed; use `mail draft [DRAFT_ID]` instead"
        )),
        _ => Ok(value.to_owned()),
    }
}

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
    /// Interact with Gmail
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
    /// List files and folders in Google Drive
    Ls {
        /// Maximum number of items to return (default: 50)
        #[arg(long)]
        limit: Option<u32>,
        /// List all items across all pages. Caps at --limit when both are given
        #[arg(long)]
        all: bool,
        /// Type of Drive items to list
        #[arg(long = "type", value_enum, default_value_t = DriveListType::Items)]
        type_: DriveListType,
        /// Drive folder ID to browse
        #[arg(long)]
        folder: Option<String>,
        /// Emit one JSON object per row. Items use browse row fields; files and folders use full Drive file JSON
        #[arg(long)]
        json: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DriveListType {
    /// Files and folders
    Items,
    /// Files only
    Files,
    /// Folders only
    Folders,
}

impl DocsCommand {
    /// Resolves the `document_id` field of any variant to a bare Document ID,
    /// extracting it first if a Google Docs/Drive URL was passed instead.
    pub fn normalize_document_id(&mut self) {
        let document_id = match self {
            DocsCommand::Create { .. } => return,
            DocsCommand::Map { document_id, .. }
            | DocsCommand::Get { document_id, .. }
            | DocsCommand::BatchUpdate { document_id, .. } => document_id,
            DocsCommand::List { command } => match command {
                DocsListCommand::Apply { document_id, .. } => document_id,
            },
            DocsCommand::Style { command } => match command {
                DocsStyleCommand::Apply { document_id, .. }
                | DocsStyleCommand::Template { document_id, .. } => document_id,
            },
            DocsCommand::Header { command } => match command {
                DocsHeaderCommand::Create { document_id, .. } => document_id,
            },
            DocsCommand::Footer { command } => match command {
                DocsFooterCommand::Create { document_id, .. } => document_id,
            },
            DocsCommand::Break { command } => match command {
                DocsBreakCommand::Page { document_id, .. }
                | DocsBreakCommand::Section { document_id, .. } => document_id,
            },
            DocsCommand::Footnote { command } => match command {
                DocsFootnoteCommand::Insert { document_id, .. } => document_id,
            },
            DocsCommand::Image { command } => match command {
                DocsImageCommand::Insert { document_id, .. } => document_id,
            },
            DocsCommand::Table { command } => match command {
                DocsTableCommand::Insert { document_id, .. }
                | DocsTableCommand::Edit { document_id, .. } => document_id,
            },
            DocsCommand::NamedRange { command } => match command {
                DocsNamedRangeCommand::Create { document_id, .. }
                | DocsNamedRangeCommand::Delete { document_id, .. } => document_id,
            },
            DocsCommand::Text { command } => match command {
                DocsTextCommand::Search { document_id, .. }
                | DocsTextCommand::Insert { document_id, .. }
                | DocsTextCommand::Replace { document_id, .. } => document_id,
            },
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
    /// Print a high-level map of editable Google Docs content, or retrieve one selected block
    #[command(after_long_help = DOCS_CONTENT_SELECTOR_HELP)]
    Map {
        /// Document ID or URL to map
        document_id: String,
        /// Type of map entries to show
        #[arg(long = "type", value_enum, default_value_t = DocsMapType::All)]
        type_: DocsMapType,
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
    /// Search, insert, or replace document text
    Text {
        #[command(subcommand)]
        command: DocsTextCommand,
    },
    /// Insert document images
    Image {
        #[command(subcommand)]
        command: DocsImageCommand,
    },
    /// Insert document page or section breaks
    Break {
        #[command(subcommand)]
        command: DocsBreakCommand,
    },
    /// Insert document footnotes
    Footnote {
        #[command(subcommand)]
        command: DocsFootnoteCommand,
    },
    /// Create document headers
    Header {
        #[command(subcommand)]
        command: DocsHeaderCommand,
    },
    /// Create document footers
    Footer {
        #[command(subcommand)]
        command: DocsFooterCommand,
    },
    /// Insert or edit tables
    Table {
        #[command(subcommand)]
        command: DocsTableCommand,
    },
    /// Apply or inspect document styles
    Style {
        #[command(subcommand)]
        command: DocsStyleCommand,
    },
    /// Apply document list formatting
    List {
        #[command(subcommand)]
        command: DocsListCommand,
    },
    /// Create or delete named ranges
    NamedRange {
        #[command(subcommand)]
        command: DocsNamedRangeCommand,
    },
    /// Read a raw Google Docs Document
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
  Use --fields to return only the paths you need, for example:
    goog docs get DOCUMENT_ID --fields 'title,body(content(paragraph(elements(textRun(content)))))'
  Use jq to inspect text runs:
    goog docs get DOCUMENT_ID | jq -r '.body.content[]?.paragraph?.elements[]?.textRun?.content // empty'")]
    Get {
        /// Document ID or URL to read
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
        /// Document ID or URL to update
        document_id: String,
        /// Path to a full documents.batchUpdate JSON request body, or - for stdin
        #[arg(long)]
        requests: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum DocsStyleCommand {
    /// Apply common text styles through a high-level Document Range
    #[command(after_long_help = DOCS_RANGE_SELECTOR_HELP)]
    Apply {
        /// Document ID or URL to update
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
        #[arg(long = "paragraph-style", value_name = "PARAGRAPH_STYLE")]
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
        no_cached_style: bool,
    },
    /// Read the locally cached style template for a Google Doc
    Template {
        /// Document ID whose cached style template to read
        document_id: String,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum DocsTextCommand {
    /// Search editable Google Docs content through the Document Map
    Search {
        /// Document ID or URL to search
        document_id: String,
        /// Text to find
        text: String,
        /// Emit structured JSON
        #[arg(long)]
        json: bool,
    },
    /// Insert text through a high-level Document Map location selector
    #[command(after_long_help = DOCS_INSERT_SELECTOR_HELP)]
    Insert {
        /// Document ID or URL to update
        document_id: String,
        /// Text to insert
        text: String,
        /// Insert location selector
        #[arg(long, value_name = "SELECTOR")]
        at: String,
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
    Replace {
        /// Document ID or URL to update
        document_id: String,
        /// Existing text to replace
        #[arg(long = "find")]
        old_text: String,
        /// Replacement text
        #[arg(long = "replace")]
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
}

#[derive(Debug, Subcommand)]
pub enum DocsHeaderCommand {
    /// Create the document's default header, returning its headerId
    #[command(after_long_help = "Output shape:
  Prints the raw documents.batchUpdate response JSON, which includes the new headerId under replies[0].createHeader.headerId.

Notes:
  Always creates the DEFAULT header for the document's first section; there is no per-section header support today.
  Edit the header's own content with `goog docs text insert`/`goog docs batch-update`, targeting a location inside the returned headerId segment.")]
    Create {
        /// Document ID or URL to update
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
}

#[derive(Debug, Subcommand)]
pub enum DocsFooterCommand {
    /// Create the document's default footer, returning its footerId
    #[command(after_long_help = "Output shape:
  Prints the raw documents.batchUpdate response JSON, which includes the new footerId under replies[0].createFooter.footerId.

Notes:
  Always creates the DEFAULT footer for the document's first section; there is no per-section footer support today.
  Edit the footer's own content with `goog docs text insert`/`goog docs batch-update`, targeting a location inside the returned footerId segment.")]
    Create {
        /// Document ID or URL to update
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
}

#[derive(Debug, Subcommand)]
pub enum DocsBreakCommand {
    /// Insert a page break through a high-level Document Map location selector
    #[command(after_long_help = DOCS_INSERT_SELECTOR_HELP)]
    Page {
        /// Document ID or URL to update
        document_id: String,
        /// Insert location selector
        #[arg(long, value_name = "SELECTOR")]
        at: String,
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
    #[command(after_long_help = DOCS_INSERT_SELECTOR_HELP)]
    Section {
        /// Document ID or URL to update
        document_id: String,
        /// Section break type
        #[arg(long, value_enum, default_value_t = DocsSectionBreakType::NextPage)]
        section_type: DocsSectionBreakType,
        /// Insert location selector
        #[arg(long, value_name = "SELECTOR")]
        at: String,
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
}

#[derive(Debug, Subcommand)]
pub enum DocsImageCommand {
    /// Insert an Inline Image through a high-level Document Map location selector
    #[command(after_long_help = DOCS_INSERT_SELECTOR_HELP)]
    Insert {
        /// Document ID or URL to update
        document_id: String,
        /// Publicly reachable image URI for Google Docs insertInlineImage
        image_uri: String,
        /// Insert location selector
        #[arg(long, value_name = "SELECTOR")]
        at: String,
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
}

#[derive(Debug, Subcommand)]
pub enum DocsFootnoteCommand {
    /// Insert a footnote at a high-level Document Map location, returning its footnoteId
    #[command(after_long_help = "Output shape:
  Prints the raw documents.batchUpdate response JSON, which includes the new footnoteId under replies[0].createFootnote.footnoteId.

Notes:
  Provide exactly one insert location selector with --at.
  Use --at index:N, --at entry:N, --at page:P,line:L, --at heading:TEXT, --at after-heading:TEXT, --at before-heading:TEXT, --at after-text:TEXT, or --at before-text:TEXT.
  The footnote reference is inserted at the resolved location; the footnote's own body starts empty.
  Edit the footnote's own content with `goog docs text insert`/`goog docs batch-update`, targeting a location inside the returned footnoteId segment.")]
    Insert {
        /// Document ID or URL to update
        document_id: String,
        /// Insert location selector
        #[arg(long, value_name = "SELECTOR")]
        at: String,
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
}

#[derive(Debug, Subcommand)]
pub enum DocsTableCommand {
    /// Insert a table through a high-level Document Map location selector
    #[command(after_long_help = DOCS_INSERT_SELECTOR_HELP)]
    Insert {
        /// Document ID or URL to update
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
        /// Insert location selector
        #[arg(long, value_name = "SELECTOR")]
        at: String,
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
    Edit {
        /// Document ID or URL to update
        document_id: String,
        /// Table handle from `docs map --type tables`, such as table-3
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
}

#[derive(Debug, Subcommand)]
pub enum DocsNamedRangeCommand {
    /// Create a named range over a high-level Document Range, returning its namedRangeId
    #[command(after_long_help = DOCS_CREATE_NAMED_RANGE_HELP)]
    Create {
        /// Document ID or URL to update
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
    Delete {
        /// Document ID or URL to update
        document_id: String,
        /// Exact namedRangeId returned by named-range create
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
}

#[derive(Debug, Subcommand)]
pub enum DocsListCommand {
    /// Apply a common list preset through a high-level Document Range
    #[command(after_long_help = DOCS_RANGE_SELECTOR_HELP)]
    Apply {
        /// Document ID or URL to update
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
        /// Matched text span to list
        #[arg(long)]
        text: Option<String>,
        /// List the Nth text match
        #[arg(long = "match")]
        match_number: Option<usize>,
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
        no_cached_style: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DocsMapType {
    /// All map entries
    All,
    /// Inline and positioned images
    Images,
    /// Tables
    Tables,
}

#[derive(Debug, Subcommand)]
pub enum MailCommand {
    /// List recent Inbox messages, or search Gmail when a query is provided
    List {
        /// Gmail search query. Omit to list recent Inbox messages.
        query: Option<String>,
        /// Maximum number of messages to return (default: 10)
        #[arg(long)]
        limit: Option<u32>,
        /// Emit JSON instead of human-readable output
        #[arg(long)]
        json: bool,
    },
    /// Read a Gmail message
    Read {
        /// Gmail message ID or URL to read
        message_id: String,
        /// Emit JSON instead of human-readable output
        #[arg(long)]
        json: bool,
    },
    /// Download a Gmail attachment
    Download {
        /// Gmail message ID or URL containing the attachment
        message_id: String,
        /// Gmail attachment ID to download. Omit when the message has one attachment.
        attachment_id: Option<String>,
        /// Destination path (defaults to attachment filename)
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Create or edit a Gmail draft message
    Draft {
        /// Gmail draft ID or URL to update. Omit to create a new draft.
        #[arg(value_parser = parse_mail_draft_id)]
        draft_id: Option<String>,
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
        /// Plain text draft body, @path for a body file, or - for stdin
        #[arg(long)]
        body: Option<String>,
        /// Local file to attach to the draft. Repeat for multiple attachments.
        #[arg(long)]
        attachment: Vec<String>,
        /// Emit JSON instead of human-readable output
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

#[derive(Debug, Subcommand)]
pub enum SheetsCommand {
    /// Read raw spreadsheet metadata
    Get {
        /// Spreadsheet ID to read
        spreadsheet_id: String,
        /// Google partial response field selector
        #[arg(long)]
        fields: Option<String>,
        /// Include grid data in the returned Spreadsheet
        #[arg(long)]
        include_grid_data: bool,
        /// Limit returned grid data to an A1 range. Repeat for multiple ranges.
        #[arg(long = "range")]
        ranges: Vec<String>,
    },
    /// Read and write Google Sheets cell values
    Values {
        #[command(subcommand)]
        command: SheetsValuesCommand,
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
        /// Spreadsheet ID to update
        spreadsheet_id: String,
        /// Path to a full spreadsheets.batchUpdate JSON request body, or - for stdin
        #[arg(long)]
        requests: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum SheetsValuesCommand {
    /// Read raw sheet values
    #[command(group(ArgGroup::new("values_get_range").required(true).args(["range", "ranges"])))]
    Get {
        /// Spreadsheet ID to read
        spreadsheet_id: String,
        /// Single A1 range to read
        range: Option<String>,
        /// A1 range to read. Repeat for multiple ranges.
        #[arg(long = "range")]
        ranges: Vec<String>,
        /// How values should be represented in the response
        #[arg(long, value_enum, default_value = "formatted-value")]
        value_render_option: SheetsValueRenderOption,
    },
    /// Update a Google Sheets ValueRange
    Update {
        /// Spreadsheet ID to update
        spreadsheet_id: String,
        /// A1 range to update. Omit to pass a full spreadsheets.values.batchUpdate body.
        range: Option<String>,
        /// Path to a Google ValueRange JSON request body, or - for stdin
        #[arg(long)]
        values: String,
        /// How input values should be interpreted when RANGE is provided
        #[arg(long, value_enum, default_value = "user-entered")]
        value_input_option: SheetsValueInputOption,
    },
    /// Append values to a range
    Append {
        /// Spreadsheet ID to update
        spreadsheet_id: String,
        /// A1 range to append into
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
    /// Clear values from one or more ranges
    #[command(group(ArgGroup::new("values_clear_range").required(true).args(["range", "ranges"])))]
    Clear {
        /// Spreadsheet ID to clear
        spreadsheet_id: String,
        /// Single A1 range to clear
        range: Option<String>,
        /// A1 range to clear. Repeat for multiple ranges.
        #[arg(long = "range")]
        ranges: Vec<String>,
    },
}
