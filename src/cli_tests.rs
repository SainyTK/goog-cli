use clap::Parser;

use crate::cli::{AuthCommand, Cli, Command, DocsCommand, DriveCommand, DriveFolderCommand};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("goog").chain(args.iter().copied()))
}

#[test]
fn auth_setup_no_flags() {
    let cli = parse(&["auth", "setup"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Auth {
            command: AuthCommand::Setup {
                client_secret_file: None
            }
        }
    ));
}

#[test]
fn auth_setup_with_client_secret_file_flag() {
    let cli = parse(&[
        "auth",
        "setup",
        "--client-secret-file",
        "/tmp/client_secret.json",
    ])
    .unwrap();
    let Command::Auth { command } = cli.command else {
        panic!("unexpected parse result");
    };
    let AuthCommand::Setup {
        client_secret_file: Some(path),
    } = command
    else {
        panic!("unexpected parse result");
    };

    assert_eq!(path, "/tmp/client_secret.json");
}

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

#[test]
fn drive_list_defaults() {
    let cli = parse(&["drive", "list"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Drive {
            command: DriveCommand::List {
                limit: None,
                all: false,
                folder: None,
                json: false
            }
        }
    ));
}

#[test]
fn drive_list_with_flags() {
    let cli = parse(&[
        "drive",
        "list",
        "--limit",
        "100",
        "--all",
        "--folder",
        "folder123",
        "--json",
    ])
    .unwrap();
    match cli.command {
        Command::Drive {
            command: DriveCommand::List {
                limit,
                all,
                folder,
                json,
            },
        } => {
            assert_eq!(limit, Some(100));
            assert!(all);
            assert_eq!(folder.as_deref(), Some("folder123"));
            assert!(json);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn drive_ls_with_flags() {
    let cli = parse(&[
        "drive",
        "ls",
        "--limit",
        "100",
        "--all",
        "--folder",
        "folder123",
        "--json",
    ])
    .unwrap();
    match cli.command {
        Command::Drive {
            command:
                DriveCommand::Ls {
                    limit,
                    all,
                    folder,
                    json,
                },
        } => {
            assert_eq!(limit, Some(100));
            assert!(all);
            assert_eq!(folder.as_deref(), Some("folder123"));
            assert!(json);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn drive_folder_list_with_flags() {
    let cli = parse(&[
        "drive",
        "folder",
        "list",
        "--limit",
        "100",
        "--all",
        "--parent",
        "folder123",
        "--json",
    ])
    .unwrap();
    match cli.command {
        Command::Drive {
            command:
                DriveCommand::Folder {
                    command:
                        DriveFolderCommand::List {
                            limit,
                            all,
                            parent,
                            json,
                        },
                },
        } => {
            assert_eq!(limit, Some(100));
            assert!(all);
            assert_eq!(parent.as_deref(), Some("folder123"));
            assert!(json);
        }
        _ => panic!("unexpected parse result"),
    }
}

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

#[test]
fn docs_get_with_document_id() {
    let cli = parse(&["docs", "get", "document-123"]).unwrap();
    match cli.command {
        Command::Docs {
            command:
                DocsCommand::Get {
                    document_id,
                    fields,
                    include_tabs_content,
                },
        } => {
            assert_eq!(document_id, "document-123");
            assert!(fields.is_none());
            assert!(!include_tabs_content);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn docs_get_with_google_query_flags() {
    let cli = parse(&[
        "docs",
        "get",
        "document-123",
        "--fields",
        "documentId,title",
        "--include-tabs-content",
    ])
    .unwrap();
    match cli.command {
        Command::Docs {
            command:
                DocsCommand::Get {
                    document_id,
                    fields,
                    include_tabs_content,
                },
        } => {
            assert_eq!(document_id, "document-123");
            assert_eq!(fields.as_deref(), Some("documentId,title"));
            assert!(include_tabs_content);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn docs_batch_update_with_requests_path() {
    let cli = parse(&[
        "docs",
        "batch-update",
        "document-123",
        "--requests",
        "requests.json",
    ])
    .unwrap();
    match cli.command {
        Command::Docs {
            command:
                DocsCommand::BatchUpdate {
                    document_id,
                    requests,
                },
        } => {
            assert_eq!(document_id, "document-123");
            assert_eq!(requests, "requests.json");
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn docs_batch_update_requires_requests() {
    assert!(parse(&["docs", "batch-update", "document-123"]).is_err());
}

#[test]
fn docs_get_does_not_accept_output_flag() {
    assert!(parse(&["docs", "get", "document-123", "--output", "document.json"]).is_err());
}

#[test]
fn docs_get_accepts_global_account_flag() {
    let cli = parse(&["docs", "get", "document-123", "--account", "docs@example.com"]).unwrap();
    assert_eq!(cli.account.as_deref(), Some("docs@example.com"));
}

#[test]
fn global_account_flag() {
    let cli = parse(&["--account", "me@example.com", "auth", "list"]).unwrap();
    assert_eq!(cli.account.as_deref(), Some("me@example.com"));
}

#[test]
fn global_account_flag_after_subcommand() {
    let cli = parse(&["drive", "list", "--account", "me@example.com"]).unwrap();
    assert_eq!(cli.account.as_deref(), Some("me@example.com"));
}

#[test]
fn global_quiet_flag() {
    let cli = parse(&["--quiet", "auth", "list"]).unwrap();
    assert!(cli.quiet);
}
