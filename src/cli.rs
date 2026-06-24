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
    BatchUpdate {
        /// Google Docs Document ID to update
        document_id: String,
        /// Path to a full documents.batchUpdate JSON request body, or - for stdin
        #[arg(long)]
        requests: String,
    },
}
