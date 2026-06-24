use clap::{Parser, Subcommand};

use crate::auth::config::OAuthAppType;

#[derive(Debug, Parser)]
#[command(name = "goog", about = "A CLI for Google APIs")]
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

#[derive(Debug, Subcommand)]
pub enum DocsCommand {
    /// Fetch a raw Google Docs Document
    #[command(after_long_help = "Output shape:
  Emits the Google Docs API Document JSON unchanged.
  Top-level metadata includes documentId, title, revisionId, and documentStyle.
  Body content is under body.content as ordered structural elements.
  Common structural elements include paragraph, table, sectionBreak, and tableOfContents.
  Paragraph text is split across paragraph.elements[].textRun.content; indexes are UTF-16 positions used by batch-update requests.
  Tab-aware documents can also return tabs[].documentTab.body.content when --include-tabs-content is set.

Tips:
  Use --fields to fetch only the paths you need, for example:
    goog docs get DOCUMENT_ID --fields 'title,body(content(paragraph(elements(textRun(content)))))'
  Use jq to inspect text runs:
    goog docs get DOCUMENT_ID | jq -r '.body.content[]?.paragraph?.elements[]?.textRun?.content // empty'")]
    Get {
        /// Google Docs Document ID to fetch
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
        /// Google Docs Document ID to update
        document_id: String,
        /// Path to a full documents.batchUpdate JSON request body, or - for stdin
        #[arg(long)]
        requests: String,
    },
}
