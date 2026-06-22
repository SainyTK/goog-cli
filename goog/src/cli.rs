use clap::{Parser, Subcommand};

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
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Import OAuth App credentials and save them to config
    Setup {
        /// Path to the client_secret_*.json file downloaded from GCP Console
        #[arg(long)]
        credentials: Option<String>,
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
        /// Email address of the account to activate
        email: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum DriveCommand {
    /// List files in Google Drive
    List {
        /// Maximum number of files to return (default: 50)
        #[arg(long)]
        limit: Option<u32>,
        /// Fetch all files across all pages
        #[arg(long)]
        all: bool,
        /// Emit newline-delimited JSON
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("goog").chain(args.iter().copied()))
    }

    // --- auth setup ---

    #[test]
    fn auth_setup_no_flags() {
        let cli = parse(&["auth", "setup"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Auth {
                command: AuthCommand::Setup { credentials: None }
            }
        ));
    }

    #[test]
    fn auth_setup_with_credentials_flag() {
        let cli = parse(&["auth", "setup", "--credentials", "/tmp/client_secret.json"]).unwrap();
        match cli.command {
            Command::Auth {
                command: AuthCommand::Setup {
                    credentials: Some(path),
                },
            } => assert_eq!(path, "/tmp/client_secret.json"),
            _ => panic!("unexpected parse result"),
        }
    }

    // --- auth login ---

    #[test]
    fn auth_login_default() {
        let cli = parse(&["auth", "login"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Auth {
                command: AuthCommand::Login { no_browser: false }
            }
        ));
    }

    #[test]
    fn auth_login_no_browser() {
        let cli = parse(&["auth", "login", "--no-browser"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Auth {
                command: AuthCommand::Login { no_browser: true }
            }
        ));
    }

    // --- auth list ---

    #[test]
    fn auth_list_default() {
        let cli = parse(&["auth", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Auth {
                command: AuthCommand::List { json: false }
            }
        ));
    }

    #[test]
    fn auth_list_json() {
        let cli = parse(&["auth", "list", "--json"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Auth {
                command: AuthCommand::List { json: true }
            }
        ));
    }

    // --- auth switch ---

    #[test]
    fn auth_switch_with_email() {
        let cli = parse(&["auth", "switch", "user@example.com"]).unwrap();
        match cli.command {
            Command::Auth {
                command: AuthCommand::Switch { email },
            } => assert_eq!(email, "user@example.com"),
            _ => panic!("unexpected parse result"),
        }
    }

    #[test]
    fn auth_switch_requires_email() {
        assert!(parse(&["auth", "switch"]).is_err());
    }

    // --- drive list ---

    #[test]
    fn drive_list_defaults() {
        let cli = parse(&["drive", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Drive {
                command: DriveCommand::List {
                    limit: None,
                    all: false,
                    json: false
                }
            }
        ));
    }

    #[test]
    fn drive_list_with_flags() {
        let cli = parse(&["drive", "list", "--limit", "100", "--all", "--json"]).unwrap();
        match cli.command {
            Command::Drive {
                command: DriveCommand::List { limit, all, json },
            } => {
                assert_eq!(limit, Some(100));
                assert!(all);
                assert!(json);
            }
            _ => panic!("unexpected parse result"),
        }
    }

    // --- drive download ---

    #[test]
    fn drive_download_file_id_only() {
        let cli = parse(&["drive", "download", "file123"]).unwrap();
        match cli.command {
            Command::Drive {
                command: DriveCommand::Download { file_id, output },
            } => {
                assert_eq!(file_id, "file123");
                assert!(output.is_none());
            }
            _ => panic!("unexpected parse result"),
        }
    }

    #[test]
    fn drive_download_with_output() {
        let cli = parse(&["drive", "download", "file123", "--output", "/tmp/out.pdf"]).unwrap();
        match cli.command {
            Command::Drive {
                command: DriveCommand::Download { output, .. },
            } => assert_eq!(output.as_deref(), Some("/tmp/out.pdf")),
            _ => panic!("unexpected parse result"),
        }
    }

    #[test]
    fn drive_download_requires_file_id() {
        assert!(parse(&["drive", "download"]).is_err());
    }

    // --- drive upload ---

    #[test]
    fn drive_upload_path_only() {
        let cli = parse(&["drive", "upload", "/tmp/file.pdf"]).unwrap();
        match cli.command {
            Command::Drive {
                command: DriveCommand::Upload { path, folder },
            } => {
                assert_eq!(path, "/tmp/file.pdf");
                assert!(folder.is_none());
            }
            _ => panic!("unexpected parse result"),
        }
    }

    #[test]
    fn drive_upload_with_folder() {
        let cli = parse(&["drive", "upload", "/tmp/file.pdf", "--folder", "folder123"]).unwrap();
        match cli.command {
            Command::Drive {
                command: DriveCommand::Upload { folder, .. },
            } => assert_eq!(folder.as_deref(), Some("folder123")),
            _ => panic!("unexpected parse result"),
        }
    }

    // --- global flags ---

    #[test]
    fn global_account_flag() {
        let cli = parse(&["--account", "me@example.com", "auth", "list"]).unwrap();
        assert_eq!(cli.account.as_deref(), Some("me@example.com"));
    }

    #[test]
    fn global_quiet_flag() {
        let cli = parse(&["--quiet", "auth", "list"]).unwrap();
        assert!(cli.quiet);
    }
}
