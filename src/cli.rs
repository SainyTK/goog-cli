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
    #[command(after_help = "Common nested commands:
  goog sheets values get-cell SPREADSHEET_ID RANGE
  goog sheets values get-row SPREADSHEET_ID RANGE
  goog sheets values get-column SPREADSHEET_ID RANGE
  goog sheets values get-table SPREADSHEET_ID RANGE
  goog sheets values update-cell SPREADSHEET_ID RANGE VALUE
  goog sheets values update-row SPREADSHEET_ID RANGE --value VALUE
  goog sheets values update-column SPREADSHEET_ID RANGE --value VALUE
  goog sheets values update-table SPREADSHEET_ID RANGE --data rows.csv
  goog sheets values append-row SPREADSHEET_ID RANGE --value VALUE
  goog sheets values append-column SPREADSHEET_ID RANGE --value VALUE
  goog sheets values append-table SPREADSHEET_ID RANGE --data rows.csv
  goog sheets sheet add SPREADSHEET_ID TITLE
  goog sheets sheet delete SPREADSHEET_ID SHEET_ID
  goog sheets sheet rename SPREADSHEET_ID SHEET_ID TITLE
  goog sheets sheet sort-range SPREADSHEET_ID SHEET_ID --range A1:D20 --sort-column 0
  goog sheets sheet delete-duplicates SPREADSHEET_ID SHEET_ID --range A1:D20
  goog sheets sheet trim-whitespace SPREADSHEET_ID SHEET_ID --range A1:D20
  goog sheets sheet protect-range SPREADSHEET_ID SHEET_ID --range A1:D20

