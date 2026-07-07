use clap::Parser;

use crate::auth::config::OAuthAppType;
use crate::cli::{
    AuthCommand, AuthMappingsCommand, Cli, Command, DocsCommand, DocsListType, DriveCommand,
    DriveFolderCommand, MailAttachmentCommand, MailCommand, MailDraftCommand, SheetsBorderEdge,
    SheetsBorderStyle, SheetsCommand, SheetsConditionalFormatCondition, SheetsDimension,
    SheetsHorizontalAlignment, SheetsInsertDataOption, SheetsMergeType, SheetsNumberFormatType,
    SheetsPasteOrientation, SheetsPasteType, SheetsSheetCommand, SheetsSortOrder,
    SheetsTableOutputFormat, SheetsTextDirection, SheetsValueInputOption, SheetsValueRenderOption,
    SheetsValuesCommand, SheetsVerticalAlignment, SheetsWrapStrategy,
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
fn auth_mappings_list_json() {
    let cli = parse(&["auth", "mappings", "list", "--json"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Auth {
            command: AuthCommand::Mappings {
                command: AuthMappingsCommand::List { json: true }
            }
        }
    ));
}

#[test]
fn auth_mappings_clear_with_surface_and_resource_id() {
    let cli = parse(&[
        "auth",
        "mappings",
        "clear",
        "--surface",
        "docs",
        "--resource-id",
        "document-123",
    ])
    .unwrap();

    match cli.command {
        Command::Auth {
            command:
                AuthCommand::Mappings {
                    command:
                        AuthMappingsCommand::Clear {
                            surface,
                            resource_id,
                        },
                },
        } => {
            assert_eq!(surface.as_deref(), Some("docs"));
            assert_eq!(resource_id.as_deref(), Some("document-123"));
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn auth_mappings_help_uses_glossary_terms() {
    let text = help(&["auth", "mappings"]);

    assert!(text.contains("Resource Account Mappings"));
    assert!(text.contains("Account"));
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
fn docs_map_with_document_id_and_json_flag() {
    let cli = parse(&["docs", "map", "document-123", "--json"]).unwrap();
    match cli.command {
        Command::Docs {
            command: DocsCommand::Map { document_id, json },
        } => {
            assert_eq!(document_id, "document-123");
            assert!(json);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn docs_search_text_with_document_id_text_and_json_flag() {
    let cli = parse(&["docs", "search-text", "document-123", "Plan", "--json"]).unwrap();
    match cli.command {
        Command::Docs {
            command:
                DocsCommand::SearchText {
                    document_id,
                    text,
                    json,
                },
        } => {
            assert_eq!(document_id, "document-123");
            assert_eq!(text, "Plan");
            assert!(json);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn docs_get_content_accepts_location_selectors() {
    let by_index = parse(&["docs", "get-content", "document-123", "--index", "44"]).unwrap();
    match by_index.command {
        Command::Docs {
            command:
                DocsCommand::GetContent {
                    document_id,
                    index,
                    entry,
                    page,
                    line,
                    heading,
                    json,
                },
        } => {
            assert_eq!(document_id, "document-123");
            assert_eq!(index, Some(44));
            assert_eq!(entry, None);
            assert_eq!(page, None);
            assert_eq!(line, None);
            assert_eq!(heading, None);
            assert!(!json);
        }
        _ => panic!("unexpected parse result"),
    }

    let by_entry = parse(&["docs", "get-content", "document-123", "--entry", "44"]).unwrap();
    match by_entry.command {
        Command::Docs {
            command: DocsCommand::GetContent { index, entry, .. },
        } => {
            assert_eq!(index, None);
            assert_eq!(entry, Some(44));
        }
        _ => panic!("unexpected parse result"),
    }

    assert!(parse(&[
        "docs",
        "get-content",
        "document-123",
        "--page",
        "2",
        "--line",
        "1",
    ])
    .is_ok());
    assert!(parse(&[
        "docs",
        "get-content",
        "document-123",
        "--heading",
        "Appendix",
        "--json",
    ])
    .is_ok());
}

#[test]
fn docs_insert_text_parses_location_and_write_options() {
    let cli = parse(&[
        "docs",
        "insert-text",
        "document-123",
        "Hello",
        "--page",
        "2",
        "--line",
        "1",
        "--dry-run",
        "--json",
        "--required-revision-id",
        "rev-123",
    ])
    .unwrap();

    let Command::Docs { command } = cli.command else {
        panic!("unexpected parse result");
    };
    let DocsCommand::InsertText {
        document_id,
        text,
        index,
        entry,
        page,
        line,
        after_heading,
        before_heading,
        after_text,
        before_text,
        dry_run,
        json,
        required_revision_id,
    } = command
    else {
        panic!("unexpected parse result");
    };

    assert_eq!(document_id, "document-123");
    assert_eq!(text, "Hello");
    assert_eq!(index, None);
    assert_eq!(entry, None);
    assert_eq!(page, Some(2));
    assert_eq!(line, Some(1));
    assert_eq!(after_heading, None);
    assert_eq!(before_heading, None);
    assert_eq!(after_text, None);
    assert_eq!(before_text, None);
    assert!(dry_run);
    assert!(json);
    assert_eq!(required_revision_id.as_deref(), Some("rev-123"));
}

#[test]
fn docs_replace_text_parses_match_and_write_options() {
    let cli = parse(&[
        "docs",
        "replace-text",
        "document-123",
        "old",
        "new",
        "--match",
        "2",
        "--dry-run",
        "--json",
        "--required-revision-id",
        "rev-123",
    ])
    .unwrap();

    let Command::Docs { command } = cli.command else {
        panic!("unexpected parse result");
    };
    let DocsCommand::ReplaceText {
        document_id,
        old_text,
        new_text,
        match_number,
        all,
        dry_run,
        json,
        required_revision_id,
    } = command
    else {
        panic!("unexpected parse result");
    };

    assert_eq!(document_id, "document-123");
    assert_eq!(old_text, "old");
    assert_eq!(new_text, "new");
    assert_eq!(match_number, Some(2));
    assert!(!all);
    assert!(dry_run);
    assert!(json);
    assert_eq!(required_revision_id.as_deref(), Some("rev-123"));
}

#[test]
fn docs_new_high_level_editing_commands_parse() {
    assert!(matches!(
        parse(&["docs", "list-images", "document-123", "--json"])
            .unwrap()
            .command,
        Command::Docs {
            command: DocsCommand::ListImages { .. }
        }
    ));
    assert!(matches!(
        parse(&["docs", "list-tables", "document-123", "--json"])
            .unwrap()
            .command,
        Command::Docs {
            command: DocsCommand::ListTables { .. }
        }
    ));

    let Command::Docs {
        command:
            DocsCommand::InsertImage {
                image_uri,
                page,
                line,
                dry_run,
                json,
                ..
            },
    } = parse(&[
        "docs",
        "insert-image",
        "document-123",
        "https://example.test/image.png",
        "--page",
        "2",
        "--line",
        "1",
        "--dry-run",
        "--json",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(image_uri, "https://example.test/image.png");
    assert_eq!(page, Some(2));
    assert_eq!(line, Some(1));
    assert!(dry_run);
    assert!(json);

    let Command::Docs {
        command: DocsCommand::ApplyList {
            list_type, entry, ..
        },
    } = parse(&[
        "docs",
        "apply-list",
        "document-123",
        "--entry",
        "2",
        "--type",
        "checkbox",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(entry, Some(2));
    assert_eq!(list_type, Some(crate::cli::DocsListType::Checkbox));

    for (value, expected) in [
        ("bullet", DocsListType::Bullet),
        ("numbered", DocsListType::Numbered),
        ("dash", DocsListType::Dash),
        ("checkbox", DocsListType::Checkbox),
    ] {
        let Command::Docs {
            command:
                DocsCommand::ApplyList {
                    list_type,
                    from_index,
                    to_index,
                    ..
                },
        } = parse(&[
            "docs",
            "apply-list",
            "document-123",
            "--from-index",
            "4",
            "--to-index",
            "12",
            "--type",
            value,
        ])
        .unwrap()
        .command
        else {
            panic!("unexpected parse result");
        };
        assert_eq!(from_index, Some(4));
        assert_eq!(to_index, Some(12));
        assert_eq!(list_type, Some(expected));
    }

    let Command::Docs {
        command:
            DocsCommand::ApplyList {
                preset,
                list_type,
                page,
                line,
                ..
            },
    } = parse(&[
        "docs",
        "apply-list",
        "document-123",
        "--page",
        "3",
        "--line",
        "4",
        "--preset",
        "BULLET_STAR_CIRCLE_SQUARE",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(page, Some(3));
    assert_eq!(line, Some(4));
    assert_eq!(list_type, None);
    assert_eq!(preset.as_deref(), Some("BULLET_STAR_CIRCLE_SQUARE"));

    let Command::Docs {
        command:
            DocsCommand::ApplyStyles {
                style_json,
                from_index,
                to_index,
                ..
            },
    } = parse(&[
        "docs",
        "apply-styles",
        "document-123",
        "--from-index",
        "1",
        "--to-index",
        "9",
        "--style-json",
        r#"{"textStyle":{"underline":true}}"#,
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(from_index, Some(1));
    assert_eq!(to_index, Some(9));
    assert_eq!(
        style_json.as_deref(),
        Some(r#"{"textStyle":{"underline":true}}"#)
    );

    let Command::Docs {
        command:
            DocsCommand::InsertTable {
                data,
                page,
                line,
                dry_run,
                json,
                ..
            },
    } = parse(&[
        "docs",
        "insert-table",
        "document-123",
        "--data",
        "table.csv",
        "--page",
        "2",
        "--line",
        "1",
        "--dry-run",
        "--json",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(data.as_deref(), Some("table.csv"));
    assert_eq!(page, Some(2));
    assert_eq!(line, Some(1));
    assert!(dry_run);
    assert!(json);

    let Command::Docs {
        command:
            DocsCommand::EditTable {
                table_id,
                data,
                dry_run,
                json,
                ..
            },
    } = parse(&[
        "docs",
        "edit-table",
        "document-123",
        "--table-id",
        "table-3",
        "--data",
        "table.csv",
        "--dry-run",
        "--json",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(table_id, "table-3");
    assert_eq!(data, "table.csv");
    assert!(dry_run);
    assert!(json);

    let Command::Docs {
        command:
            DocsCommand::InsertTable {
                rows,
                columns,
                no_auto_style,
                ..
            },
    } = parse(&[
        "docs",
        "insert-table",
        "document-123",
        "--rows",
        "2",
        "--columns",
        "3",
        "--index",
        "44",
        "--no-auto-style",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(rows, Some(2));
    assert_eq!(columns, Some(3));
    assert!(no_auto_style);

    let Command::Docs {
        command:
            DocsCommand::ApplyStyles {
                heading,
                no_auto_style,
                ..
            },
    } = parse(&[
        "docs",
        "apply-styles",
        "document-123",
        "--entry",
        "2",
        "--heading",
        "HEADING_2",
        "--no-auto-style",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(heading.as_deref(), Some("HEADING_2"));
    assert!(no_auto_style);

    let Command::Docs {
        command:
            DocsCommand::ApplyList {
                entry,
                no_auto_style,
                ..
            },
    } = parse(&[
        "docs",
        "apply-list",
        "document-123",
        "--entry",
        "2",
        "--no-auto-style",
        "--preset",
        "BULLET_DISC_CIRCLE_SQUARE",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(entry, Some(2));
    assert!(no_auto_style);
}

#[test]
fn docs_show_style_template_parse() {
    let cli = parse(&["docs", "show-style-template", "document-123", "--json"]).unwrap();

    match cli.command {
        Command::Docs {
            command: DocsCommand::ShowStyleTemplate { document_id, json },
        } => {
            assert_eq!(document_id, "document-123");
            assert!(json);
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
            command: MailCommand::Read { message_id, json },
        } => {
            assert_eq!(message_id, "message-123");
            assert!(!json);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn mail_read_accepts_json_flag() {
    let cli = parse(&["mail", "read", "message-123", "--json"]).unwrap();
    match cli.command {
        Command::Mail {
            command: MailCommand::Read { json, .. },
        } => assert!(json),
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
fn mail_draft_create_with_body_flags() {
    let cli = parse(&[
        "mail",
        "draft",
        "create",
        "--to",
        "alice@example.com",
        "--to",
        "bob@example.com",
        "--cc",
        "carol@example.com",
        "--bcc",
        "dave@example.com",
        "--subject",
        "Draft subject",
        "--body",
        "Hello from goog",
        "--json",
    ])
    .unwrap();
    match cli.command {
        Command::Mail {
            command:
                MailCommand::Draft {
                    command:
                        MailDraftCommand::Create {
                            to,
                            cc,
                            bcc,
                            subject,
                            body,
                            body_file,
                            attachment,
                            json,
                        },
                },
        } => {
            assert_eq!(to, ["alice@example.com", "bob@example.com"]);
            assert_eq!(cc, ["carol@example.com"]);
            assert_eq!(bcc, ["dave@example.com"]);
            assert_eq!(subject, "Draft subject");
            assert_eq!(body.as_deref(), Some("Hello from goog"));
            assert!(body_file.is_none());
            assert!(attachment.is_empty());
            assert!(json);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn mail_draft_create_with_attachment_paths() {
    let cli = parse(&[
        "mail",
        "draft",
        "create",
        "--to",
        "alice@example.com",
        "--subject",
        "Draft subject",
        "--body",
        "Hello from goog",
        "--attachment",
        "./invoice.pdf",
        "--attachment",
        "./terms.txt",
    ])
    .unwrap();
    match cli.command {
        Command::Mail {
            command:
                MailCommand::Draft {
                    command:
                        MailDraftCommand::Create {
                            attachment, json, ..
                        },
                },
        } => {
            assert_eq!(attachment, ["./invoice.pdf", "./terms.txt"]);
            assert!(!json);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn mail_draft_create_requires_to_recipient() {
    let err = parse(&[
        "mail",
        "draft",
        "create",
        "--subject",
        "Draft subject",
        "--body",
        "Hello from goog",
    ])
    .unwrap_err();

    assert!(err.to_string().contains("--to <TO>"));
}

#[test]
fn mail_draft_edit_with_body_and_attachment_paths() {
    let cli = parse(&[
        "mail",
        "draft",
        "edit",
        "draft-123",
        "--to",
        "alice@example.com",
        "--subject",
        "Updated draft",
        "--body",
        "Updated body",
        "--attachment",
        "./updated.pdf",
        "--json",
    ])
    .unwrap();
    match cli.command {
        Command::Mail {
            command:
                MailCommand::Draft {
                    command:
                        MailDraftCommand::Edit {
                            draft_id,
                            to,
                            cc,
                            bcc,
                            subject,
                            body,
                            body_file,
                            attachment,
                            json,
                        },
                },
        } => {
            assert_eq!(draft_id, "draft-123");
            assert_eq!(to, ["alice@example.com"]);
            assert!(cc.is_empty());
            assert!(bcc.is_empty());
            assert_eq!(subject, "Updated draft");
            assert_eq!(body.as_deref(), Some("Updated body"));
            assert!(body_file.is_none());
            assert_eq!(attachment, ["./updated.pdf"]);
            assert!(json);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn mail_draft_edit_requires_to_recipient() {
    let err = parse(&[
        "mail",
        "draft",
        "edit",
        "draft-123",
        "--subject",
        "Updated draft",
        "--body",
        "Updated body",
    ])
    .unwrap_err();

    assert!(err.to_string().contains("--to <TO>"));
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
fn sheets_create_with_title() {
    let cli = parse(&["sheets", "create", "Quarterly Plan"]).unwrap();
    match cli.command {
        Command::Sheets {
            command: SheetsCommand::Create { title },
        } => {
            assert_eq!(title, "Quarterly Plan");
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
fn sheets_values_update_table_accepts_data_file() {
    let cli = parse(&[
        "sheets",
        "values",
        "update-table",
        "spreadsheet-123",
        "Sheet1!A1:C3",
        "--data",
        "./rows.tsv",
        "--value-input-option",
        "raw",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::UpdateTable {
                            spreadsheet_id,
                            range,
                            data,
                            value_input_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!A1:C3");
            assert_eq!(data, "./rows.tsv");
            assert_eq!(value_input_option, SheetsValueInputOption::Raw);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_update_table_requires_data_file() {
    assert!(parse(&[
        "sheets",
        "values",
        "update-table",
        "spreadsheet-123",
        "Sheet1!A1:C3",
    ])
    .is_err());
}

#[test]
fn sheets_values_get_cell_accepts_range_and_render_option() {
    let cli = parse(&[
        "sheets",
        "values",
        "get-cell",
        "spreadsheet-123",
        "Sheet1!D2",
        "--value-render-option",
        "formula",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::GetCell {
                            spreadsheet_id,
                            range,
                            value_render_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!D2");
            assert_eq!(value_render_option, SheetsValueRenderOption::Formula);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_get_row_accepts_range_and_render_option() {
    let cli = parse(&[
        "sheets",
        "values",
        "get-row",
        "spreadsheet-123",
        "Sheet1!A2:C2",
        "--value-render-option",
        "formula",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::GetRow {
                            spreadsheet_id,
                            range,
                            value_render_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!A2:C2");
            assert_eq!(value_render_option, SheetsValueRenderOption::Formula);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_get_column_accepts_range_and_render_option() {
    let cli = parse(&[
        "sheets",
        "values",
        "get-column",
        "spreadsheet-123",
        "Sheet1!D2:D10",
        "--value-render-option",
        "unformatted-value",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::GetColumn {
                            spreadsheet_id,
                            range,
                            value_render_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!D2:D10");
            assert_eq!(
                value_render_option,
                SheetsValueRenderOption::UnformattedValue
            );
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_get_table_accepts_range_and_render_option() {
    let cli = parse(&[
        "sheets",
        "values",
        "get-table",
        "spreadsheet-123",
        "Sheet1!A1:C10",
        "--value-render-option",
        "formula",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::GetTable {
                            spreadsheet_id,
                            range,
                            value_render_option,
                            format,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!A1:C10");
            assert_eq!(value_render_option, SheetsValueRenderOption::Formula);
            assert_eq!(format, SheetsTableOutputFormat::Tsv);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_get_table_accepts_csv_format() {
    let cli = parse(&[
        "sheets",
        "values",
        "get-table",
        "spreadsheet-123",
        "Sheet1!A1:C10",
        "--format",
        "csv",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command: SheetsValuesCommand::GetTable { format, .. },
                },
        } => {
            assert_eq!(format, SheetsTableOutputFormat::Csv);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_update_cell_accepts_value_argument() {
    let cli = parse(&[
        "sheets",
        "values",
        "update-cell",
        "spreadsheet-123",
        "Sheet1!D2",
        "=SUM(B2:B4)",
        "--value-input-option",
        "raw",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::UpdateCell {
                            spreadsheet_id,
                            range,
                            value,
                            value_input_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!D2");
            assert_eq!(value, "=SUM(B2:B4)");
            assert_eq!(value_input_option, SheetsValueInputOption::Raw);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_update_cell_requires_value() {
    assert!(parse(&[
        "sheets",
        "values",
        "update-cell",
        "spreadsheet-123",
        "Sheet1!D2",
    ])
    .is_err());
}

#[test]
fn sheets_values_update_row_accepts_repeated_values() {
    let cli = parse(&[
        "sheets",
        "values",
        "update-row",
        "spreadsheet-123",
        "Sheet1!A2:C2",
        "--value",
        "Grace",
        "--value",
        "99",
        "--value",
        "=SUM(B2:B4)",
        "--value-input-option",
        "raw",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::UpdateRow {
                            spreadsheet_id,
                            range,
                            values,
                            value_input_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!A2:C2");
            assert_eq!(values, ["Grace", "99", "=SUM(B2:B4)"]);
            assert_eq!(value_input_option, SheetsValueInputOption::Raw);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_update_row_requires_value() {
    assert!(parse(&[
        "sheets",
        "values",
        "update-row",
        "spreadsheet-123",
        "Sheet1!A2:C2",
    ])
    .is_err());
}

#[test]
fn sheets_values_update_column_accepts_repeated_values() {
    let cli = parse(&[
        "sheets",
        "values",
        "update-column",
        "spreadsheet-123",
        "Sheet1!D2:D4",
        "--value",
        "Open",
        "--value",
        "Closed",
        "--value",
        "Blocked",
        "--value-input-option",
        "raw",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::UpdateColumn {
                            spreadsheet_id,
                            range,
                            values,
                            value_input_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!D2:D4");
            assert_eq!(values, ["Open", "Closed", "Blocked"]);
            assert_eq!(value_input_option, SheetsValueInputOption::Raw);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_update_column_requires_value() {
    assert!(parse(&[
        "sheets",
        "values",
        "update-column",
        "spreadsheet-123",
        "Sheet1!D2:D4",
    ])
    .is_err());
}

#[test]
fn sheets_values_batch_update_with_values_path() {
    let cli = parse(&[
        "sheets",
        "values",
        "batch-update",
        "spreadsheet-123",
        "--values",
        "values.json",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::BatchUpdate {
                            spreadsheet_id,
                            values,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(values, "values.json");
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_batch_update_requires_values() {
    assert!(parse(&["sheets", "values", "batch-update", "spreadsheet-123"]).is_err());
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
fn sheets_values_append_row_accepts_repeated_values() {
    let cli = parse(&[
        "sheets",
        "values",
        "append-row",
        "spreadsheet-123",
        "Sheet1!A:C",
        "--value",
        "Grace",
        "--value",
        "99",
        "--value",
        "=SUM(B2:B4)",
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
                        SheetsValuesCommand::AppendRow {
                            spreadsheet_id,
                            range,
                            values,
                            value_input_option,
                            insert_data_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!A:C");
            assert_eq!(values, ["Grace", "99", "=SUM(B2:B4)"]);
            assert_eq!(value_input_option, SheetsValueInputOption::Raw);
            assert_eq!(insert_data_option, SheetsInsertDataOption::Overwrite);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_append_row_requires_value() {
    assert!(parse(&[
        "sheets",
        "values",
        "append-row",
        "spreadsheet-123",
        "Sheet1!A:C",
    ])
    .is_err());
}

#[test]
fn sheets_values_append_column_accepts_repeated_values() {
    let cli = parse(&[
        "sheets",
        "values",
        "append-column",
        "spreadsheet-123",
        "Sheet1!A:D",
        "--value",
        "Open",
        "--value",
        "Closed",
        "--value",
        "Blocked",
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
                        SheetsValuesCommand::AppendColumn {
                            spreadsheet_id,
                            range,
                            values,
                            value_input_option,
                            insert_data_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!A:D");
            assert_eq!(values, ["Open", "Closed", "Blocked"]);
            assert_eq!(value_input_option, SheetsValueInputOption::Raw);
            assert_eq!(insert_data_option, SheetsInsertDataOption::Overwrite);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_append_column_requires_value() {
    assert!(parse(&[
        "sheets",
        "values",
        "append-column",
        "spreadsheet-123",
        "Sheet1!A:D",
    ])
    .is_err());
}

#[test]
fn sheets_values_append_table_accepts_data_file() {
    let cli = parse(&[
        "sheets",
        "values",
        "append-table",
        "spreadsheet-123",
        "Sheet1!A:C",
        "--data",
        "./rows.tsv",
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
                        SheetsValuesCommand::AppendTable {
                            spreadsheet_id,
                            range,
                            data,
                            value_input_option,
                            insert_data_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, "Sheet1!A:C");
            assert_eq!(data, "./rows.tsv");
            assert_eq!(value_input_option, SheetsValueInputOption::Raw);
            assert_eq!(insert_data_option, SheetsInsertDataOption::Overwrite);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_append_table_requires_data_file() {
    assert!(parse(&[
        "sheets",
        "values",
        "append-table",
        "spreadsheet-123",
        "Sheet1!A:C",
    ])
    .is_err());
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
fn sheets_values_batch_clear_accepts_repeated_ranges() {
    let cli = parse(&[
        "sheets",
        "values",
        "batch-clear",
        "spreadsheet-123",
        "--range",
        "Sheet1!A1:B2",
        "--range",
        "Summary!A:A",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Values {
                    command:
                        SheetsValuesCommand::BatchClear {
                            spreadsheet_id,
                            ranges,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(ranges, vec!["Sheet1!A1:B2", "Summary!A:A"]);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_batch_clear_requires_range() {
    assert!(parse(&["sheets", "values", "batch-clear", "spreadsheet-123"]).is_err());
}

#[test]
fn sheets_sheet_add_accepts_title_and_optional_properties() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "add",
        "spreadsheet-123",
        "Planning",
        "--sheet-id",
        "42",
        "--index",
        "1",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Add {
                            spreadsheet_id,
                            title,
                            sheet_id,
                            index,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(title, "Planning");
            assert_eq!(sheet_id, Some(42));
            assert_eq!(index, Some(1));
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_delete_accepts_sheet_id() {
    let cli = parse(&["sheets", "sheet", "delete", "spreadsheet-123", "42"]).unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Delete {
                            spreadsheet_id,
                            sheet_id,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_rename_accepts_sheet_id_and_title() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "rename",
        "spreadsheet-123",
        "42",
        "Archive",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Rename {
                            spreadsheet_id,
                            sheet_id,
                            title,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(title, "Archive");
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_move_accepts_sheet_id_and_index() {
    let cli = parse(&["sheets", "sheet", "move", "spreadsheet-123", "42", "3"]).unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Move {
                            spreadsheet_id,
                            sheet_id,
                            index,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(index, 3);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_duplicate_accepts_source_sheet_id_title_and_optional_properties() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "duplicate",
        "spreadsheet-123",
        "42",
        "Planning Copy",
        "--sheet-id",
        "43",
        "--index",
        "2",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Duplicate {
                            spreadsheet_id,
                            source_sheet_id,
                            title,
                            sheet_id,
                            index,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(source_sheet_id, 42);
            assert_eq!(title, "Planning Copy");
            assert_eq!(sheet_id, Some(43));
            assert_eq!(index, Some(2));
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_freeze_accepts_rows_and_columns() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "freeze",
        "spreadsheet-123",
        "42",
        "--rows",
        "1",
        "--columns",
        "2",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Freeze {
                            spreadsheet_id,
                            sheet_id,
                            rows,
                            columns,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(rows, Some(1));
            assert_eq!(columns, Some(2));
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_freeze_requires_rows_or_columns() {
    assert!(parse(&["sheets", "sheet", "freeze", "spreadsheet-123", "42"]).is_err());
}

#[test]
fn sheets_sheet_freeze_rejects_negative_counts() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "freeze",
        "spreadsheet-123",
        "42",
        "--rows",
        "-1",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_resize_accepts_rows_and_columns() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "resize",
        "spreadsheet-123",
        "42",
        "--rows",
        "200",
        "--columns",
        "12",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Resize {
                            spreadsheet_id,
                            sheet_id,
                            rows,
                            columns,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(rows, Some(200));
            assert_eq!(columns, Some(12));
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_resize_requires_rows_or_columns() {
    assert!(parse(&["sheets", "sheet", "resize", "spreadsheet-123", "42"]).is_err());
}

#[test]
fn sheets_sheet_resize_rejects_zero_counts() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "resize",
        "spreadsheet-123",
        "42",
        "--rows",
        "0",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_auto_resize_accepts_dimension_and_indexes() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "auto-resize",
        "spreadsheet-123",
        "42",
        "--dimension",
        "columns",
        "--start-index",
        "0",
        "--end-index",
        "5",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::AutoResize {
                            spreadsheet_id,
                            sheet_id,
                            dimension,
                            start_index,
                            end_index,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(dimension, SheetsDimension::Columns);
            assert_eq!(start_index, 0);
            assert_eq!(end_index, 5);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_auto_resize_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "auto-resize",
        "spreadsheet-123",
        "42",
        "--dimension",
        "rows",
        "--start-index",
        "-1",
        "--end-index",
        "5",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_set_dimension_size_accepts_dimension_indexes_and_pixel_size() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "set-dimension-size",
        "spreadsheet-123",
        "42",
        "--dimension",
        "rows",
        "--start-index",
        "1",
        "--end-index",
        "3",
        "--pixel-size",
        "28",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::SetDimensionSize {
                            spreadsheet_id,
                            sheet_id,
                            dimension,
                            start_index,
                            end_index,
                            pixel_size,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(dimension, SheetsDimension::Rows);
            assert_eq!(start_index, 1);
            assert_eq!(end_index, 3);
            assert_eq!(pixel_size, 28);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_set_dimension_size_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "set-dimension-size",
        "spreadsheet-123",
        "42",
        "--dimension",
        "columns",
        "--start-index",
        "-1",
        "--end-index",
        "5",
        "--pixel-size",
        "80",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_set_dimension_size_rejects_zero_pixel_size() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "set-dimension-size",
        "spreadsheet-123",
        "42",
        "--dimension",
        "columns",
        "--start-index",
        "0",
        "--end-index",
        "5",
        "--pixel-size",
        "0",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_hide_dimension_accepts_dimension_and_indexes() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "hide-dimension",
        "spreadsheet-123",
        "42",
        "--dimension",
        "columns",
        "--start-index",
        "1",
        "--end-index",
        "3",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::HideDimension {
                            spreadsheet_id,
                            sheet_id,
                            dimension,
                            start_index,
                            end_index,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(dimension, SheetsDimension::Columns);
            assert_eq!(start_index, 1);
            assert_eq!(end_index, 3);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_hide_dimension_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "hide-dimension",
        "spreadsheet-123",
        "42",
        "--dimension",
        "rows",
        "--start-index",
        "-1",
        "--end-index",
        "5",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_unhide_dimension_accepts_dimension_and_indexes() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "unhide-dimension",
        "spreadsheet-123",
        "42",
        "--dimension",
        "rows",
        "--start-index",
        "4",
        "--end-index",
        "8",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::UnhideDimension {
                            spreadsheet_id,
                            sheet_id,
                            dimension,
                            start_index,
                            end_index,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(dimension, SheetsDimension::Rows);
            assert_eq!(start_index, 4);
            assert_eq!(end_index, 8);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_unhide_dimension_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "unhide-dimension",
        "spreadsheet-123",
        "42",
        "--dimension",
        "columns",
        "--start-index",
        "0",
        "--end-index",
        "-5",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_group_dimension_accepts_dimension_and_indexes() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "group-dimension",
        "spreadsheet-123",
        "42",
        "--dimension",
        "rows",
        "--start-index",
        "1",
        "--end-index",
        "5",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::GroupDimension {
                            spreadsheet_id,
                            sheet_id,
                            dimension,
                            start_index,
                            end_index,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(dimension, SheetsDimension::Rows);
            assert_eq!(start_index, 1);
            assert_eq!(end_index, 5);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_group_dimension_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "group-dimension",
        "spreadsheet-123",
        "42",
        "--dimension",
        "columns",
        "--start-index",
        "-1",
        "--end-index",
        "5",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_ungroup_dimension_accepts_dimension_and_indexes() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "ungroup-dimension",
        "spreadsheet-123",
        "42",
        "--dimension",
        "columns",
        "--start-index",
        "2",
        "--end-index",
        "6",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::UngroupDimension {
                            spreadsheet_id,
                            sheet_id,
                            dimension,
                            start_index,
                            end_index,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(dimension, SheetsDimension::Columns);
            assert_eq!(start_index, 2);
            assert_eq!(end_index, 6);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_ungroup_dimension_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "ungroup-dimension",
        "spreadsheet-123",
        "42",
        "--dimension",
        "rows",
        "--start-index",
        "0",
        "--end-index",
        "-1",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_collapse_dimension_group_accepts_dimension_and_indexes() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "collapse-dimension-group",
        "spreadsheet-123",
        "42",
        "--dimension",
        "rows",
        "--start-index",
        "1",
        "--end-index",
        "5",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::CollapseDimensionGroup {
                            spreadsheet_id,
                            sheet_id,
                            dimension,
                            start_index,
                            end_index,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(dimension, SheetsDimension::Rows);
            assert_eq!(start_index, 1);
            assert_eq!(end_index, 5);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_collapse_dimension_group_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "collapse-dimension-group",
        "spreadsheet-123",
        "42",
        "--dimension",
        "columns",
        "--start-index",
        "-1",
        "--end-index",
        "5",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_expand_dimension_group_accepts_dimension_and_indexes() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "expand-dimension-group",
        "spreadsheet-123",
        "42",
        "--dimension",
        "columns",
        "--start-index",
        "2",
        "--end-index",
        "6",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::ExpandDimensionGroup {
                            spreadsheet_id,
                            sheet_id,
                            dimension,
                            start_index,
                            end_index,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(dimension, SheetsDimension::Columns);
            assert_eq!(start_index, 2);
            assert_eq!(end_index, 6);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_expand_dimension_group_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "expand-dimension-group",
        "spreadsheet-123",
        "42",
        "--dimension",
        "rows",
        "--start-index",
        "0",
        "--end-index",
        "-1",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_insert_dimension_accepts_dimension_indexes_and_inheritance() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "insert-dimension",
        "spreadsheet-123",
        "42",
        "--dimension",
        "rows",
        "--start-index",
        "2",
        "--end-index",
        "4",
        "--inherit-from-before",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::InsertDimension {
                            spreadsheet_id,
                            sheet_id,
                            dimension,
                            start_index,
                            end_index,
                            inherit_from_before,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(dimension, SheetsDimension::Rows);
            assert_eq!(start_index, 2);
            assert_eq!(end_index, 4);
            assert!(inherit_from_before);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_insert_dimension_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "insert-dimension",
        "spreadsheet-123",
        "42",
        "--dimension",
        "columns",
        "--start-index",
        "-1",
        "--end-index",
        "5",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_delete_dimension_accepts_dimension_and_indexes() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "delete-dimension",
        "spreadsheet-123",
        "42",
        "--dimension",
        "columns",
        "--start-index",
        "3",
        "--end-index",
        "6",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::DeleteDimension {
                            spreadsheet_id,
                            sheet_id,
                            dimension,
                            start_index,
                            end_index,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(dimension, SheetsDimension::Columns);
            assert_eq!(start_index, 3);
            assert_eq!(end_index, 6);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_delete_dimension_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "delete-dimension",
        "spreadsheet-123",
        "42",
        "--dimension",
        "rows",
        "--start-index",
        "0",
        "--end-index",
        "-1",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_basic_filter_accepts_grid_range() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "basic-filter",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "100",
        "--start-column",
        "0",
        "--end-column",
        "5",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::BasicFilter {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 100);
            assert_eq!(start_column, 0);
            assert_eq!(end_column, 5);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_basic_filter_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "basic-filter",
        "spreadsheet-123",
        "42",
        "--start-row",
        "-1",
        "--end-row",
        "100",
        "--start-column",
        "0",
        "--end-column",
        "5",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_clear_basic_filter_accepts_sheet_id() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "clear-basic-filter",
        "spreadsheet-123",
        "42",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::ClearBasicFilter {
                            spreadsheet_id,
                            sheet_id,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_merge_accepts_grid_range_and_merge_type() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "merge",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "2",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--merge-type",
        "rows",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Merge {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            merge_type,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 2);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert_eq!(merge_type, SheetsMergeType::Rows);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_merge_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "merge",
        "spreadsheet-123",
        "42",
        "--start-row",
        "-1",
        "--end-row",
        "2",
        "--start-column",
        "1",
        "--end-column",
        "4",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_unmerge_accepts_grid_range() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "unmerge",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "2",
        "--start-column",
        "1",
        "--end-column",
        "4",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Unmerge {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 2);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_sort_range_accepts_grid_range_and_order() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "sort-range",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "100",
        "--start-column",
        "0",
        "--end-column",
        "5",
        "--sort-column",
        "3",
        "--order",
        "descending",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::SortRange {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            sort_column,
                            order,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 1);
            assert_eq!(end_row, 100);
            assert_eq!(start_column, 0);
            assert_eq!(end_column, 5);
            assert_eq!(sort_column, 3);
            assert_eq!(order, SheetsSortOrder::Descending);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_sort_range_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "sort-range",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "100",
        "--start-column",
        "0",
        "--end-column",
        "5",
        "--sort-column",
        "-1",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_delete_duplicates_accepts_grid_range_and_comparison_columns() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "delete-duplicates",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "100",
        "--start-column",
        "0",
        "--end-column",
        "5",
        "--comparison-column",
        "1",
        "--comparison-column",
        "3",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::DeleteDuplicates {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            comparison_columns,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 1);
            assert_eq!(end_row, 100);
            assert_eq!(start_column, 0);
            assert_eq!(end_column, 5);
            assert_eq!(comparison_columns, vec![1, 3]);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_delete_duplicates_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "delete-duplicates",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "100",
        "--start-column",
        "0",
        "--end-column",
        "5",
        "--comparison-column",
        "-1",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_trim_whitespace_accepts_grid_range() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "trim-whitespace",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "100",
        "--start-column",
        "0",
        "--end-column",
        "5",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::TrimWhitespace {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 1);
            assert_eq!(end_row, 100);
            assert_eq!(start_column, 0);
            assert_eq!(end_column, 5);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_trim_whitespace_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "trim-whitespace",
        "spreadsheet-123",
        "42",
        "--start-row",
        "-1",
        "--end-row",
        "100",
        "--start-column",
        "0",
        "--end-column",
        "5",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_randomize_range_accepts_grid_range() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "randomize-range",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "100",
        "--start-column",
        "0",
        "--end-column",
        "5",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::RandomizeRange {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 1);
            assert_eq!(end_row, 100);
            assert_eq!(start_column, 0);
            assert_eq!(end_column, 5);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_randomize_range_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "randomize-range",
        "spreadsheet-123",
        "42",
        "--start-row",
        "-1",
        "--end-row",
        "100",
        "--start-column",
        "0",
        "--end-column",
        "5",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_find_replace_accepts_scope_and_match_options() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "find-replace",
        "spreadsheet-123",
        "old",
        "new",
        "--sheet-id",
        "42",
        "--match-case",
        "--match-entire-cell",
        "--regex",
        "--include-formulas",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::FindReplace {
                            spreadsheet_id,
                            find,
                            replacement,
                            sheet_id,
                            match_case,
                            match_entire_cell,
                            search_by_regex,
                            include_formulas,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(find, "old");
            assert_eq!(replacement, "new");
            assert_eq!(sheet_id, Some(42));
            assert!(match_case);
            assert!(match_entire_cell);
            assert!(search_by_regex);
            assert!(include_formulas);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_copy_paste_accepts_source_destination_and_paste_options() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "copy-paste",
        "spreadsheet-123",
        "42",
        "--source-start-row",
        "1",
        "--source-end-row",
        "4",
        "--source-start-column",
        "0",
        "--source-end-column",
        "3",
        "--destination-sheet-id",
        "99",
        "--destination-start-row",
        "10",
        "--destination-end-row",
        "13",
        "--destination-start-column",
        "5",
        "--destination-end-column",
        "8",
        "--paste-type",
        "values",
        "--paste-orientation",
        "transposed",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::CopyPaste {
                            spreadsheet_id,
                            source_sheet_id,
                            source_start_row,
                            source_end_row,
                            source_start_column,
                            source_end_column,
                            destination_sheet_id,
                            destination_start_row,
                            destination_end_row,
                            destination_start_column,
                            destination_end_column,
                            paste_type,
                            paste_orientation,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(source_sheet_id, 42);
            assert_eq!(source_start_row, 1);
            assert_eq!(source_end_row, 4);
            assert_eq!(source_start_column, 0);
            assert_eq!(source_end_column, 3);
            assert_eq!(destination_sheet_id, 99);
            assert_eq!(destination_start_row, 10);
            assert_eq!(destination_end_row, 13);
            assert_eq!(destination_start_column, 5);
            assert_eq!(destination_end_column, 8);
            assert_eq!(paste_type, SheetsPasteType::Values);
            assert_eq!(paste_orientation, SheetsPasteOrientation::Transposed);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_copy_paste_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "copy-paste",
        "spreadsheet-123",
        "42",
        "--source-start-row",
        "-1",
        "--source-end-row",
        "4",
        "--source-start-column",
        "0",
        "--source-end-column",
        "3",
        "--destination-sheet-id",
        "99",
        "--destination-start-row",
        "10",
        "--destination-end-row",
        "13",
        "--destination-start-column",
        "5",
        "--destination-end-column",
        "8",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_cut_paste_accepts_source_range_destination_coordinate_and_paste_type() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "cut-paste",
        "spreadsheet-123",
        "42",
        "--source-start-row",
        "1",
        "--source-end-row",
        "4",
        "--source-start-column",
        "0",
        "--source-end-column",
        "3",
        "--destination-sheet-id",
        "99",
        "--destination-row",
        "10",
        "--destination-column",
        "5",
        "--paste-type",
        "values",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::CutPaste {
                            spreadsheet_id,
                            source_sheet_id,
                            source_start_row,
                            source_end_row,
                            source_start_column,
                            source_end_column,
                            destination_sheet_id,
                            destination_row,
                            destination_column,
                            paste_type,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(source_sheet_id, 42);
            assert_eq!(source_start_row, 1);
            assert_eq!(source_end_row, 4);
            assert_eq!(source_start_column, 0);
            assert_eq!(source_end_column, 3);
            assert_eq!(destination_sheet_id, 99);
            assert_eq!(destination_row, 10);
            assert_eq!(destination_column, 5);
            assert_eq!(paste_type, SheetsPasteType::Values);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_cut_paste_rejects_negative_destination_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "cut-paste",
        "spreadsheet-123",
        "42",
        "--source-start-row",
        "1",
        "--source-end-row",
        "4",
        "--source-start-column",
        "0",
        "--source-end-column",
        "3",
        "--destination-sheet-id",
        "99",
        "--destination-row",
        "-1",
        "--destination-column",
        "5",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_background_color_accepts_grid_range_and_color() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "background-color",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "#ffcc00",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::BackgroundColor {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            color,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert_eq!(color, "#ffcc00");
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_background_color_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "background-color",
        "spreadsheet-123",
        "42",
        "--start-row",
        "-1",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "#ffcc00",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_text_color_accepts_grid_range_and_color() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "text-color",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "#3366cc",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::TextColor {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            color,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert_eq!(color, "#3366cc");
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_text_color_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "text-color",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
        "#3366cc",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_font_size_accepts_grid_range_and_size() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "font-size",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--size",
        "14",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::FontSize {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            size,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert_eq!(size, 14);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_font_size_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "font-size",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
        "--size",
        "14",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_font_size_rejects_zero_size() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "font-size",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--size",
        "0",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_font_family_accepts_grid_range_and_family() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "font-family",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--family",
        "Roboto",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::FontFamily {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            family,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert_eq!(family, "Roboto");
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_font_family_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "font-family",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
        "--family",
        "Roboto",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_number_format_accepts_grid_range_type_and_pattern() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "number-format",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "10",
        "--start-column",
        "2",
        "--end-column",
        "3",
        "--type",
        "currency",
        "--pattern",
        "$#,##0.00",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::NumberFormat {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            format_type,
                            pattern,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 1);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 2);
            assert_eq!(end_column, 3);
            assert_eq!(format_type, SheetsNumberFormatType::Currency);
            assert_eq!(pattern.as_deref(), Some("$#,##0.00"));
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_number_format_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "number-format",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "3",
        "--type",
        "number",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_borders_accepts_grid_range_edges_style_and_color() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "borders",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--edge",
        "outer",
        "--edge",
        "inner",
        "--style",
        "solid-thick",
        "--color",
        "#3366cc",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Borders {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            edge,
                            style,
                            color,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert_eq!(edge, vec![SheetsBorderEdge::Outer, SheetsBorderEdge::Inner]);
            assert_eq!(style, SheetsBorderStyle::SolidThick);
            assert_eq!(color.as_deref(), Some("#3366cc"));
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_borders_defaults_to_all_solid() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "borders",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Borders {
                            edge, style, color, ..
                        },
                },
        } => {
            assert_eq!(edge, vec![SheetsBorderEdge::All]);
            assert_eq!(style, SheetsBorderStyle::Solid);
            assert_eq!(color, None);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_borders_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "borders",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_clear_format_accepts_grid_range() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "clear-format",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::ClearFormat {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_clear_format_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "clear-format",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_bold_accepts_grid_range() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "bold",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Bold {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            off,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert!(!off);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_bold_accepts_off_flag() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "bold",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--off",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command: SheetsSheetCommand::Bold { off, .. },
                },
        } => assert!(off),
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_bold_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "bold",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_italic_accepts_grid_range() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "italic",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Italic {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            off,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert!(!off);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_italic_accepts_off_flag() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "italic",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--off",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command: SheetsSheetCommand::Italic { off, .. },
                },
        } => assert!(off),
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_italic_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "italic",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_underline_accepts_grid_range() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "underline",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Underline {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            off,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert!(!off);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_underline_accepts_off_flag() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "underline",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--off",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command: SheetsSheetCommand::Underline { off, .. },
                },
        } => assert!(off),
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_underline_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "underline",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_strikethrough_accepts_grid_range() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "strikethrough",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Strikethrough {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            off,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert!(!off);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_strikethrough_accepts_off_flag() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "strikethrough",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--off",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command: SheetsSheetCommand::Strikethrough { off, .. },
                },
        } => assert!(off),
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_strikethrough_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "strikethrough",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_horizontal_align_accepts_grid_range_and_alignment() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "horizontal-align",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--alignment",
        "center",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::HorizontalAlign {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            alignment,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert_eq!(alignment, SheetsHorizontalAlignment::Center);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_horizontal_align_rejects_unknown_alignment() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "horizontal-align",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--alignment",
        "justify",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_horizontal_align_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "horizontal-align",
        "spreadsheet-123",
        "42",
        "--start-row",
        "-1",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--alignment",
        "left",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_vertical_align_accepts_grid_range_and_alignment() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "vertical-align",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--alignment",
        "middle",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::VerticalAlign {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            alignment,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert_eq!(alignment, SheetsVerticalAlignment::Middle);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_vertical_align_rejects_unknown_alignment() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "vertical-align",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--alignment",
        "center",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_vertical_align_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "vertical-align",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
        "--alignment",
        "top",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_text_wrap_accepts_grid_range_and_strategy() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "text-wrap",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--strategy",
        "wrap",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::TextWrap {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            strategy,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert_eq!(strategy, SheetsWrapStrategy::Wrap);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_text_wrap_rejects_unknown_strategy() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "text-wrap",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--strategy",
        "shrink",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_text_wrap_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "text-wrap",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
        "--strategy",
        "clip",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_text_rotation_accepts_grid_range_and_angle() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "text-rotation",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--angle",
        "45",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::TextRotation {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            angle,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert_eq!(angle, 45);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_text_rotation_rejects_out_of_range_angle() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "text-rotation",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--angle",
        "91",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_text_rotation_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "text-rotation",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
        "--angle",
        "45",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_text_direction_accepts_grid_range_and_direction() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "text-direction",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--direction",
        "right-to-left",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::TextDirection {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            direction,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert_eq!(direction, SheetsTextDirection::RightToLeft);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_text_direction_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "text-direction",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
        "--direction",
        "left-to-right",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_note_accepts_grid_range_and_note() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "note",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "Review this input",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Note {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            note,
                            clear,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert_eq!(note.as_deref(), Some("Review this input"));
            assert!(!clear);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_note_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "note",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
        "Review this input",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_note_clear_accepts_grid_range() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "note",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "1",
        "--end-column",
        "4",
        "--clear",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Note {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            note,
                            clear,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 10);
            assert_eq!(start_column, 1);
            assert_eq!(end_column, 4);
            assert_eq!(note, None);
            assert!(clear);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_note_clear_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "note",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "10",
        "--start-column",
        "-1",
        "--end-column",
        "4",
        "--clear",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_data_validation_list_accepts_values_and_options() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "data-validation-list",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "20",
        "--start-column",
        "3",
        "--end-column",
        "4",
        "--value",
        "Open",
        "--value",
        "Closed",
        "--allow-invalid",
        "--hide-dropdown",
        "--input-message",
        "Pick a status",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::DataValidationList {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            values,
                            allow_invalid,
                            hide_dropdown,
                            input_message,
                            clear,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 1);
            assert_eq!(end_row, 20);
            assert_eq!(start_column, 3);
            assert_eq!(end_column, 4);
            assert_eq!(values, vec!["Open", "Closed"]);
            assert!(allow_invalid);
            assert!(hide_dropdown);
            assert_eq!(input_message.as_deref(), Some("Pick a status"));
            assert!(!clear);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_data_validation_list_clear_accepts_grid_range() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "data-validation-list",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "20",
        "--start-column",
        "3",
        "--end-column",
        "4",
        "--clear",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::DataValidationList {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            values,
                            clear,
                            ..
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 1);
            assert_eq!(end_row, 20);
            assert_eq!(start_column, 3);
            assert_eq!(end_column, 4);
            assert!(values.is_empty());
            assert!(clear);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_data_validation_list_rejects_values_with_clear() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "data-validation-list",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "20",
        "--start-column",
        "3",
        "--end-column",
        "4",
        "--value",
        "Open",
        "--clear",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_data_validation_checkbox_accepts_options() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "data-validation-checkbox",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "20",
        "--start-column",
        "3",
        "--end-column",
        "4",
        "--checked-value",
        "Done",
        "--unchecked-value",
        "Todo",
        "--allow-invalid",
        "--input-message",
        "Mark complete",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::DataValidationCheckbox {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            checked_value,
                            unchecked_value,
                            allow_invalid,
                            input_message,
                            clear,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 1);
            assert_eq!(end_row, 20);
            assert_eq!(start_column, 3);
            assert_eq!(end_column, 4);
            assert_eq!(checked_value.as_deref(), Some("Done"));
            assert_eq!(unchecked_value.as_deref(), Some("Todo"));
            assert!(allow_invalid);
            assert_eq!(input_message.as_deref(), Some("Mark complete"));
            assert!(!clear);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_data_validation_checkbox_clear_accepts_grid_range() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "data-validation-checkbox",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "20",
        "--start-column",
        "3",
        "--end-column",
        "4",
        "--clear",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::DataValidationCheckbox {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            checked_value,
                            unchecked_value,
                            clear,
                            ..
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 1);
            assert_eq!(end_row, 20);
            assert_eq!(start_column, 3);
            assert_eq!(end_column, 4);
            assert!(checked_value.is_none());
            assert!(unchecked_value.is_none());
            assert!(clear);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_data_validation_checkbox_requires_checked_value_before_unchecked_value() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "data-validation-checkbox",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "20",
        "--start-column",
        "3",
        "--end-column",
        "4",
        "--unchecked-value",
        "Todo",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_data_validation_checkbox_rejects_options_with_clear() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "data-validation-checkbox",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "20",
        "--start-column",
        "3",
        "--end-column",
        "4",
        "--checked-value",
        "Done",
        "--clear",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_conditional_format_color_accepts_rule_options() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "conditional-format-color",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "20",
        "--start-column",
        "3",
        "--end-column",
        "4",
        "--condition",
        "number-greater",
        "--value",
        "100",
        "--background-color",
        "#ffcccc",
        "--text-color",
        "990000",
        "--index",
        "2",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::ConditionalFormatColor {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            condition,
                            value,
                            background_color,
                            text_color,
                            index,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 1);
            assert_eq!(end_row, 20);
            assert_eq!(start_column, 3);
            assert_eq!(end_column, 4);
            assert_eq!(condition, SheetsConditionalFormatCondition::NumberGreater);
            assert_eq!(value, "100");
            assert_eq!(background_color.as_deref(), Some("#ffcccc"));
            assert_eq!(text_color.as_deref(), Some("990000"));
            assert_eq!(index, 2);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_conditional_format_color_rejects_negative_index() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "conditional-format-color",
        "spreadsheet-123",
        "42",
        "--start-row",
        "1",
        "--end-row",
        "20",
        "--start-column",
        "3",
        "--end-column",
        "4",
        "--condition",
        "text-contains",
        "--value",
        "Blocked",
        "--background-color",
        "#ffeeee",
        "--index",
        "-1",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_conditional_format_update_accepts_replacement_rule_options() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "conditional-format-update",
        "spreadsheet-123",
        "42",
        "3",
        "--start-row",
        "1",
        "--end-row",
        "20",
        "--start-column",
        "3",
        "--end-column",
        "4",
        "--condition",
        "text-contains",
        "--value",
        "Blocked",
        "--background-color",
        "#ffeeee",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::ConditionalFormatUpdate {
                            spreadsheet_id,
                            sheet_id,
                            index,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            condition,
                            value,
                            background_color,
                            text_color,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(index, 3);
            assert_eq!(start_row, 1);
            assert_eq!(end_row, 20);
            assert_eq!(start_column, 3);
            assert_eq!(end_column, 4);
            assert_eq!(condition, SheetsConditionalFormatCondition::TextContains);
            assert_eq!(value, "Blocked");
            assert_eq!(background_color.as_deref(), Some("#ffeeee"));
            assert_eq!(text_color, None);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_conditional_format_update_rejects_negative_index() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "conditional-format-update",
        "spreadsheet-123",
        "42",
        "-1",
        "--start-row",
        "1",
        "--end-row",
        "20",
        "--start-column",
        "3",
        "--end-column",
        "4",
        "--condition",
        "text-contains",
        "--value",
        "Blocked",
        "--background-color",
        "#ffeeee",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_conditional_format_delete_accepts_sheet_id_and_index() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "conditional-format-delete",
        "spreadsheet-123",
        "42",
        "3",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::ConditionalFormatDelete {
                            spreadsheet_id,
                            sheet_id,
                            index,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(index, 3);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_conditional_format_delete_rejects_negative_index() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "conditional-format-delete",
        "spreadsheet-123",
        "42",
        "-1",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_conditional_format_move_accepts_sheet_id_and_indexes() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "conditional-format-move",
        "spreadsheet-123",
        "42",
        "3",
        "0",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::ConditionalFormatMove {
                            spreadsheet_id,
                            sheet_id,
                            index,
                            new_index,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(index, 3);
            assert_eq!(new_index, 0);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_conditional_format_move_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "conditional-format-move",
        "spreadsheet-123",
        "42",
        "-1",
        "0",
    ])
    .is_err());
    assert!(parse(&[
        "sheets",
        "sheet",
        "conditional-format-move",
        "spreadsheet-123",
        "42",
        "1",
        "-1",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_protect_range_accepts_grid_range_and_options() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "protect-range",
        "spreadsheet-123",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "1",
        "--start-column",
        "0",
        "--end-column",
        "5",
        "--description",
        "Lock headers",
        "--warning-only",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::ProtectRange {
                            spreadsheet_id,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            description,
                            warning_only,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 1);
            assert_eq!(start_column, 0);
            assert_eq!(end_column, 5);
            assert_eq!(description.as_deref(), Some("Lock headers"));
            assert!(warning_only);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_protect_range_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "protect-range",
        "spreadsheet-123",
        "42",
        "--start-row",
        "-1",
        "--end-row",
        "1",
        "--start-column",
        "0",
        "--end-column",
        "5",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_add_named_range_accepts_grid_range_and_optional_id() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "add-named-range",
        "spreadsheet-123",
        "42",
        "HeaderCells",
        "--start-row",
        "0",
        "--end-row",
        "1",
        "--start-column",
        "0",
        "--end-column",
        "5",
        "--named-range-id",
        "header_cells",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::AddNamedRange {
                            spreadsheet_id,
                            sheet_id,
                            name,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                            named_range_id,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(name, "HeaderCells");
            assert_eq!(start_row, 0);
            assert_eq!(end_row, 1);
            assert_eq!(start_column, 0);
            assert_eq!(end_column, 5);
            assert_eq!(named_range_id.as_deref(), Some("header_cells"));
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_add_named_range_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "add-named-range",
        "spreadsheet-123",
        "42",
        "HeaderCells",
        "--start-row",
        "-1",
        "--end-row",
        "1",
        "--start-column",
        "0",
        "--end-column",
        "5",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_delete_named_range_accepts_named_range_id() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "delete-named-range",
        "spreadsheet-123",
        "header_cells",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::DeleteNamedRange {
                            spreadsheet_id,
                            named_range_id,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(named_range_id, "header_cells");
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_update_named_range_accepts_name_and_grid_range() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "update-named-range",
        "spreadsheet-123",
        "header_cells",
        "--name",
        "HeaderRows",
        "--sheet-id",
        "42",
        "--start-row",
        "0",
        "--end-row",
        "2",
        "--start-column",
        "0",
        "--end-column",
        "5",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::UpdateNamedRange {
                            spreadsheet_id,
                            named_range_id,
                            name,
                            sheet_id,
                            start_row,
                            end_row,
                            start_column,
                            end_column,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(named_range_id, "header_cells");
            assert_eq!(name.as_deref(), Some("HeaderRows"));
            assert_eq!(sheet_id, Some(42));
            assert_eq!(start_row, Some(0));
            assert_eq!(end_row, Some(2));
            assert_eq!(start_column, Some(0));
            assert_eq!(end_column, Some(5));
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_update_named_range_rejects_negative_indexes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "update-named-range",
        "spreadsheet-123",
        "header_cells",
        "--sheet-id",
        "42",
        "--start-row",
        "-1",
        "--end-row",
        "2",
        "--start-column",
        "0",
        "--end-column",
        "5",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_update_protected_range_accepts_description_and_mode() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "update-protected-range",
        "spreadsheet-123",
        "7",
        "--description",
        "Editable warning",
        "--warning-only",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::UpdateProtectedRange {
                            spreadsheet_id,
                            protected_range_id,
                            description,
                            warning_only,
                            enforce,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(protected_range_id, 7);
            assert_eq!(description.as_deref(), Some("Editable warning"));
            assert!(warning_only);
            assert!(!enforce);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_update_protected_range_requires_one_update() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "update-protected-range",
        "spreadsheet-123",
        "7",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_update_protected_range_rejects_conflicting_modes() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "update-protected-range",
        "spreadsheet-123",
        "7",
        "--warning-only",
        "--enforce",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_unprotect_range_accepts_protected_range_id() {
    let cli = parse(&["sheets", "sheet", "unprotect-range", "spreadsheet-123", "7"]).unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::UnprotectRange {
                            spreadsheet_id,
                            protected_range_id,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(protected_range_id, 7);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_unprotect_range_rejects_negative_protected_range_id() {
    assert!(parse(&[
        "sheets",
        "sheet",
        "unprotect-range",
        "spreadsheet-123",
        "-1",
    ])
    .is_err());
}

#[test]
fn sheets_sheet_tab_color_accepts_sheet_id_and_color() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "tab-color",
        "spreadsheet-123",
        "42",
        "#3366cc",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::TabColor {
                            spreadsheet_id,
                            sheet_id,
                            color,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
            assert_eq!(color, "#3366cc");
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_clear_tab_color_accepts_sheet_id() {
    let cli = parse(&[
        "sheets",
        "sheet",
        "clear-tab-color",
        "spreadsheet-123",
        "42",
    ])
    .unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::ClearTabColor {
                            spreadsheet_id,
                            sheet_id,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_hide_accepts_sheet_id() {
    let cli = parse(&["sheets", "sheet", "hide", "spreadsheet-123", "42"]).unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Hide {
                            spreadsheet_id,
                            sheet_id,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_sheet_unhide_accepts_sheet_id() {
    let cli = parse(&["sheets", "sheet", "unhide", "spreadsheet-123", "42"]).unwrap();
    match cli.command {
        Command::Sheets {
            command:
                SheetsCommand::Sheet {
                    command:
                        SheetsSheetCommand::Unhide {
                            spreadsheet_id,
                            sheet_id,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(sheet_id, 42);
        }
        _ => panic!("unexpected parse result"),
    }
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
fn sheets_batch_update_requires_requests() {
    assert!(parse(&["sheets", "batch-update", "spreadsheet-123"]).is_err());
}

#[test]
fn sheets_batch_update_help_explains_request_shape() {
    let help = help(&["sheets", "batch-update", "--help"]);

    assert!(
        help.contains("--requests reads the full Google Sheets spreadsheets.batchUpdate JSON body")
    );
    assert!(help.contains("not only the requests array"));
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
