use clap::Parser;

use crate::auth::config::OAuthAppType;
use crate::cli::{
    AuthCommand, Cli, Command, DocsCommand, DriveCommand, DriveFolderCommand,
    MailAttachmentCommand, MailCommand, SheetsCommand, SheetsInsertDataOption,
    SheetsValueInputOption, SheetsValueRenderOption, SheetsValuesCommand,
};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("goog").chain(args.iter().copied()))
}

fn help(args: &[&str]) -> String {
    let err = parse(args).unwrap_err();
    err.to_string()
}

#[test]
fn auth_setup_no_flags() {
    let cli = parse(&["auth", "setup"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Auth {
            command: AuthCommand::Setup {
                client_secret_file: None,
                app_type: None
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
        app_type: None,
    } = command
    else {
        panic!("unexpected parse result");
    };

    assert_eq!(path, "/tmp/client_secret.json");
}

#[test]
fn auth_setup_with_app_type_flag() {
    let cli = parse(&[
        "auth",
        "setup",
        "--client-secret-file",
        "/tmp/client_secret.json",
        "--app-type",
        "device",
    ])
    .unwrap();
    let Command::Auth { command } = cli.command else {
        panic!("unexpected parse result");
    };
    let AuthCommand::Setup {
        client_secret_file: Some(path),
        app_type: Some(OAuthAppType::Device),
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
fn drive_list_with_folder() {
    let cli = parse(&["drive", "list", "--folder", "folder123"]).unwrap();
    let Command::Drive {
        command:
            DriveCommand::List {
                limit,
                all,
                folder,
                json,
            },
    } = cli.command
    else {
        panic!("unexpected parse result");
    };

    assert_eq!(limit, None);
    assert!(!all);
    assert_eq!(folder.as_deref(), Some("folder123"));
    assert!(!json);
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
            command:
                DriveCommand::List {
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
fn drive_folder_list_defaults() {
    let cli = parse(&["drive", "folder", "list"]).unwrap();
    let Command::Drive {
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
    } = cli.command
    else {
        panic!("unexpected parse result");
    };

    assert_eq!(limit, None);
    assert!(!all);
    assert_eq!(parent, None);
    assert!(!json);
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
fn docs_get_help_explains_raw_document_shape() {
    let help = help(&["docs", "get", "--help"]);

    assert!(help.contains("Emits the Google Docs API Document JSON unchanged."));
    assert!(help.contains("body.content as ordered structural elements"));
    assert!(help.contains("paragraph.elements[].textRun.content"));
    assert!(help.contains("--include-tabs-content"));
}

#[test]
fn docs_batch_update_help_explains_request_shape() {
    let help = help(&["docs", "batch-update", "--help"]);

    assert!(help.contains("--requests reads the full Google Docs documents.batchUpdate JSON body"));
    assert!(help.contains("not only the requests array"));
    assert!(help.contains("writeControl"));
    assert!(help.contains("Common request types:"));
    assert!(help.contains("insertText"));
    assert!(help.contains("updateParagraphStyle"));
    assert!(help.contains("insertTable"));
    assert!(help.contains("addDocumentTab"));
    assert!(help
        .contains("developers.google.com/workspace/docs/api/reference/rest/v1/documents/request"));
    assert!(help.contains("location"));
}

#[test]
fn docs_get_does_not_accept_output_flag() {
    assert!(parse(&["docs", "get", "document-123", "--output", "document.json"]).is_err());
}

#[test]
fn docs_get_accepts_global_account_flag() {
    let cli = parse(&[
        "docs",
        "get",
        "document-123",
        "--account",
        "docs@example.com",
    ])
    .unwrap();
    assert_eq!(cli.account.as_deref(), Some("docs@example.com"));
}

#[test]
fn mail_list_defaults_to_table_with_no_explicit_limit() {
    let cli = parse(&["mail", "list"]).unwrap();
    match cli.command {
        Command::Mail {
            command: MailCommand::List { limit, json },
        } => {
            assert!(limit.is_none());
            assert!(!json);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn mail_list_accepts_limit_and_json() {
    let cli = parse(&["mail", "list", "--limit", "25", "--json"]).unwrap();
    match cli.command {
        Command::Mail {
            command: MailCommand::List { limit, json },
        } => {
            assert_eq!(limit, Some(25));
            assert!(json);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn mail_list_does_not_accept_all() {
    assert!(parse(&["mail", "list", "--all"]).is_err());
}

#[test]
fn mail_search_with_query_limit_and_json() {
    let cli = parse(&[
        "mail",
        "search",
        "from:alice@example.com",
        "--limit",
        "25",
        "--json",
    ])
    .unwrap();
    match cli.command {
        Command::Mail {
            command: MailCommand::Search { query, limit, json },
        } => {
            assert_eq!(query, "from:alice@example.com");
            assert_eq!(limit, Some(25));
            assert!(json);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn mail_search_requires_query() {
    assert!(parse(&["mail", "search"]).is_err());
}

#[test]
fn mail_search_does_not_accept_all() {
    assert!(parse(&["mail", "search", "has:attachment", "--all"]).is_err());
}

#[test]
fn mail_read_with_message_id() {
    let cli = parse(&["mail", "read", "message-123"]).unwrap();
    match cli.command {
        Command::Mail {
            command: MailCommand::Read { message_id },
        } => assert_eq!(message_id, "message-123"),
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn mail_read_requires_message_id() {
    assert!(parse(&["mail", "read"]).is_err());
}

#[test]
fn mail_read_accepts_global_account_flag() {
    let cli = parse(&[
        "mail",
        "read",
        "message-123",
        "--account",
        "mail@example.com",
    ])
    .unwrap();
    assert_eq!(cli.account.as_deref(), Some("mail@example.com"));
}

#[test]
fn mail_attachment_download_with_explicit_output() {
    let cli = parse(&[
        "mail",
        "attachment",
        "download",
        "message-123",
        "attachment-456",
        "--output",
        "report.pdf",
    ])
    .unwrap();
    match cli.command {
        Command::Mail {
            command:
                MailCommand::Attachment {
                    command:
                        MailAttachmentCommand::Download {
                            message_id,
                            attachment_id,
                            output,
                        },
                },
        } => {
            assert_eq!(message_id, "message-123");
            assert_eq!(attachment_id, "attachment-456");
            assert_eq!(output.as_deref(), Some("report.pdf"));
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn mail_attachment_download_output_is_optional() {
    let cli = parse(&[
        "mail",
        "attachment",
        "download",
        "message-123",
        "attachment-456",
    ])
    .unwrap();
    match cli.command {
        Command::Mail {
            command:
                MailCommand::Attachment {
                    command:
                        MailAttachmentCommand::Download {
                            message_id,
                            attachment_id,
                            output,
                        },
                },
        } => {
            assert_eq!(message_id, "message-123");
            assert_eq!(attachment_id, "attachment-456");
            assert!(output.is_none());
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_get_with_spreadsheet_id() {
    let cli = parse(&["sheets", "get", "spreadsheet-123"]).unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Get {
                    spreadsheet_id,
                    fields,
                    include_grid_data,
                    ranges,
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert!(fields.is_none());
            assert!(!include_grid_data);
            assert!(ranges.is_empty());
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_get_with_google_query_flags() {
    let cli = parse(&[
        "sheets",
        "get",
        "spreadsheet-123",
        "--fields",
        "spreadsheetId,properties.title",
        "--include-grid-data",
        "--ranges",
        "Sheet1!A1:B2",
        "--ranges",
        "Summary!A:A",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Get {
                    spreadsheet_id,
                    fields,
                    include_grid_data,
                    ranges,
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(fields.as_deref(), Some("spreadsheetId,properties.title"));
            assert!(include_grid_data);
            assert_eq!(ranges, vec!["Sheet1!A1:B2", "Summary!A:A"]);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_get_defaults_to_formatted_values() {
    let cli = parse(&["sheets", "values", "get", "spreadsheet-123", "Sheet1!A1:B2"]).unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::Get {
                            spreadsheet_id,
                            range,
                            value_render_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!A1:B2");
            assert_eq!(value_render_option, SheetsValueRenderOption::FormattedValue);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_get_accepts_formula_render_option() {
    let cli = parse(&[
        "sheets",
        "values",
        "get",
        "spreadsheet-123",
        "Sheet1!C1:C3",
        "--value-render-option",
        "formula",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::Get {
                            spreadsheet_id,
                            range,
                            value_render_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!C1:C3");
            assert_eq!(value_render_option, SheetsValueRenderOption::Formula);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_batch_get_accepts_repeated_ranges_and_render_option() {
    let cli = parse(&[
        "sheets",
        "values",
        "batch-get",
        "spreadsheet-123",
        "--range",
        "Sheet1!A1:B2",
        "--range",
        "Summary!A:A",
        "--value-render-option",
        "unformatted-value",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::BatchGet {
                            spreadsheet_id,
                            ranges,
                            value_render_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(ranges, vec!["Sheet1!A1:B2", "Summary!A:A"]);
            assert_eq!(
                value_render_option,
                SheetsValueRenderOption::UnformattedValue
            );
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_batch_get_requires_range() {
    assert!(parse(&["sheets", "values", "batch-get", "spreadsheet-123"]).is_err());
}

#[test]
fn sheets_values_update_defaults_to_user_entered() {
    let cli = parse(&[
        "sheets",
        "values",
        "update",
        "spreadsheet-123",
        "Sheet1!A1:B2",
        "--values",
        "values.json",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::Update {
                            spreadsheet_id,
                            range,
                            values,
                            value_input_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!A1:B2");
            assert_eq!(values, "values.json");
            assert_eq!(value_input_option, SheetsValueInputOption::UserEntered);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_update_requires_values() {
    assert!(parse(&[
        "sheets",
        "values",
        "update",
        "spreadsheet-123",
        "Sheet1!A1:B2",
    ])
    .is_err());
}

#[test]
fn sheets_values_append_defaults_to_user_entered_and_insert_rows() {
    let cli = parse(&[
        "sheets",
        "values",
        "append",
        "spreadsheet-123",
        "Sheet1!A:B",
        "--values",
        "values.json",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::Append {
                            spreadsheet_id,
                            range,
                            values,
                            value_input_option,
                            insert_data_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!A:B");
            assert_eq!(values, "values.json");
            assert_eq!(value_input_option, SheetsValueInputOption::UserEntered);
            assert_eq!(insert_data_option, SheetsInsertDataOption::InsertRows);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_append_requires_values() {
    assert!(parse(&[
        "sheets",
        "values",
        "append",
        "spreadsheet-123",
        "Sheet1!A:B",
    ])
    .is_err());
}

#[test]
fn sheets_values_append_accepts_raw_and_overwrite_options() {
    let cli = parse(&[
        "sheets",
        "values",
        "append",
        "spreadsheet-123",
        "Sheet1!A:B",
        "--values",
        "-",
        "--value-input-option",
        "raw",
        "--insert-data-option",
        "overwrite",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::Append {
                            spreadsheet_id,
                            range,
                            values,
                            value_input_option,
                            insert_data_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!A:B");
            assert_eq!(values, "-");
            assert_eq!(value_input_option, SheetsValueInputOption::Raw);
            assert_eq!(insert_data_option, SheetsInsertDataOption::Overwrite);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_clear_with_range() {
    let cli = parse(&[
        "sheets",
        "values",
        "clear",
        "spreadsheet-123",
        "Sheet1!A1:B2",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::Clear {
                            spreadsheet_id,
                            range,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!A1:B2");
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_batch_clear_requires_range() {
    assert!(parse(&["sheets", "values", "batch-clear", "spreadsheet-123"]).is_err());
}

#[test]
fn sheets_values_rejects_unknown_enum_values() {
    assert!(parse(&[
        "sheets",
        "values",
        "get",
        "spreadsheet-123",
        "Sheet1!A1:B2",
        "--value-render-option",
        "displayed",
    ])
    .is_err());
    assert!(parse(&[
        "sheets",
        "values",
        "update",
        "spreadsheet-123",
        "Sheet1!A1:B2",
        "--values",
        "-",
        "--value-input-option",
        "typed",
    ])
    .is_err());
    assert!(parse(&[
        "sheets",
        "values",
        "append",
        "spreadsheet-123",
        "Sheet1!A:B",
        "--values",
        "-",
        "--insert-data-option",
        "replace",
    ])
    .is_err());
}

#[test]
fn sheets_batch_update_with_requests_path() {
    let cli = parse(&[
        "sheets",
        "batch-update",
        "spreadsheet-123",
        "--requests",
        "requests.json",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::BatchUpdate {
                    spreadsheet_id,
                    requests,
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(requests, "requests.json");
        }
        _ => panic!("unexpected parse result"),
    }
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