More commands:
  goog sheets values --help
  goog sheets sheet --help")]
    Sheets {
        #[command(subcommand)]
        command: SheetsCommand,
    },
    /// Interact with Google Slides
    Slides {
        #[command(subcommand)]
        command: SlidesCommand,
    },
    /// Interact with Google Calendar
    Calendar {
        #[command(subcommand)]
        command: CalendarCommand,
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
        /// Maximum number of items to return. Without --all, defaults to 50
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

impl SlidesCommand {
    /// Resolves the `presentation_id` field of any variant to a bare
    /// Presentation ID, extracting it first if a Google Slides/Drive URL was
    /// passed instead.
    pub fn normalize_presentation_id(&mut self) {
        let presentation_id = match self {
            SlidesCommand::Create { .. } | SlidesCommand::List { .. } => return,
            SlidesCommand::Get {
                presentation_id, ..
            }
            | SlidesCommand::BatchUpdate {
                presentation_id, ..
            }
            | SlidesCommand::Slide {
                command:
                    SlidesSlideCommand::Create {
                        presentation_id, ..
                    }
                    | SlidesSlideCommand::Duplicate {
                        presentation_id, ..
                    }
                    | SlidesSlideCommand::Move {
                        presentation_id, ..
                    }
                    | SlidesSlideCommand::Background {
                        presentation_id, ..
                    }
                    | SlidesSlideCommand::Delete {
                        presentation_id, ..
                    },
            }
            | SlidesCommand::Object {
                command:
                    SlidesObjectCommand::Move {
                        presentation_id, ..
                    }
                    | SlidesObjectCommand::Order {
                        presentation_id, ..
                    }
                    | SlidesObjectCommand::Group {
                        presentation_id, ..
                    }
                    | SlidesObjectCommand::Ungroup {
                        presentation_id, ..
                    }
                    | SlidesObjectCommand::Style {
                        presentation_id, ..
                    }
                    | SlidesObjectCommand::LineStyle {
                        presentation_id, ..
                    }
                    | SlidesObjectCommand::TextStyle {
                        presentation_id, ..
                    }
                    | SlidesObjectCommand::InsertText {
                        presentation_id, ..
                    }
                    | SlidesObjectCommand::DeleteText {
                        presentation_id, ..
                    }
                    | SlidesObjectCommand::AltText {
                        presentation_id, ..
                    }
                    | SlidesObjectCommand::ReplaceImage {
                        presentation_id, ..
                    }
                    | SlidesObjectCommand::Delete {
                        presentation_id, ..
                    },
            }
            | SlidesCommand::TextBox {
                presentation_id, ..
            }
            | SlidesCommand::Image {
                presentation_id, ..
            }
            | SlidesCommand::Video {
                presentation_id, ..
            }
            | SlidesCommand::Table {
                presentation_id, ..
            }
            | SlidesCommand::TableFill {
                presentation_id, ..
            }
            | SlidesCommand::Shape {
                presentation_id, ..
            }
            | SlidesCommand::Line {
                presentation_id, ..
            }
            | SlidesCommand::ReplaceText {
                presentation_id, ..
            } => presentation_id,
        };
        *presentation_id = crate::slides::extract_presentation_id(presentation_id);
    }
}

#[derive(Debug, Subcommand)]
pub enum SlidesCommand {
    /// List native Google Slides presentations from Google Drive
    List {
        /// Maximum number of presentations to return (default: 50)
        #[arg(long)]
        limit: Option<u32>,
        /// Fetch all presentations across all pages
        #[arg(long)]
        all: bool,
        /// Drive folder ID to list presentations from
        #[arg(long)]
        folder: Option<String>,
        /// Emit newline-delimited JSON
        #[arg(long)]
        json: bool,
    },
    /// Create a new, blank Google Slides presentation
    Create {
        /// Title for the new Google Slides presentation
        title: String,
    },
    /// Read a Google Slides presentation
    Get {
        /// Presentation ID or URL to read
        presentation_id: String,
        /// Google partial response field selector
        #[arg(long)]
        fields: Option<String>,
        /// Emit raw JSON
        #[arg(long)]
        json: bool,
    },
    /// Apply a raw Google Slides Batch Update request body
    BatchUpdate {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Path to a full presentations.batchUpdate JSON request body, or - for stdin
        #[arg(long)]
        requests: String,
    },
    /// Create or manage slides inside a presentation
    Slide {
        #[command(subcommand)]
        command: SlidesSlideCommand,
    },
    /// Manage objects inside slides
    Object {
        #[command(subcommand)]
        command: SlidesObjectCommand,
    },
    /// Add a text box to a slide without writing Batch Update JSON
    TextBox {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Slide page object ID to place the text box on
        #[arg(long)]
        page_id: String,
        /// Text to insert into the text box
        #[arg(long)]
        text: String,
        /// Stable object ID for the new text box. Generated when omitted.
        #[arg(long)]
        object_id: Option<String>,
        /// Left offset in points
        #[arg(long, default_value_t = 72.0)]
        x: f64,
        /// Top offset in points
        #[arg(long, default_value_t = 72.0)]
        y: f64,
        /// Text box width in points
        #[arg(long, default_value_t = 360.0)]
        width: f64,
        /// Text box height in points
        #[arg(long, default_value_t = 120.0)]
        height: f64,
    },
    /// Add an image to a slide without writing Batch Update JSON
    Image {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Slide page object ID to place the image on
        #[arg(long)]
        page_id: String,
        /// Publicly reachable image URL
        #[arg(long)]
        url: String,
        /// Stable object ID for the new image. Generated when omitted.
        #[arg(long)]
        object_id: Option<String>,
        /// Left offset in points
        #[arg(long, default_value_t = 72.0)]
        x: f64,
        /// Top offset in points
        #[arg(long, default_value_t = 72.0)]
        y: f64,
        /// Image width in points
        #[arg(long, default_value_t = 360.0)]
        width: f64,
        /// Image height in points
        #[arg(long, default_value_t = 240.0)]
        height: f64,
    },
    /// Add a YouTube video to a slide without writing Batch Update JSON
    Video {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Slide page object ID to place the video on
        #[arg(long)]
        page_id: String,
        /// YouTube video ID, such as dQw4w9WgXcQ
        #[arg(long)]
        video_id: String,
        /// Stable object ID for the new video. Generated when omitted.
        #[arg(long)]
        object_id: Option<String>,
        /// Left offset in points
        #[arg(long, default_value_t = 72.0)]
        x: f64,
        /// Top offset in points
        #[arg(long, default_value_t = 72.0)]
        y: f64,
        /// Video width in points
        #[arg(long, default_value_t = 360.0)]
        width: f64,
        /// Video height in points
        #[arg(long, default_value_t = 240.0)]
        height: f64,
    },
    /// Add a table to a slide without writing Batch Update JSON
    Table {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Slide page object ID to place the table on
        #[arg(long)]
        page_id: String,
        /// Number of table rows
        #[arg(long)]
        rows: u32,
        /// Number of table columns
        #[arg(long)]
        columns: u32,
        /// Stable object ID for the new table. Generated when omitted.
        #[arg(long)]
        object_id: Option<String>,
        /// Left offset in points
        #[arg(long, default_value_t = 72.0)]
        x: f64,
        /// Top offset in points
        #[arg(long, default_value_t = 72.0)]
        y: f64,
        /// Table width in points
        #[arg(long, default_value_t = 360.0)]
        width: f64,
        /// Table height in points
        #[arg(long, default_value_t = 180.0)]
        height: f64,
    },
    /// Fill an existing table with row values without writing Batch Update JSON
    TableFill {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Table object ID to fill
        table_id: String,
        /// Table row values separated by --delimiter. Repeat once per row.
        #[arg(long = "row", required = true)]
        rows: Vec<String>,
        /// Delimiter used to split each --row value into cells
        #[arg(long, default_value = "|")]
        delimiter: String,
        /// Zero-based row index for the first provided row
        #[arg(long, default_value_t = 0)]
        start_row: u32,
        /// Zero-based column index for the first provided cell
        #[arg(long, default_value_t = 0)]
        start_column: u32,
    },
    /// Add a shape to a slide without writing Batch Update JSON
    Shape {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Slide page object ID to place the shape on
        #[arg(long)]
        page_id: String,
        /// Shape type to create
        #[arg(long = "type", value_enum)]
        shape_type: SlidesShapeType,
        /// Stable object ID for the new shape. Generated when omitted.
        #[arg(long)]
        object_id: Option<String>,
        /// Left offset in points
        #[arg(long, default_value_t = 72.0)]
        x: f64,
        /// Top offset in points
        #[arg(long, default_value_t = 72.0)]
        y: f64,
        /// Shape width in points
        #[arg(long, default_value_t = 180.0)]
        width: f64,
        /// Shape height in points
        #[arg(long, default_value_t = 120.0)]
        height: f64,
    },
    /// Add a line or connector to a slide without writing Batch Update JSON
    Line {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Slide page object ID to place the line on
        #[arg(long)]
        page_id: String,
        /// Line category to create
        #[arg(long, value_enum, default_value_t = SlidesLineCategory::Straight)]
        category: SlidesLineCategory,
        /// Stable object ID for the new line. Generated when omitted.
        #[arg(long)]
        object_id: Option<String>,
        /// Left offset in points
        #[arg(long, default_value_t = 72.0)]
        x: f64,
        /// Top offset in points
        #[arg(long, default_value_t = 72.0)]
        y: f64,
        /// Line bounding-box width in points
        #[arg(long, default_value_t = 240.0)]
        width: f64,
        /// Line bounding-box height in points
        #[arg(long, default_value_t = 0.0)]
        height: f64,
    },
    /// Replace text across a presentation or selected slides without writing Batch Update JSON
    ReplaceText {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Text to find
        #[arg(long)]
        find: String,
        /// Replacement text
        #[arg(long = "replace")]
        replacement: String,
        /// Match case when searching for text
        #[arg(long)]
        match_case: bool,
        /// Slide page object ID to limit replacement to. Repeat for multiple slides.
        #[arg(long = "page-id")]
        page_ids: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum SlidesSlideCommand {
    /// Create a slide without writing Batch Update JSON
    Create {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Stable object ID for the new slide. Google generates one when omitted.
        #[arg(long)]
        object_id: Option<String>,
        /// Zero-based insertion index. Appends when omitted.
        #[arg(long)]
        insertion_index: Option<u32>,
        /// Google Slides predefined layout to use
        #[arg(long, value_enum, default_value_t = SlidesPredefinedLayout::Blank)]
        layout: SlidesPredefinedLayout,
    },
    /// Duplicate a slide without writing Batch Update JSON
    Duplicate {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Slide page object ID to duplicate
        page_id: String,
        /// Stable object ID for the duplicated slide. Google generates one when omitted.
        #[arg(long)]
        object_id: Option<String>,
        /// Zero-based insertion index for the duplicated slide. Requires --object-id.
        #[arg(long, requires = "object_id")]
        insertion_index: Option<u32>,
    },
    /// Move one or more existing slides to a new position without writing Batch Update JSON
    Move {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Slide page object ID to move. Repeat to move multiple slides together.
        #[arg(long = "page-id", required = true)]
        page_ids: Vec<String>,
        /// Zero-based insertion index for the moved slide group
        #[arg(long)]
        insertion_index: u32,
    },
    /// Set a slide background color without writing Batch Update JSON
    Background {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Slide page object ID to update
        page_id: String,
        /// Background color as #RRGGBB or RRGGBB
        #[arg(long)]
        color: String,
    },
    /// Delete a slide without writing Batch Update JSON
    Delete {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Slide page object ID to delete
        page_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum SlidesObjectCommand {
    /// Move or scale a shape, image, table, or other page object without writing Batch Update JSON
    Move {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Page object ID to move or scale
        object_id: String,
        /// Left offset in points
        #[arg(long)]
        x: f64,
        /// Top offset in points
        #[arg(long)]
        y: f64,
        /// Horizontal scale to apply
        #[arg(long, default_value_t = 1.0)]
        scale_x: f64,
        /// Vertical scale to apply
        #[arg(long, default_value_t = 1.0)]
        scale_y: f64,
    },
    /// Bring objects forward or send them backward without writing Batch Update JSON
    Order {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Page object ID to arrange. Repeat to move multiple objects together.
        #[arg(long = "object-id", required = true)]
        object_ids: Vec<String>,
        /// Z-order operation to apply
        #[arg(long, value_enum)]
        operation: SlidesZOrderOperation,
    },
    /// Group two or more page objects without writing Batch Update JSON
    Group {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Page object ID to group. Repeat for each child object.
        #[arg(long = "object-id", required = true)]
        object_ids: Vec<String>,
        /// Stable object ID for the new group. Google generates one when omitted.
        #[arg(long)]
        group_id: Option<String>,
    },
    /// Ungroup one or more grouped page objects without writing Batch Update JSON
    Ungroup {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Group object ID to ungroup. Repeat to ungroup multiple groups on the same slide.
        #[arg(long = "object-id", required = true)]
        object_ids: Vec<String>,
    },
    /// Style a shape or text box fill and outline without writing Batch Update JSON
    Style {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Shape or text box page object ID to style
        object_id: String,
        /// Fill color as #RRGGBB or RRGGBB
        #[arg(long)]
        fill_color: Option<String>,
        /// Outline color as #RRGGBB or RRGGBB
        #[arg(long)]
        outline_color: Option<String>,
        /// Outline weight in points
        #[arg(long)]
        outline_weight: Option<f64>,
    },
    /// Style a line color and weight without writing Batch Update JSON
    LineStyle {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Line page object ID to style
        object_id: String,
        /// Line color as #RRGGBB or RRGGBB
        #[arg(long)]
        color: Option<String>,
        /// Line weight in points
        #[arg(long)]
        weight: Option<f64>,
    },
    /// Style text inside a shape or text box without writing Batch Update JSON
    TextStyle {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Shape or text box page object ID whose text should be styled
        object_id: String,
        /// Text color as #RRGGBB or RRGGBB
        #[arg(long)]
        color: Option<String>,
        /// Font family, such as Arial or Georgia
        #[arg(long)]
        font_family: Option<String>,
        /// Font size in points
        #[arg(long)]
        font_size: Option<f64>,
        /// Set or clear bold. Omit the value to set true.
        #[arg(long, num_args = 0..=1, default_missing_value = "true")]
        bold: Option<bool>,
        /// Set or clear italic. Omit the value to set true.
        #[arg(long, num_args = 0..=1, default_missing_value = "true")]
        italic: Option<bool>,
        /// Set or clear underline. Omit the value to set true.
        #[arg(long, num_args = 0..=1, default_missing_value = "true")]
        underline: Option<bool>,
        /// Zero-based start index for a fixed text range
        #[arg(long)]
        start_index: Option<u32>,
        /// Zero-based end index for a fixed text range
        #[arg(long)]
        end_index: Option<u32>,
    },
    /// Insert text into an existing shape or text box without writing Batch Update JSON
    InsertText {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Shape or text box page object ID to receive the text
        object_id: String,
        /// Text to insert
        #[arg(long)]
        text: String,
        /// Zero-based text insertion index
        #[arg(long, default_value_t = 0)]
        index: u32,
    },
    /// Delete text from an existing shape or text box without writing Batch Update JSON
    DeleteText {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Shape or text box page object ID whose text should be deleted
        object_id: String,
        /// Delete all text from the object
        #[arg(long)]
        all: bool,
        /// Zero-based start index for a fixed text range
        #[arg(long)]
        start_index: Option<u32>,
        /// Zero-based end index for a fixed text range
        #[arg(long)]
        end_index: Option<u32>,
    },
    /// Set accessibility alt text on a page object without writing Batch Update JSON
    AltText {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Page object ID whose alt text should be updated
        object_id: String,
        /// Human-readable alt text title
        #[arg(long)]
        title: Option<String>,
        /// Human-readable alt text description
        #[arg(long)]
        description: Option<String>,
    },
    /// Replace an existing image while preserving its size and position
    ReplaceImage {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Image page object ID to replace
        image_id: String,
        /// Publicly reachable replacement image URL
        #[arg(long)]
        url: String,
        /// How the replacement image should fit the existing image bounds
        #[arg(long, value_enum, default_value_t = SlidesImageReplaceMethod::CenterInside)]
        method: SlidesImageReplaceMethod,
    },
    /// Delete a shape, image, table, or other page object without writing Batch Update JSON
    Delete {
        /// Presentation ID or URL to update
        presentation_id: String,
        /// Page object ID to delete
        object_id: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SlidesImageReplaceMethod {
    CenterInside,
    CenterCrop,
}

impl SlidesImageReplaceMethod {
    pub fn as_api_value(self) -> &'static str {
        match self {
            SlidesImageReplaceMethod::CenterInside => "CENTER_INSIDE",
            SlidesImageReplaceMethod::CenterCrop => "CENTER_CROP",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SlidesShapeType {
    Rectangle,
    RoundRectangle,
    Ellipse,
    Arc,
    RightTriangle,
    Triangle,
    Diamond,
    Parallelogram,
    Trapezoid,
    Pentagon,
    Hexagon,
    Cloud,
    #[value(name = "star-5", alias = "star5")]
    Star5,
    Heart,
    Chevron,
    HomePlate,
    RightArrow,
    LeftArrow,
    UpArrow,
    DownArrow,
    Plus,
}

impl SlidesShapeType {
    pub fn api_value(self) -> &'static str {
        match self {
            SlidesShapeType::Rectangle => "RECTANGLE",
            SlidesShapeType::RoundRectangle => "ROUND_RECTANGLE",
            SlidesShapeType::Ellipse => "ELLIPSE",
            SlidesShapeType::Arc => "ARC",
            SlidesShapeType::RightTriangle => "RIGHT_TRIANGLE",
            SlidesShapeType::Triangle => "TRIANGLE",
            SlidesShapeType::Diamond => "DIAMOND",
            SlidesShapeType::Parallelogram => "PARALLELOGRAM",
            SlidesShapeType::Trapezoid => "TRAPEZOID",
            SlidesShapeType::Pentagon => "PENTAGON",
            SlidesShapeType::Hexagon => "HEXAGON",
            SlidesShapeType::Cloud => "CLOUD",
            SlidesShapeType::Star5 => "STAR_5",
            SlidesShapeType::Heart => "HEART",
            SlidesShapeType::Chevron => "CHEVRON",
            SlidesShapeType::HomePlate => "HOME_PLATE",
            SlidesShapeType::RightArrow => "RIGHT_ARROW",
            SlidesShapeType::LeftArrow => "LEFT_ARROW",
            SlidesShapeType::UpArrow => "UP_ARROW",
            SlidesShapeType::DownArrow => "DOWN_ARROW",
            SlidesShapeType::Plus => "PLUS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SlidesLineCategory {
    Straight,
    Bent,
    Curved,
}

impl SlidesLineCategory {
    pub fn api_value(self) -> &'static str {
        match self {
            SlidesLineCategory::Straight => "STRAIGHT",
            SlidesLineCategory::Bent => "BENT",
            SlidesLineCategory::Curved => "CURVED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SlidesZOrderOperation {
    BringToFront,
    BringForward,
    SendBackward,
    SendToBack,
}

impl SlidesZOrderOperation {
    pub fn api_value(self) -> &'static str {
        match self {
            SlidesZOrderOperation::BringToFront => "BRING_TO_FRONT",
            SlidesZOrderOperation::BringForward => "BRING_FORWARD",
            SlidesZOrderOperation::SendBackward => "SEND_BACKWARD",
            SlidesZOrderOperation::SendToBack => "SEND_TO_BACK",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SlidesPredefinedLayout {
    Blank,
    CaptionOnly,
    Title,
    TitleAndBody,
    TitleAndTwoColumns,
    TitleOnly,
    SectionHeader,
    SectionTitleAndDescription,
    OneColumnText,
    MainPoint,
    BigNumber,
}

impl SlidesPredefinedLayout {
    pub fn api_value(self) -> &'static str {
        match self {
            SlidesPredefinedLayout::Blank => "BLANK",
            SlidesPredefinedLayout::CaptionOnly => "CAPTION_ONLY",
            SlidesPredefinedLayout::Title => "TITLE",
            SlidesPredefinedLayout::TitleAndBody => "TITLE_AND_BODY",
            SlidesPredefinedLayout::TitleAndTwoColumns => "TITLE_AND_TWO_COLUMNS",
            SlidesPredefinedLayout::TitleOnly => "TITLE_ONLY",
            SlidesPredefinedLayout::SectionHeader => "SECTION_HEADER",
            SlidesPredefinedLayout::SectionTitleAndDescription => "SECTION_TITLE_AND_DESCRIPTION",
            SlidesPredefinedLayout::OneColumnText => "ONE_COLUMN_TEXT",
            SlidesPredefinedLayout::MainPoint => "MAIN_POINT",
            SlidesPredefinedLayout::BigNumber => "BIG_NUMBER",
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum CalendarCommand {
    /// List calendars or inspect calendar metadata
    Calendars {
        #[command(subcommand)]
        command: CalendarCalendarsCommand,
    },
    /// Inspect calendar sharing and access rules
    Acl {
        #[command(subcommand)]
        command: CalendarAclCommand,
    },
    /// Inspect available calendar and event color IDs
    Colors {
        #[command(subcommand)]
        command: CalendarColorsCommand,
    },
    /// List, create, import, update, move, quick-add, or delete Google Calendar events
    Events {
        #[command(subcommand)]
        command: CalendarEventsCommand,
    },
    /// Query free/busy windows across calendars
    Freebusy {
        /// Query interval start as RFC3339, such as 2026-07-09T09:00:00Z
        #[arg(long)]
        time_min: String,
        /// Query interval end as RFC3339, such as 2026-07-09T17:00:00Z
        #[arg(long)]
        time_max: String,
        /// Calendar or group ID to query. Repeat for multiple calendars.
        #[arg(long = "calendar", required = true)]
        calendars: Vec<String>,
        /// Time zone used in the response, such as Asia/Bangkok.
        #[arg(long)]
        time_zone: Option<String>,
        /// Maximum members to expand for a group.
        #[arg(long)]
        group_expansion_max: Option<u32>,
        /// Maximum calendars to expand.
        #[arg(long)]
        calendar_expansion_max: Option<u32>,
        /// Emit raw JSON response
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum CalendarColorsCommand {
    /// Read the available color palettes for calendars and events
    Get {
        /// Emit raw JSON response
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum CalendarAclCommand {
    /// Add an access control rule to a calendar
    Add {
        /// Calendar ID to share. Use primary for the account's primary calendar.
        calendar_id: String,
        /// Scope type for the rule.
        #[arg(long)]
        scope: CalendarAclScope,
        /// Scope value, such as a user email, group email, or domain. Omit for default scope.
        #[arg(long)]
        value: Option<String>,
        /// Access role to grant.
        #[arg(long)]
        role: CalendarAclRole,
        /// Suppress Google Calendar sharing notification emails.
        #[arg(long)]
        no_send_notifications: bool,
        /// Emit raw JSON response
        #[arg(long)]
        json: bool,
    },
    /// List access control rules for a calendar
    List {
        /// Calendar ID to inspect. Use primary for the account's primary calendar.
        calendar_id: String,
        /// Maximum number of ACL rules to return (default: 50)
        #[arg(long)]
        limit: Option<u32>,
        /// Fetch all ACL rules across all pages
        #[arg(long)]
        all: bool,
        /// Emit newline-delimited JSON
        #[arg(long)]
        json: bool,
    },
    /// Read one access control rule by rule ID
    Get {
        /// Calendar ID to inspect. Use primary for the account's primary calendar.
        calendar_id: String,
        /// ACL rule ID, such as user:teammate@example.com or default.
        rule_id: String,
        /// Emit raw JSON response
        #[arg(long)]
        json: bool,
    },
    /// Delete one access control rule
    Delete {
        /// Calendar ID containing the rule. Use primary for the account's primary calendar.
        calendar_id: String,
        /// ACL rule ID, such as user:teammate@example.com or default.
        rule_id: String,
    },
    /// Partially update one access control rule
    Patch {
        /// Calendar ID containing the rule. Use primary for the account's primary calendar.
        calendar_id: String,
        /// ACL rule ID, such as user:teammate@example.com or default.
        rule_id: String,
        /// Access role to set.
        #[arg(long)]
        role: CalendarAclRole,
        /// Emit raw JSON response
        #[arg(long)]
        json: bool,
    },
    /// Replace one access control rule
    Update {
        /// Calendar ID containing the rule. Use primary for the account's primary calendar.
        calendar_id: String,
        /// ACL rule ID, such as user:teammate@example.com or default.
        rule_id: String,
        /// Scope type for the replacement rule.
        #[arg(long)]
        scope: CalendarAclScope,
        /// Scope value, such as a user email, group email, or domain. Omit for default scope.
        #[arg(long)]
        value: Option<String>,
        /// Access role to set.
        #[arg(long)]
        role: CalendarAclRole,
        /// Emit raw JSON response
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CalendarAclScope {
    Default,
    User,
    Group,
    Domain,
}

impl CalendarAclScope {
    pub fn api_value(self) -> &'static str {
        match self {
            CalendarAclScope::Default => "default",
            CalendarAclScope::User => "user",
            CalendarAclScope::Group => "group",
            CalendarAclScope::Domain => "domain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CalendarAclRole {
    None,
    FreeBusyReader,
    Reader,
    Writer,
    Owner,
}

impl CalendarAclRole {
    pub fn api_value(self) -> &'static str {
        match self {
            CalendarAclRole::None => "none",
            CalendarAclRole::FreeBusyReader => "freeBusyReader",
            CalendarAclRole::Reader => "reader",
            CalendarAclRole::Writer => "writer",
            CalendarAclRole::Owner => "owner",
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum CalendarCalendarsCommand {
    /// List calendars visible in the account's calendar list
    List {
        /// Maximum number of calendars to return (default: 50)
        #[arg(long)]
        limit: Option<u32>,
        /// Fetch all calendars across all pages
        #[arg(long)]
        all: bool,
        /// Emit newline-delimited JSON
        #[arg(long)]
        json: bool,
    },
    /// Read one calendar's metadata
    Get {
        /// Calendar ID to read. Use primary for the account's primary calendar.
        calendar_id: String,
    },
    /// Create a secondary calendar
    Create {
        /// Calendar title
        #[arg(long)]
        summary: String,
        /// Calendar description
        #[arg(long)]
        description: Option<String>,
        /// Calendar location
        #[arg(long)]
        location: Option<String>,
        /// Calendar time zone, such as Asia/Bangkok
        #[arg(long)]
        time_zone: Option<String>,
    },
    /// Replace one calendar's editable metadata
    Update {
        /// Calendar ID to update. Primary calendars can be updated.
        calendar_id: String,
        /// Calendar title
        #[arg(long)]
        summary: String,
        /// Calendar description
        #[arg(long)]
        description: Option<String>,
        /// Calendar location
        #[arg(long)]
        location: Option<String>,
        /// Calendar time zone, such as Asia/Bangkok
        #[arg(long)]
        time_zone: Option<String>,
    },
    /// Patch one calendar's editable metadata
    Patch {
        /// Calendar ID to patch. Primary calendars can be patched.
        calendar_id: String,
        /// Calendar title
        #[arg(long)]
        summary: Option<String>,
        /// Calendar description
        #[arg(long)]
        description: Option<String>,
        /// Calendar location
        #[arg(long)]
        location: Option<String>,
        /// Calendar time zone, such as Asia/Bangkok
        #[arg(long)]
        time_zone: Option<String>,
    },
    /// Delete a secondary calendar
    Delete {
        /// Calendar ID to delete. Primary calendars cannot be deleted.
        calendar_id: String,
    },
    /// Manage the authenticated user's calendar list entry settings
    ListEntry {
        #[command(subcommand)]
        command: CalendarListEntryCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum CalendarListEntryCommand {
    /// Add an existing calendar to the authenticated user's calendar list
    Add {
        /// Calendar ID to add to the authenticated user's calendar list.
        calendar_id: String,
        /// Display name override for this calendar in the authenticated user's list.
        #[arg(long)]
        summary_override: Option<String>,
        /// Calendar color ID from `goog calendar colors get`.
        #[arg(long)]
        color_id: Option<String>,
        /// Hide this calendar in the authenticated user's calendar list.
        #[arg(long)]
        hidden: Option<bool>,
        /// Show this calendar's events in the Google Calendar UI.
        #[arg(long)]
        selected: Option<bool>,
        /// Default reminder as METHOD:MINUTES, where METHOD is popup or email. Repeat for multiple reminders.
        #[arg(long)]
        default_reminder: Vec<String>,
        /// Emit raw JSON response
        #[arg(long)]
        json: bool,
    },
    /// Read per-user settings for one calendar list entry
    Get {
        /// Calendar ID to read. Use primary for the account's primary calendar.
        calendar_id: String,
        /// Emit raw JSON response
        #[arg(long)]
        json: bool,
    },
    /// Replace per-user settings for one calendar list entry
    Update {
        /// Calendar ID to replace. Use primary for the account's primary calendar.
        calendar_id: String,
        /// Display name override for this calendar in the authenticated user's list.
        #[arg(long)]
        summary_override: Option<String>,
        /// Calendar color ID from `goog calendar colors get`.
        #[arg(long)]
        color_id: Option<String>,
        /// Hide or unhide this calendar in the authenticated user's calendar list.
        #[arg(long)]
        hidden: Option<bool>,
        /// Show or hide this calendar's events in the Google Calendar UI.
        #[arg(long)]
        selected: Option<bool>,
        /// Default reminder as METHOD:MINUTES, where METHOD is popup or email. Repeat for multiple reminders.
        #[arg(long)]
        default_reminder: Vec<String>,
        /// Clear default reminders for this calendar list entry.
        #[arg(long, conflicts_with = "default_reminder")]
        clear_default_reminders: bool,
        /// Emit raw JSON response
        #[arg(long)]
        json: bool,
    },
    /// Remove one calendar from the authenticated user's calendar list
    Delete {
        /// Calendar ID to remove from the authenticated user's calendar list.
        calendar_id: String,
    },
    /// Patch per-user settings for one calendar list entry
    Patch {
        /// Calendar ID to patch. Use primary for the account's primary calendar.
        calendar_id: String,
        /// Display name override for this calendar in the authenticated user's list.
        #[arg(long)]
        summary_override: Option<String>,
        /// Calendar color ID from `goog calendar colors get`.
        #[arg(long)]
        color_id: Option<String>,
        /// Hide or unhide this calendar in the authenticated user's calendar list.
        #[arg(long)]
        hidden: Option<bool>,
        /// Show or hide this calendar's events in the Google Calendar UI.
        #[arg(long)]
        selected: Option<bool>,
        /// Default reminder as METHOD:MINUTES, where METHOD is popup or email. Repeat for multiple reminders.
        #[arg(long)]
        default_reminder: Vec<String>,
        /// Clear default reminders for this calendar list entry.
        #[arg(long, conflicts_with = "default_reminder")]
        clear_default_reminders: bool,
        /// Emit raw JSON response
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum CalendarEventsCommand {
    /// List events on a calendar
    List {
        /// Calendar ID to read. Use primary for the account's primary calendar.
        calendar_id: String,
        /// Maximum number of events to return (default: 50)
        #[arg(long)]
        limit: Option<u32>,
        /// Fetch all events across all pages
        #[arg(long)]
        all: bool,
        /// Lower bound for event start time as RFC3339, such as 2026-07-09T09:00:00Z
        #[arg(long)]
        time_min: Option<String>,
        /// Upper bound for event start time as RFC3339, such as 2026-07-10T09:00:00Z
        #[arg(long)]
        time_max: Option<String>,
        /// Time zone used for returned event times, such as Asia/Bangkok
        #[arg(long)]
        time_zone: Option<String>,
        /// Free-text search query
        #[arg(long)]
        query: Option<String>,
        /// Lower bound for event last modification time as RFC3339
        #[arg(long)]
        updated_min: Option<String>,
        /// Filter events by iCalendar UID
        #[arg(long, alias = "ical-uid")]
        i_cal_uid: Option<String>,
        /// Filter by private extended property as NAME=VALUE. Repeat for multiple filters.
        #[arg(long)]
        private_extended_property: Vec<String>,
        /// Filter by shared extended property as NAME=VALUE. Repeat for multiple filters.
        #[arg(long)]
        shared_extended_property: Vec<String>,
        /// Filter by event type. Repeat for multiple types.
        #[arg(long, value_enum)]
        event_type: Vec<CalendarEventType>,
        /// Expand recurring events into instances
        #[arg(long)]
        single_events: bool,
        /// Include deleted events with cancelled status
        #[arg(long)]
        show_deleted: bool,
        /// Include hidden invitations
        #[arg(long)]
        show_hidden_invitations: bool,
        /// Order events by start time or last update time
        #[arg(long, value_enum)]
        order_by: Option<CalendarEventsOrderBy>,
        /// Emit newline-delimited JSON
        #[arg(long)]
        json: bool,
    },
    /// Read one event from a calendar
    Get {
        /// Calendar ID to read. Use primary for the account's primary calendar.
        calendar_id: String,
        /// Event ID to read
        event_id: String,
        /// Emit raw JSON response
        #[arg(long)]
        json: bool,
    },
    /// List generated instances for a recurring event
    Instances {
        /// Calendar ID to read. Use primary for the account's primary calendar.
        calendar_id: String,
        /// Recurring event ID to expand
        event_id: String,
        /// Maximum number of instances to return (default: 50)
        #[arg(long)]
        limit: Option<u32>,
        /// Fetch all instances across all pages
        #[arg(long)]
        all: bool,
        /// Lower bound for instance start time as RFC3339, such as 2026-07-09T09:00:00Z
        #[arg(long)]
        time_min: Option<String>,
        /// Upper bound for instance start time as RFC3339, such as 2026-07-10T09:00:00Z
        #[arg(long)]
        time_max: Option<String>,
        /// Emit newline-delimited JSON
        #[arg(long)]
        json: bool,
    },
    /// Create an event from flags or an Events resource JSON body
    Create {
        /// Calendar ID to update. Use primary for the account's primary calendar.
        calendar_id: String,
        /// Path to an Event JSON request body, or - for stdin
        #[arg(long)]
        event: Option<String>,
        /// Event summary. Required unless --event is used.
        #[arg(long, required_unless_present = "event", conflicts_with = "event")]
        summary: Option<String>,
        /// Event start as RFC3339 date-time, or YYYY-MM-DD with --all-day.
        #[arg(long, required_unless_present = "event", conflicts_with = "event")]
        start: Option<String>,
        /// Event end as RFC3339 date-time, or YYYY-MM-DD with --all-day.
        #[arg(long, required_unless_present = "event", conflicts_with = "event")]
        end: Option<String>,
        /// IANA time zone for date-time events, such as Asia/Bangkok.
        #[arg(long, conflicts_with = "event")]
        time_zone: Option<String>,
        /// Treat --start and --end as all-day dates.
        #[arg(long, conflicts_with = "event")]
        all_day: bool,
        /// Event location.
        #[arg(long, conflicts_with = "event")]
        location: Option<String>,
        /// Event description.
        #[arg(long, conflicts_with = "event")]
        description: Option<String>,
        /// Event color ID from `goog calendar colors get`.
        #[arg(long, conflicts_with = "event")]
        color_id: Option<String>,
        /// Attendee email address. Repeat for multiple attendees.
        #[arg(long, conflicts_with = "event")]
        attendee: Vec<String>,
        /// Recurrence rule or date entry, such as RRULE:FREQ=WEEKLY;COUNT=4. Repeat for multiple entries.
        #[arg(long, conflicts_with = "event")]
        recurrence: Vec<String>,
        /// Reminder override as METHOD:MINUTES, where METHOD is popup or email. Repeat for multiple reminders.
        #[arg(long, conflicts_with = "event", conflicts_with = "no_reminders")]
        reminder: Vec<String>,
        /// Disable default reminders for this event.
        #[arg(long, conflicts_with = "event")]
        no_reminders: bool,
        /// Guests who should receive creation notifications: all, external-only, or none.
        #[arg(long, value_enum)]
        send_updates: Option<CalendarSendUpdates>,
    },
    /// Import a private event copy from flags or an Events resource JSON body
    Import {
        /// Calendar ID to update. Use primary for the account's primary calendar.
        calendar_id: String,
        /// Path to an Event JSON request body, or - for stdin
        #[arg(long)]
        event: Option<String>,
        /// Event summary. Required unless --event is used.
        #[arg(long, required_unless_present = "event", conflicts_with = "event")]
        summary: Option<String>,
        /// Event start as RFC3339 date-time, or YYYY-MM-DD with --all-day.
        #[arg(long, required_unless_present = "event", conflicts_with = "event")]
        start: Option<String>,
        /// Event end as RFC3339 date-time, or YYYY-MM-DD with --all-day.
        #[arg(long, required_unless_present = "event", conflicts_with = "event")]
        end: Option<String>,
        /// IANA time zone for date-time events, such as Asia/Bangkok.
        #[arg(long, conflicts_with = "event")]
        time_zone: Option<String>,
        /// Treat --start and --end as all-day dates.
        #[arg(long, conflicts_with = "event")]
        all_day: bool,
        /// Event location.
        #[arg(long, conflicts_with = "event")]
        location: Option<String>,
        /// Event description.
        #[arg(long, conflicts_with = "event")]
        description: Option<String>,
        /// Event color ID from `goog calendar colors get`.
        #[arg(long, conflicts_with = "event")]
        color_id: Option<String>,
        /// Attendee email address. Repeat for multiple attendees.
        #[arg(long, conflicts_with = "event")]
        attendee: Vec<String>,
        /// Recurrence rule or date entry, such as RRULE:FREQ=WEEKLY;COUNT=4. Repeat for multiple entries.
        #[arg(long, conflicts_with = "event")]
        recurrence: Vec<String>,
        /// Reminder override as METHOD:MINUTES, where METHOD is popup or email. Repeat for multiple reminders.
        #[arg(long, conflicts_with = "event", conflicts_with = "no_reminders")]
        reminder: Vec<String>,
        /// Disable default reminders for this event.
        #[arg(long, conflicts_with = "event")]
        no_reminders: bool,
    },
    /// Replace an event from flags or an Events resource JSON body
    Update {
        /// Calendar ID to update. Use primary for the account's primary calendar.
        calendar_id: String,
        /// Event ID to update
        event_id: String,
        /// Path to an Event JSON request body, or - for stdin
        #[arg(long)]
        event: Option<String>,
        /// Event summary. Required unless --event is used.
        #[arg(long, required_unless_present = "event", conflicts_with = "event")]
        summary: Option<String>,
        /// Event start as RFC3339 date-time, or YYYY-MM-DD with --all-day.
        #[arg(long, required_unless_present = "event", conflicts_with = "event")]
        start: Option<String>,
        /// Event end as RFC3339 date-time, or YYYY-MM-DD with --all-day.
        #[arg(long, required_unless_present = "event", conflicts_with = "event")]
        end: Option<String>,
        /// IANA time zone for date-time events, such as Asia/Bangkok.
        #[arg(long, conflicts_with = "event")]
        time_zone: Option<String>,
        /// Treat --start and --end as all-day dates.
        #[arg(long, conflicts_with = "event")]
        all_day: bool,
        /// Event location.
        #[arg(long, conflicts_with = "event")]
        location: Option<String>,
        /// Event description.
        #[arg(long, conflicts_with = "event")]
        description: Option<String>,
        /// Event color ID from `goog calendar colors get`.
        #[arg(long, conflicts_with = "event")]
        color_id: Option<String>,
        /// Attendee email address. Repeat for multiple attendees.
        #[arg(long, conflicts_with = "event")]
        attendee: Vec<String>,
        /// Recurrence rule or date entry, such as RRULE:FREQ=WEEKLY;COUNT=4. Repeat for multiple entries.
        #[arg(long, conflicts_with = "event")]
        recurrence: Vec<String>,
        /// Reminder override as METHOD:MINUTES, where METHOD is popup or email. Repeat for multiple reminders.
        #[arg(long, conflicts_with = "event", conflicts_with = "no_reminders")]
        reminder: Vec<String>,
        /// Disable default reminders for this event.
        #[arg(long, conflicts_with = "event")]
        no_reminders: bool,
        /// Guests who should receive update notifications: all, external-only, or none.
        #[arg(long, value_enum)]
        send_updates: Option<CalendarSendUpdates>,
    },
    /// Partially update an event from flags or an Events resource JSON body
    Patch {
        /// Calendar ID to update. Use primary for the account's primary calendar.
        calendar_id: String,
        /// Event ID to patch
        event_id: String,
        /// Path to an Event JSON request body, or - for stdin
        #[arg(long)]
        event: Option<String>,
        /// Event summary.
        #[arg(long, conflicts_with = "event")]
        summary: Option<String>,
        /// Event start as RFC3339 date-time, or YYYY-MM-DD with --all-day.
        #[arg(long, conflicts_with = "event")]
        start: Option<String>,
        /// Event end as RFC3339 date-time, or YYYY-MM-DD with --all-day.
        #[arg(long, conflicts_with = "event")]
        end: Option<String>,
        /// IANA time zone for patched date-time fields, such as Asia/Bangkok.
        #[arg(long, conflicts_with = "event")]
        time_zone: Option<String>,
        /// Treat patched --start and --end values as all-day dates.
        #[arg(long, conflicts_with = "event")]
        all_day: bool,
        /// Event location.
        #[arg(long, conflicts_with = "event")]
        location: Option<String>,
        /// Event description.
        #[arg(long, conflicts_with = "event")]
        description: Option<String>,
        /// Event color ID from `goog calendar colors get`.
        #[arg(long, conflicts_with = "event")]
        color_id: Option<String>,
        /// Attendee email address. Repeat for multiple attendees.
        #[arg(long, conflicts_with = "event")]
        attendee: Vec<String>,
        /// Recurrence rule or date entry, such as RRULE:FREQ=WEEKLY;COUNT=4. Repeat for multiple entries.
        #[arg(long, conflicts_with = "event")]
        recurrence: Vec<String>,
        /// Reminder override as METHOD:MINUTES, where METHOD is popup or email. Repeat for multiple reminders.
        #[arg(long, conflicts_with = "event", conflicts_with = "no_reminders")]
        reminder: Vec<String>,
        /// Disable default reminders for this event.
        #[arg(long, conflicts_with = "event")]
        no_reminders: bool,
        /// Guests who should receive update notifications: all, external-only, or none.
        #[arg(long, value_enum)]
        send_updates: Option<CalendarSendUpdates>,
    },
    /// Move an event from one calendar to another
    Move {
        /// Source calendar ID. Use primary for the account's primary calendar.
        source_calendar_id: String,
        /// Event ID to move
        event_id: String,
        /// Destination calendar ID
        #[arg(long)]
        destination: String,
    },
    /// Create an event from natural language text
    QuickAdd {
        /// Calendar ID to update. Use primary for the account's primary calendar.
        calendar_id: String,
        /// Natural language event text, such as "Lunch with Sam tomorrow at noon"
        text: String,
        /// Guests who should receive creation notifications: all, external-only, or none.
        #[arg(long, value_enum)]
        send_updates: Option<CalendarSendUpdates>,
    },
    /// Delete an event from a calendar
    Delete {
        /// Calendar ID to update. Use primary for the account's primary calendar.
        calendar_id: String,
        /// Event ID to delete
        event_id: String,
        /// Guests who should receive deletion notifications: all, external-only, or none.
        #[arg(long, value_enum)]
        send_updates: Option<CalendarSendUpdates>,
    },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum CalendarSendUpdates {
    All,
    ExternalOnly,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum CalendarEventsOrderBy {
    StartTime,
    Updated,
}

impl CalendarEventsOrderBy {
    pub fn api_value(self) -> &'static str {
        match self {
            CalendarEventsOrderBy::StartTime => "startTime",
            CalendarEventsOrderBy::Updated => "updated",
        }
    }

    pub fn from_api_value(value: &str) -> Option<Self> {
        match value {
            "startTime" => Some(CalendarEventsOrderBy::StartTime),
            "updated" => Some(CalendarEventsOrderBy::Updated),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum CalendarEventType {
    Birthday,
    Default,
    FocusTime,
    FromGmail,
    OutOfOffice,
    WorkingLocation,
}

impl CalendarEventType {
    pub fn api_value(self) -> &'static str {
        match self {
            CalendarEventType::Birthday => "birthday",
            CalendarEventType::Default => "default",
            CalendarEventType::FocusTime => "focusTime",
            CalendarEventType::FromGmail => "fromGmail",
            CalendarEventType::OutOfOffice => "outOfOffice",
            CalendarEventType::WorkingLocation => "workingLocation",
        }
    }
}

impl DocsCommand {
    /// Resolves the `document_id` field of any variant to a bare Document ID,
    /// extracting it first if a Google Docs/Drive URL was passed instead.
    pub fn normalize_document_id(&mut self) {
        let document_id = match self {
            DocsCommand::Create { .. } | DocsCommand::List { .. } => return,
            DocsCommand::Map { document_id, .. }
            | DocsCommand::Get { document_id, .. }
            | DocsCommand::BatchUpdate { document_id, .. } => document_id,
            DocsCommand::ListFormat { command } => match command {
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
    /// List native Google Docs Documents from Google Drive
    List {
        /// Maximum number of Documents to return (default: 50)
        #[arg(long)]
        limit: Option<u32>,
        /// Fetch all Documents across all pages
        #[arg(long)]
        all: bool,
        /// Drive folder ID to list Documents from
        #[arg(long)]
        folder: Option<String>,
        /// Emit newline-delimited JSON
        #[arg(long)]
        json: bool,
    },
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
    ListFormat {
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
    /// Return values formatted as displayed in the sheet
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
pub enum SheetsTableOutputFormat {
    /// Print tab-separated rows
    Tsv,
    /// Print comma-separated rows with CSV quoting
    Csv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsTableInputFormat {
    /// Infer CSV or TSV from the data file extension
    Auto,
    /// Read tab-separated rows
    Tsv,
    /// Read comma-separated rows
    Csv,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SheetsConditionalFormatCondition {
    /// Cell number is greater than the provided value
    NumberGreater,
    /// Cell number is less than the provided value
    NumberLess,
    /// Cell value is equal to the provided value
    Equal,
    /// Cell value is not equal to the provided value
    NotEqual,
    /// Cell text contains the provided value
    TextContains,
    /// Cell text is exactly the provided value
    TextEq,
    /// Use the provided value as a custom formula
    CustomFormula,
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
    /// List native Google Sheets Spreadsheets from Google Drive
    List {
        /// Maximum number of Spreadsheets to return (default: 50)
        #[arg(long)]
        limit: Option<u32>,
        /// Fetch all Spreadsheets across all pages
        #[arg(long)]
        all: bool,
        /// Drive folder ID to list Spreadsheets from
        #[arg(long)]
        folder: Option<String>,
        /// Emit newline-delimited JSON
        #[arg(long)]
        json: bool,
    },
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
    /// Read and write sheet values
    Values {
        #[command(subcommand)]
        command: SheetsValuesCommand,
    },
    /// Manage individual sheets inside a Spreadsheet
    Sheet {
        #[command(subcommand)]
        command: SheetsSheetCommand,
    },
    /// Apply a raw structural spreadsheet update request body
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
    /// Hide rows or columns without writing a Batch Update JSON body
    HideDimension {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Dimension to hide
        #[arg(long)]
        dimension: SheetsDimension,
        /// Zero-based inclusive start index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_index: i64,
        /// Zero-based exclusive end index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_index: i64,
    },
    /// Unhide rows or columns without writing a Batch Update JSON body
    UnhideDimension {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Dimension to unhide
        #[arg(long)]
        dimension: SheetsDimension,
        /// Zero-based inclusive start index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_index: i64,
        /// Zero-based exclusive end index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_index: i64,
    },
    /// Group rows or columns without writing a Batch Update JSON body
    GroupDimension {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Dimension to group
        #[arg(long)]
        dimension: SheetsDimension,
        /// Zero-based inclusive start index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_index: i64,
        /// Zero-based exclusive end index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_index: i64,
    },
    /// Ungroup rows or columns without writing a Batch Update JSON body
    UngroupDimension {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Dimension to ungroup
        #[arg(long)]
        dimension: SheetsDimension,
        /// Zero-based inclusive start index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_index: i64,
        /// Zero-based exclusive end index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_index: i64,
    },
    /// Collapse a row or column group without writing a Batch Update JSON body
    CollapseDimensionGroup {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Dimension group to collapse
        #[arg(long)]
        dimension: SheetsDimension,
        /// Zero-based inclusive start index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_index: i64,
        /// Zero-based exclusive end index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_index: i64,
    },
    /// Expand a row or column group without writing a Batch Update JSON body
    ExpandDimensionGroup {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Dimension group to expand
        #[arg(long)]
        dimension: SheetsDimension,
        /// Zero-based inclusive start index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_index: i64,
        /// Zero-based exclusive end index
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_index: i64,
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
    /// Remove duplicate rows over a grid range without writing a Batch Update JSON body
    DeleteDuplicates {
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
        /// Zero-based sheet column to compare for duplicates. Repeat for multiple columns.
        #[arg(long = "comparison-column", value_parser = clap::value_parser!(i64).range(0..))]
        comparison_columns: Vec<i64>,
    },
    /// Trim whitespace in every cell over a grid range without writing a Batch Update JSON body
    TrimWhitespace {
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
    /// Randomize rows over a grid range without writing a Batch Update JSON body
    RandomizeRange {
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
    /// Set or clear checkbox data validation over a range without writing a Batch Update JSON body
    DataValidationCheckbox {
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
        /// Optional custom value for checked cells
        #[arg(long, conflicts_with = "clear")]
        checked_value: Option<String>,
        /// Optional custom value for unchecked cells
        #[arg(long, requires = "checked_value", conflicts_with = "clear")]
        unchecked_value: Option<String>,
        /// Show a warning instead of rejecting invalid values
        #[arg(long, conflicts_with = "clear")]
        allow_invalid: bool,
        /// Optional validation help text shown in Google Sheets
        #[arg(long, conflicts_with = "clear")]
        input_message: Option<String>,
        /// Clear data validation from the selected cells instead of setting checkboxes
        #[arg(long)]
        clear: bool,
    },
    /// Add a single-color conditional format rule without writing a Batch Update JSON body
    ConditionalFormatColor {
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
        /// Boolean condition for the rule
        #[arg(long, value_enum)]
        condition: SheetsConditionalFormatCondition,
        /// User-entered comparison value or custom formula
        #[arg(long)]
        value: String,
        /// Optional hex background color to apply, as RRGGBB or #RRGGBB
        #[arg(long)]
        background_color: Option<String>,
        /// Optional hex text color to apply, as RRGGBB or #RRGGBB
        #[arg(long)]
        text_color: Option<String>,
        /// Conditional formatting rule insertion index
        #[arg(long, default_value = "0", value_parser = clap::value_parser!(i64).range(0..))]
        index: i64,
    },
    /// Replace a single-color conditional format rule without writing a Batch Update JSON body
    ConditionalFormatUpdate {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based conditional format rule index to replace
        #[arg(value_parser = clap::value_parser!(i64).range(0..))]
        index: i64,
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
        /// Boolean condition for the replacement rule
        #[arg(long, value_enum)]
        condition: SheetsConditionalFormatCondition,
        /// User-entered comparison value or custom formula
        #[arg(long)]
        value: String,
        /// Optional hex background color to apply, as RRGGBB or #RRGGBB
        #[arg(long)]
        background_color: Option<String>,
        /// Optional hex text color to apply, as RRGGBB or #RRGGBB
        #[arg(long)]
        text_color: Option<String>,
    },
    /// Delete a conditional format rule by index without writing a Batch Update JSON body
    ConditionalFormatDelete {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based conditional format rule index to delete
        #[arg(value_parser = clap::value_parser!(i64).range(0..))]
        index: i64,
    },
    /// Move a conditional format rule to another index without writing a Batch Update JSON body
    ConditionalFormatMove {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the tab to update
        sheet_id: i64,
        /// Zero-based conditional format rule index to move
        #[arg(value_parser = clap::value_parser!(i64).range(0..))]
        index: i64,
        /// Zero-based destination index for the rule
        #[arg(value_parser = clap::value_parser!(i64).range(0..))]
        new_index: i64,
    },
    /// Protect a cell range without writing a Batch Update JSON body
    ProtectRange {
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
        /// Optional protected range description shown in Google Sheets
        #[arg(long)]
        description: Option<String>,
        /// Warn users before edits instead of blocking edits
        #[arg(long)]
        warning_only: bool,
    },
    /// Add a named range without writing a Batch Update JSON body
    AddNamedRange {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets numeric sheetId for the named range
        sheet_id: i64,
        /// Named range name to create
        name: String,
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
        /// Optional Google Sheets namedRangeId for the new named range
        #[arg(long)]
        named_range_id: Option<String>,
    },
    /// Delete a named range without writing a Batch Update JSON body
    DeleteNamedRange {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets namedRangeId to delete
        named_range_id: String,
    },
    /// Update a named range without writing a Batch Update JSON body
    UpdateNamedRange {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets namedRangeId to update
        named_range_id: String,
        /// New named range name
        #[arg(long)]
        name: Option<String>,
        /// Google Sheets numeric sheetId for the new range
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        sheet_id: Option<i64>,
        /// Zero-based inclusive start row for the new range
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_row: Option<i64>,
        /// Zero-based exclusive end row for the new range
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_row: Option<i64>,
        /// Zero-based inclusive start column for the new range
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        start_column: Option<i64>,
        /// Zero-based exclusive end column for the new range
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
        end_column: Option<i64>,
    },
    /// Update a protected range without writing a Batch Update JSON body
    UpdateProtectedRange {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets protectedRangeId to update
        #[arg(value_parser = clap::value_parser!(i64).range(0..))]
        protected_range_id: i64,
        /// New protected range description
        #[arg(long, required_unless_present_any = ["warning_only", "enforce"])]
        description: Option<String>,
        /// Warn users before edits instead of blocking edits
        #[arg(long, conflicts_with = "enforce")]
        warning_only: bool,
        /// Block edits instead of showing warnings only
        #[arg(long, conflicts_with = "warning_only")]
        enforce: bool,
    },
    /// Remove a protected range without writing a Batch Update JSON body
    UnprotectRange {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets protectedRangeId to delete
        #[arg(value_parser = clap::value_parser!(i64).range(0..))]
        protected_range_id: i64,
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
    /// Fetch one Google Sheets cell as a scalar line
    GetCell {
        /// Google Sheets Spreadsheet ID to fetch
        spreadsheet_id: String,
        /// Google Sheets A1 cell Range to fetch
        range: String,
        /// How the value should be represented in the response
        #[arg(long, value_enum, default_value = "formatted-value")]
        value_render_option: SheetsValueRenderOption,
    },
    /// Fetch one Google Sheets row as a tab-separated line
    GetRow {
        /// Google Sheets Spreadsheet ID to fetch
        spreadsheet_id: String,
        /// Google Sheets A1 row Range to fetch
        range: String,
        /// How values should be represented in the response
        #[arg(long, value_enum, default_value = "formatted-value")]
        value_render_option: SheetsValueRenderOption,
    },
    /// Fetch one Google Sheets column as one scalar value per line
    GetColumn {
        /// Google Sheets Spreadsheet ID to fetch
        spreadsheet_id: String,
        /// Google Sheets A1 column Range to fetch
        range: String,
        /// How values should be represented in the response
        #[arg(long, value_enum, default_value = "formatted-value")]
        value_render_option: SheetsValueRenderOption,
    },
    /// Fetch a Google Sheets range as TSV or CSV rows
    GetTable {
        /// Google Sheets Spreadsheet ID to fetch
        spreadsheet_id: String,
        /// Google Sheets A1 Range to fetch
        range: String,
        /// How values should be represented in the response
        #[arg(long, value_enum, default_value = "formatted-value")]
        value_render_option: SheetsValueRenderOption,
        /// Table output format
        #[arg(long, value_enum, default_value = "tsv")]
        format: SheetsTableOutputFormat,
    },
    /// Update sheet values
    Update {
        /// Spreadsheet ID to update
        spreadsheet_id: String,
        /// A1 range to update. Omit to pass a full spreadsheets.values.batchUpdate body.
        range: Option<String>,
        /// Path to a ValueRange JSON request body, or - for stdin
        #[arg(long)]
        values: String,
        /// How input values should be interpreted when RANGE is provided
        #[arg(long, value_enum, default_value = "user-entered")]
        value_input_option: SheetsValueInputOption,
    },
    /// Update a Google Sheets Range from CSV or TSV without writing a ValueRange JSON body
    UpdateTable {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets A1 Range to update
        range: String,
        /// CSV or TSV data file to write, or - for stdin
        #[arg(long)]
        data: String,
        /// Table input format
        #[arg(long, value_enum, default_value = "auto")]
        format: SheetsTableInputFormat,
        /// How input values should be interpreted
        #[arg(long, value_enum, default_value = "user-entered")]
        value_input_option: SheetsValueInputOption,
    },
    /// Update one cell without writing a ValueRange JSON body
    UpdateCell {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets A1 cell Range to update
        range: String,
        /// Cell value to write
        value: String,
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
    /// Update one column without writing a ValueRange JSON body
    UpdateColumn {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets A1 Range to update
        range: String,
        /// Cell value to write. Repeat once per row.
        #[arg(long = "value", required = true)]
        values: Vec<String>,
        /// How input values should be interpreted
        #[arg(long, value_enum, default_value = "user-entered")]
        value_input_option: SheetsValueInputOption,
    },
    /// Append values to a range
    Append {
        /// Spreadsheet ID to update
        spreadsheet_id: String,
        /// A1 range to append into
        range: String,
        /// Path to a ValueRange JSON request body, or - for stdin
        #[arg(long)]
        values: String,
        /// How input values should be interpreted
        #[arg(long, value_enum, default_value = "user-entered")]
        value_input_option: SheetsValueInputOption,
        /// How appended data should be inserted
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
    /// Append one column without writing a ValueRange JSON body
    AppendColumn {
        /// Google Sheets Spreadsheet ID to update
        spreadsheet_id: String,
        /// Google Sheets A1 Range to append into
        range: String,
        /// Cell value to append. Repeat once per row.
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
        /// CSV or TSV data file to append, or - for stdin
        #[arg(long)]
        data: String,
        /// Table input format
        #[arg(long, value_enum, default_value = "auto")]
        format: SheetsTableInputFormat,
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
