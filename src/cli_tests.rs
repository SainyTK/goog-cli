use clap::Parser;

use crate::auth::config::OAuthAppType;
use crate::cli::{
    AuthCommand, AuthMappingsCommand, Cli, Command, DocsBreakCommand, DocsCommand,
    DocsFooterCommand, DocsFootnoteCommand, DocsHeaderCommand, DocsImageCommand, DocsListCommand,
    DocsListType, DocsMapType, DocsNamedRangeCommand, DocsStyleCommand, DocsTableCommand,
    DocsTextCommand, DriveCommand, DriveListType, MailCommand, SheetsCommand,
    SheetsInsertDataOption, SheetsValueInputOption, SheetsValueRenderOption, SheetsValuesCommand,
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
fn drive_ls_with_flags() {
    let cli = parse(&[
        "drive",
        "ls",
        "--limit",
        "100",
        "--all",
        "--type",
        "files",
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
                    type_,
                    folder,
                    json,
                },
        } => {
            assert_eq!(limit, Some(100));
            assert!(all);
            assert_eq!(type_, DriveListType::Files);
            assert_eq!(folder.as_deref(), Some("folder123"));
            assert!(json);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn drive_ls_defaults_to_items() {
    let cli = parse(&["drive", "ls"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Drive {
            command: DriveCommand::Ls {
                limit: None,
                all: false,
                type_: DriveListType::Items,
                folder: None,
                json: false
            }
        }
    ));
}

#[test]
fn drive_ls_help_describes_limit_cap_and_type() {
    let text = help(&["drive", "ls", "--help"]);

    assert!(text.contains("List all items across all pages"));
    assert!(text.contains("Caps at --limit when both are given"));
    assert!(text.contains("--type <TYPE>"));
    assert!(text.contains("[default: items]"));
    assert!(text.contains("Items use browse row fields"));
    assert!(!text.contains("Fetch all items across all pages"));
}

#[test]
fn drive_rejects_removed_listing_commands() {
    assert!(parse(&["drive", "list"]).is_err());
    assert!(parse(&["drive", "folder", "list"]).is_err());
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
            command:
                DocsCommand::Map {
                    document_id,
                    type_,
                    index,
                    entry,
                    page,
                    line,
                    heading,
                    json,
                },
        } => {
            assert_eq!(document_id, "document-123");
            assert_eq!(type_, DocsMapType::All);
            assert_eq!(index, None);
            assert_eq!(entry, None);
            assert_eq!(page, None);
            assert_eq!(line, None);
            assert_eq!(heading, None);
            assert!(json);
        }
        _ => panic!("unexpected parse result"),
    }

    let cli = parse(&["docs", "map", "document-123", "--type", "images"]).unwrap();
    match cli.command {
        Command::Docs {
            command: DocsCommand::Map { type_, .. },
        } => assert_eq!(type_, DocsMapType::Images),
        _ => panic!("unexpected parse result"),
    }

    let cli = parse(&["docs", "map", "document-123", "--type", "tables"]).unwrap();
    match cli.command {
        Command::Docs {
            command: DocsCommand::Map { type_, .. },
        } => assert_eq!(type_, DocsMapType::Tables),
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn docs_map_rejects_removed_list_object_verbs() {
    assert!(parse(&["docs", "list-images", "document-123", "--json"]).is_err());
    assert!(parse(&["docs", "list-tables", "document-123", "--json"]).is_err());
}

#[test]
fn docs_search_text_with_document_id_text_and_json_flag() {
    let cli = parse(&["docs", "text", "search", "document-123", "Plan", "--json"]).unwrap();
    match cli.command {
        Command::Docs {
            command:
                DocsCommand::Text {
                    command:
                        DocsTextCommand::Search {
                            document_id,
                            text,
                            json,
                        },
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
fn docs_map_accepts_content_location_selectors() {
    let by_index = parse(&["docs", "map", "document-123", "--index", "44"]).unwrap();
    match by_index.command {
        Command::Docs {
            command:
                DocsCommand::Map {
                    document_id,
                    type_,
                    index,
                    entry,
                    page,
                    line,
                    heading,
                    json,
                },
        } => {
            assert_eq!(document_id, "document-123");
            assert_eq!(type_, DocsMapType::All);
            assert_eq!(index, Some(44));
            assert_eq!(entry, None);
            assert_eq!(page, None);
            assert_eq!(line, None);
            assert_eq!(heading, None);
            assert!(!json);
        }
        _ => panic!("unexpected parse result"),
    }

    let by_entry = parse(&["docs", "map", "document-123", "--entry", "44"]).unwrap();
    match by_entry.command {
        Command::Docs {
            command: DocsCommand::Map { index, entry, .. },
        } => {
            assert_eq!(index, None);
            assert_eq!(entry, Some(44));
        }
        _ => panic!("unexpected parse result"),
    }

    assert!(parse(&["docs", "map", "document-123", "--page", "2", "--line", "1",]).is_ok());
    assert!(parse(&[
        "docs",
        "map",
        "document-123",
        "--heading",
        "Appendix",
        "--json",
    ])
    .is_ok());
    assert!(parse(&["docs", "get-content", "document-123", "--entry", "44"]).is_err());
}

#[test]
fn docs_insert_text_parses_location_and_write_options() {
    let cli = parse(&[
        "docs",
        "text",
        "insert",
        "document-123",
        "Hello",
        "--at",
        "page:2,line:1",
        "--dry-run",
        "--json",
        "--required-revision-id",
        "rev-123",
    ])
    .unwrap();

    let Command::Docs { command } = cli.command else {
        panic!("unexpected parse result");
    };
    let DocsCommand::Text {
        command:
            DocsTextCommand::Insert {
                document_id,
                text,
                at,
                dry_run,
                json,
                required_revision_id,
            },
    } = command
    else {
        panic!("unexpected parse result");
    };

    assert_eq!(document_id, "document-123");
    assert_eq!(text, "Hello");
    assert_eq!(at, "page:2,line:1");
    assert!(dry_run);
    assert!(json);
    assert_eq!(required_revision_id.as_deref(), Some("rev-123"));
}

#[test]
fn docs_insert_commands_accept_at_selector() {
    let Command::Docs {
        command:
            DocsCommand::Text {
                command: DocsTextCommand::Insert { at, .. },
            },
    } = parse(&[
        "docs",
        "text",
        "insert",
        "document-123",
        "Hello",
        "--at",
        "after-text:quarterly plan",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(at, "after-text:quarterly plan");

    let Command::Docs {
        command:
            DocsCommand::Table {
                command: DocsTableCommand::Insert { at, .. },
            },
    } = parse(&[
        "docs",
        "table",
        "insert",
        "document-123",
        "--rows",
        "2",
        "--columns",
        "2",
        "--at",
        "page:1,line:3",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(at, "page:1,line:3");
}

#[test]
fn docs_insert_text_rejects_legacy_heading_selector() {
    assert!(parse(&[
        "docs",
        "text",
        "insert",
        "document-123",
        "Hello",
        "--heading",
        "Summary",
    ])
    .is_err());
}

#[test]
fn docs_insert_commands_reject_legacy_selector_flags() {
    assert!(parse(&[
        "docs",
        "text",
        "insert",
        "document-123",
        "Hello",
        "--page",
        "2",
        "--line",
        "1",
    ])
    .is_err());
    assert!(parse(&[
        "docs",
        "footnote",
        "insert",
        "document-123",
        "--after-text",
        "quarterly plan",
    ])
    .is_err());
}

#[test]
fn docs_replace_text_parses_match_and_write_options() {
    let cli = parse(&[
        "docs",
        "text",
        "replace",
        "document-123",
        "--find",
        "old",
        "--replace",
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
    let DocsCommand::Text {
        command:
            DocsTextCommand::Replace {
                document_id,
                old_text,
                new_text,
                match_number,
                all,
                dry_run,
                json,
                required_revision_id,
            },
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
fn docs_replace_text_rejects_order_sensitive_text_positionals() {
    assert!(parse(&["docs", "text", "replace", "document-123", "old", "new"]).is_err());
}

#[test]
fn docs_flat_text_commands_are_removed() {
    assert!(parse(&["docs", "search-text", "document-123", "Plan"]).is_err());
    assert!(parse(&["docs", "insert-text", "document-123", "Hello"]).is_err());
    assert!(parse(&[
        "docs",
        "replace-text",
        "document-123",
        "--find",
        "old",
        "--replace",
        "new",
    ])
    .is_err());
}

#[test]
fn docs_flat_table_commands_are_removed() {
    assert!(parse(&[
        "docs",
        "insert-table",
        "document-123",
        "--rows",
        "2",
        "--columns",
        "2",
        "--at",
        "index:1",
    ])
    .is_err());
    assert!(parse(&[
        "docs",
        "edit-table",
        "document-123",
        "--table-id",
        "table-3",
        "--data",
        "table.csv",
    ])
    .is_err());
}

#[test]
fn docs_flat_image_commands_are_removed() {
    assert!(parse(&[
        "docs",
        "insert-image",
        "document-123",
        "https://example.test/image.png",
        "--at",
        "index:1",
    ])
    .is_err());
}

#[test]
fn docs_flat_footnote_commands_are_removed() {
    assert!(parse(&["docs", "insert-footnote", "document-123", "--at", "index:1",]).is_err());
}

#[test]
fn docs_flat_break_commands_are_removed() {
    assert!(parse(&[
        "docs",
        "insert-page-break",
        "document-123",
        "--at",
        "index:1"
    ])
    .is_err());
    assert!(parse(&[
        "docs",
        "insert-section-break",
        "document-123",
        "--section-type",
        "next-page",
        "--at",
        "index:1",
    ])
    .is_err());
}

#[test]
fn docs_flat_header_command_is_removed() {
    assert!(parse(&["docs", "create-header", "document-123"]).is_err());
}

#[test]
fn docs_flat_footer_command_is_removed() {
    assert!(parse(&["docs", "create-footer", "document-123"]).is_err());
}

#[test]
fn docs_new_high_level_editing_commands_parse() {
    let Command::Docs {
        command:
            DocsCommand::Image {
                command:
                    DocsImageCommand::Insert {
                        image_uri,
                        at,
                        dry_run,
                        json,
                        ..
                    },
            },
    } = parse(&[
        "docs",
        "image",
        "insert",
        "document-123",
        "https://example.test/image.png",
        "--at",
        "page:2,line:1",
        "--dry-run",
        "--json",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(image_uri, "https://example.test/image.png");
    assert_eq!(at, "page:2,line:1");
    assert!(dry_run);
    assert!(json);

    let Command::Docs {
        command:
            DocsCommand::Footnote {
                command:
                    DocsFootnoteCommand::Insert {
                        at, dry_run, json, ..
                    },
            },
    } = parse(&[
        "docs",
        "footnote",
        "insert",
        "document-123",
        "--at",
        "after-text:quarterly plan",
        "--dry-run",
        "--json",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(at, "after-text:quarterly plan");
    assert!(dry_run);
    assert!(json);

    let Command::Docs {
        command:
            DocsCommand::Break {
                command:
                    DocsBreakCommand::Page {
                        at, dry_run, json, ..
                    },
            },
    } = parse(&[
        "docs",
        "break",
        "page",
        "document-123",
        "--at",
        "heading:Summary",
        "--dry-run",
        "--json",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(at, "heading:Summary");
    assert!(dry_run);
    assert!(json);

    let Command::Docs {
        command:
            DocsCommand::Break {
                command:
                    DocsBreakCommand::Section {
                        section_type, at, ..
                    },
            },
    } = parse(&[
        "docs",
        "break",
        "section",
        "document-123",
        "--section-type",
        "continuous",
        "--at",
        "heading:Appendix",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(section_type, crate::cli::DocsSectionBreakType::Continuous);
    assert_eq!(at, "heading:Appendix");

    let Command::Docs {
        command:
            DocsCommand::Header {
                command:
                    DocsHeaderCommand::Create {
                        document_id,
                        dry_run,
                        json,
                        ..
                    },
            },
    } = parse(&[
        "docs",
        "header",
        "create",
        "document-123",
        "--dry-run",
        "--json",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(document_id, "document-123");
    assert!(dry_run);
    assert!(json);

    let Command::Docs {
        command:
            DocsCommand::Footer {
                command:
                    DocsFooterCommand::Create {
                        document_id,
                        dry_run,
                        json,
                        ..
                    },
            },
    } = parse(&[
        "docs",
        "footer",
        "create",
        "document-123",
        "--dry-run",
        "--json",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(document_id, "document-123");
    assert!(dry_run);
    assert!(json);

    let Command::Docs {
        command:
            DocsCommand::List {
                command:
                    DocsListCommand::Apply {
                        list_type, entry, ..
                    },
            },
    } = parse(&[
        "docs",
        "list",
        "apply",
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
                DocsCommand::List {
                    command:
                        DocsListCommand::Apply {
                            list_type,
                            from_index,
                            to_index,
                            ..
                        },
                },
        } = parse(&[
            "docs",
            "list",
            "apply",
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
            DocsCommand::List {
                command:
                    DocsListCommand::Apply {
                        preset,
                        list_type,
                        page,
                        line,
                        ..
                    },
            },
    } = parse(&[
        "docs",
        "list",
        "apply",
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
            DocsCommand::List {
                command:
                    DocsListCommand::Apply {
                        text,
                        match_number,
                        list_type,
                        ..
                    },
            },
    } = parse(&[
        "docs",
        "list",
        "apply",
        "document-123",
        "--text",
        "Plan",
        "--match",
        "2",
        "--type",
        "numbered",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(text.as_deref(), Some("Plan"));
    assert_eq!(match_number, Some(2));
    assert_eq!(list_type, Some(DocsListType::Numbered));

    let Command::Docs {
        command:
            DocsCommand::Style {
                command:
                    DocsStyleCommand::Apply {
                        style_json,
                        from_index,
                        to_index,
                        ..
                    },
            },
    } = parse(&[
        "docs",
        "style",
        "apply",
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
            DocsCommand::Table {
                command:
                    DocsTableCommand::Insert {
                        data,
                        at,
                        dry_run,
                        json,
                        ..
                    },
            },
    } = parse(&[
        "docs",
        "table",
        "insert",
        "document-123",
        "--data",
        "table.csv",
        "--at",
        "page:2,line:1",
        "--dry-run",
        "--json",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(data.as_deref(), Some("table.csv"));
    assert_eq!(at, "page:2,line:1");
    assert!(dry_run);
    assert!(json);

    let Command::Docs {
        command:
            DocsCommand::Table {
                command:
                    DocsTableCommand::Edit {
                        table_id,
                        data,
                        dry_run,
                        json,
                        ..
                    },
            },
    } = parse(&[
        "docs",
        "table",
        "edit",
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
            DocsCommand::Table {
                command:
                    DocsTableCommand::Insert {
                        rows,
                        columns,
                        no_auto_style,
                        ..
                    },
            },
    } = parse(&[
        "docs",
        "table",
        "insert",
        "document-123",
        "--rows",
        "2",
        "--columns",
        "3",
        "--at",
        "index:44",
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
            DocsCommand::Style {
                command:
                    DocsStyleCommand::Apply {
                        heading,
                        no_cached_style,
                        ..
                    },
            },
    } = parse(&[
        "docs",
        "style",
        "apply",
        "document-123",
        "--entry",
        "2",
        "--paragraph-style",
        "HEADING_2",
        "--no-cached-style",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(heading.as_deref(), Some("HEADING_2"));
    assert!(no_cached_style);

    let Command::Docs {
        command:
            DocsCommand::List {
                command:
                    DocsListCommand::Apply {
                        entry,
                        no_cached_style,
                        ..
                    },
            },
    } = parse(&[
        "docs",
        "list",
        "apply",
        "document-123",
        "--entry",
        "2",
        "--no-cached-style",
        "--preset",
        "BULLET_DISC_CIRCLE_SQUARE",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };
    assert_eq!(entry, Some(2));
    assert!(no_cached_style);
}

#[test]
fn docs_apply_styles_uses_paragraph_style_flag() {
    let Command::Docs {
        command:
            DocsCommand::Style {
                command: DocsStyleCommand::Apply { heading, entry, .. },
            },
    } = parse(&[
        "docs",
        "style",
        "apply",
        "document-123",
        "--entry",
        "2",
        "--paragraph-style",
        "HEADING_1",
    ])
    .unwrap()
    .command
    else {
        panic!("unexpected parse result");
    };

    assert_eq!(entry, Some(2));
    assert_eq!(heading.as_deref(), Some("HEADING_1"));
    assert!(parse(&[
        "docs",
        "style",
        "apply",
        "document-123",
        "--entry",
        "2",
        "--heading",
        "HEADING_1",
    ])
    .is_err());
}

#[test]
fn docs_apply_commands_reject_no_auto_style() {
    assert!(parse(&[
        "docs",
        "style",
        "apply",
        "document-123",
        "--entry",
        "2",
        "--no-auto-style"
    ])
    .is_err());

    assert!(parse(&[
        "docs",
        "list",
        "apply",
        "document-123",
        "--entry",
        "2",
        "--type",
        "bullet",
        "--no-auto-style"
    ])
    .is_err());
}

#[test]
fn docs_style_template_bypass_help_uses_distinct_flags() {
    let insert_table_help = help(&["docs", "table", "insert", "--help"]);
    assert!(insert_table_help.contains("--no-auto-style"));
    assert!(!insert_table_help.contains("--no-cached-style"));

    let style_apply_help = help(&["docs", "style", "apply", "--help"]);
    assert!(style_apply_help.contains("--no-cached-style"));
    assert!(!style_apply_help.contains("--no-auto-style"));

    let apply_list_help = help(&["docs", "list", "apply", "--help"]);
    assert!(apply_list_help.contains("--no-cached-style"));
    assert!(!apply_list_help.contains("--no-auto-style"));
}

#[test]
fn docs_style_template_parse() {
    let cli = parse(&["docs", "style", "template", "document-123", "--json"]).unwrap();

    match cli.command {
        Command::Docs {
            command:
                DocsCommand::Style {
                    command: DocsStyleCommand::Template { document_id, json },
                },
        } => {
            assert_eq!(document_id, "document-123");
            assert!(json);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn docs_show_style_template_is_removed() {
    assert!(parse(&["docs", "show-style-template", "document-123"]).is_err());
    assert!(parse(&["docs", "style-template", "document-123"]).is_err());
    assert!(parse(&["docs", "apply-styles", "document-123", "--entry", "1"]).is_err());
    assert!(parse(&["docs", "apply-list", "document-123", "--entry", "1"]).is_err());
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

    assert!(help.contains("Read a raw Google Docs Document"));
    assert!(help.contains("Document ID or URL to read"));
    assert!(help.contains("Emits the Google Docs API Document JSON unchanged."));
    assert!(help.contains("body.content as ordered structural elements"));
    assert!(help.contains("paragraph.elements[].textRun.content"));
    assert!(help.contains("--include-tabs-content"));
    assert!(!help.contains("Fetch a raw Google Docs Document"));
    assert!(!help.contains("Google Docs Document ID or URL to fetch"));
}

#[test]
fn docs_help_uses_short_document_id_wording() {
    for (args, expected) in [
        (&["docs", "map", "--help"][..], "Document ID or URL to map"),
        (&["docs", "get", "--help"][..], "Document ID or URL to read"),
        (
            &["docs", "batch-update", "--help"][..],
            "Document ID or URL to update",
        ),
        (
            &["docs", "style", "template", "--help"][..],
            "Document ID whose cached style template to read",
        ),
        (
            &["docs", "text", "search", "--help"][..],
            "Document ID or URL to search",
        ),
        (
            &["docs", "text", "insert", "--help"][..],
            "Document ID or URL to update",
        ),
        (
            &["docs", "table", "edit", "--help"][..],
            "Document ID or URL to update",
        ),
        (
            &["docs", "list", "apply", "--help"][..],
            "Document ID or URL to update",
        ),
    ] {
        let help = help(args);
        assert!(
            help.contains(expected),
            "{args:?} did not contain {expected}"
        );
        assert!(!help.contains("Google Docs Document ID"));
    }
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
fn docs_selector_help_explains_exactly_one_selector_rule() {
    let map_help = help(&["docs", "map", "--help"]);
    assert!(map_help.contains("Provide exactly one content selector."));
    assert!(map_help.contains("--page P --line L"));
    assert!(map_help.contains("--heading TEXT"));

    for command in [
        &["docs", "text", "insert", "--help"][..],
        &["docs", "image", "insert", "--help"],
        &["docs", "break", "page", "--help"],
        &["docs", "break", "section", "--help"],
        &["docs", "footnote", "insert", "--help"],
        &["docs", "table", "insert", "--help"],
    ] {
        let command_help = help(command);
        assert!(command_help.contains("Provide exactly one insert location selector"));
        assert!(command_help.contains("--at <SELECTOR>"));
        assert!(command_help.contains("--at heading:TEXT"));
        assert!(command_help.contains("--at before-text:TEXT"));
        assert!(!command_help.contains("--after-heading <AFTER_HEADING>"));
    }

    for command in [
        &["docs", "style", "apply", "--help"][..],
        &["docs", "list", "apply", "--help"],
    ] {
        let command_help = help(command);
        assert!(command_help.contains("Provide exactly one range selector."));
        assert!(command_help.contains("--from-index START --to-index END"));
        assert!(command_help.contains("--text TEXT with optional --match N"));
    }

    let create_named_range_help = help(&["docs", "named-range", "create", "--help"]);
    assert!(create_named_range_help.contains("Provide exactly one range selector."));
    assert!(create_named_range_help.contains("--from-index START --to-index END"));
    assert!(create_named_range_help.contains("--text TEXT with optional --match N"));

    let apply_styles_help = help(&["docs", "style", "apply", "--help"]);
    assert!(apply_styles_help.contains("--paragraph-style <PARAGRAPH_STYLE>"));
    assert!(!apply_styles_help.contains("--heading <HEADING>"));
}

#[test]
fn docs_named_range_create_accepts_range_selector() {
    let cli = parse(&[
        "docs",
        "named-range",
        "create",
        "document-123",
        "highlights",
        "--text",
        "quarterly plan",
        "--match",
        "2",
        "--dry-run",
    ])
    .unwrap();

    match cli.command {
        Command::Docs {
            command:
                DocsCommand::NamedRange {
                    command:
                        DocsNamedRangeCommand::Create {
                            document_id,
                            name,
                            text,
                            match_number,
                            dry_run,
                            ..
                        },
                },
        } => {
            assert_eq!(document_id, "document-123");
            assert_eq!(name, "highlights");
            assert_eq!(text.as_deref(), Some("quarterly plan"));
            assert_eq!(match_number, Some(2));
            assert!(dry_run);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn docs_named_range_delete_accepts_name_or_id_selector() {
    let cli = parse(&[
        "docs",
        "named-range",
        "delete",
        "document-123",
        "--name",
        "highlights",
        "--json",
    ])
    .unwrap();

    match cli.command {
        Command::Docs {
            command:
                DocsCommand::NamedRange {
                    command:
                        DocsNamedRangeCommand::Delete {
                            document_id,
                            name,
                            named_range_id,
                            json,
                            ..
                        },
                },
        } => {
            assert_eq!(document_id, "document-123");
            assert_eq!(name.as_deref(), Some("highlights"));
            assert!(named_range_id.is_none());
            assert!(json);
        }
        _ => panic!("unexpected parse result"),
    }

    let cli = parse(&[
        "docs",
        "named-range",
        "delete",
        "document-123",
        "--named-range-id",
        "named-range-123",
    ])
    .unwrap();

    match cli.command {
        Command::Docs {
            command:
                DocsCommand::NamedRange {
                    command: DocsNamedRangeCommand::Delete { named_range_id, .. },
                },
        } => {
            assert_eq!(named_range_id.as_deref(), Some("named-range-123"));
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn docs_flat_named_range_commands_are_removed() {
    assert!(parse(&[
        "docs",
        "create-named-range",
        "document-123",
        "highlights",
        "--text",
        "quarterly plan",
    ])
    .is_err());
    assert!(parse(&[
        "docs",
        "delete-named-range",
        "document-123",
        "--name",
        "highlights",
    ])
    .is_err());
}

#[test]
fn docs_create_footnote_is_not_a_command() {
    assert!(parse(&["docs", "create-footnote", "document-123", "--index", "1"]).is_err());
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
            command: MailCommand::List { query, limit, json },
        } => {
            assert!(query.is_none());
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
            command: MailCommand::List { query, limit, json },
        } => {
            assert!(query.is_none());
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
fn mail_list_accepts_query_limit_and_json() {
    let cli = parse(&[
        "mail",
        "list",
        "from:alice@example.com",
        "--limit",
        "25",
        "--json",
    ])
    .unwrap();
    match cli.command {
        Command::Mail {
            command: MailCommand::List { query, limit, json },
        } => {
            assert_eq!(query.as_deref(), Some("from:alice@example.com"));
            assert_eq!(limit, Some(25));
            assert!(json);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn mail_search_is_not_a_command() {
    assert!(parse(&["mail", "search"]).is_err());
}

#[test]
fn mail_list_with_query_does_not_accept_all() {
    assert!(parse(&["mail", "list", "has:attachment", "--all"]).is_err());
}

#[test]
fn mail_help_uses_plain_gmail_language() {
    let mail_help = help(&["mail", "--help"]);
    let read_help = help(&["mail", "read", "--help"]);
    let download_help = help(&["mail", "download", "--help"]);
    let draft_help = help(&["mail", "draft", "--help"]);

    assert!(mail_help.contains("Interact with Gmail"));
    assert!(read_help.contains("Read a Gmail message"));
    assert!(read_help.contains("Gmail message ID or URL to read"));
    assert!(download_help.contains("Download a Gmail attachment"));
    assert!(download_help.contains("Destination path (defaults to attachment filename)"));
    assert!(draft_help.contains("Create or edit a Gmail draft message"));
    assert!(draft_help.contains("Gmail draft ID or URL to update"));
    assert!(draft_help.contains("Emit JSON instead of human-readable output"));

    for rendered in [mail_help, read_help, download_help, draft_help] {
        assert!(!rendered.contains("GoogleMail"));
        assert!(!rendered.contains("Fetch a"));
        assert!(!rendered.contains("Manage GoogleMail"));
    }
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
                MailCommand::Download {
                    message_id,
                    attachment_id,
                    output,
                },
        } => {
            assert_eq!(message_id, "message-123");
            assert_eq!(attachment_id.as_deref(), Some("attachment-456"));
            assert_eq!(output.as_deref(), Some("report.pdf"));
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn mail_attachment_download_output_and_attachment_id_are_optional() {
    let cli = parse(&["mail", "download", "message-123", "attachment-456"]).unwrap();
    match cli.command {
        Command::Mail {
            command:
                MailCommand::Download {
                    message_id,
                    attachment_id,
                    output,
                },
        } => {
            assert_eq!(message_id, "message-123");
            assert_eq!(attachment_id.as_deref(), Some("attachment-456"));
            assert!(output.is_none());
        }
        _ => panic!("unexpected parse result"),
    }

    let cli = parse(&["mail", "download", "message-123"]).unwrap();
    match cli.command {
        Command::Mail {
            command:
                MailCommand::Download {
                    message_id,
                    attachment_id,
                    output,
                },
        } => {
            assert_eq!(message_id, "message-123");
            assert!(attachment_id.is_none());
            assert!(output.is_none());
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn mail_attachment_group_is_not_a_command() {
    assert!(parse(&[
        "mail",
        "attachment",
        "download",
        "message-123",
        "attachment-456"
    ])
    .is_err());
}

#[test]
fn mail_draft_create_with_body_flags() {
    let cli = parse(&[
        "mail",
        "draft",
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
                    draft_id,
                    to,
                    cc,
                    bcc,
                    subject,
                    body,
                    attachment,
                    json,
                },
        } => {
            assert!(draft_id.is_none());
            assert_eq!(to, ["alice@example.com", "bob@example.com"]);
            assert_eq!(cc, ["carol@example.com"]);
            assert_eq!(bcc, ["dave@example.com"]);
            assert_eq!(subject, "Draft subject");
            assert_eq!(body.as_deref(), Some("Hello from goog"));
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
                    draft_id,
                    attachment,
                    json,
                    ..
                },
        } => {
            assert!(draft_id.is_none());
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
                    draft_id,
                    to,
                    cc,
                    bcc,
                    subject,
                    body,
                    attachment,
                    json,
                },
        } => {
            assert_eq!(draft_id.as_deref(), Some("draft-123"));
            assert_eq!(to, ["alice@example.com"]);
            assert!(cc.is_empty());
            assert!(bcc.is_empty());
            assert_eq!(subject, "Updated draft");
            assert_eq!(body.as_deref(), Some("Updated body"));
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
fn mail_draft_create_and_edit_subcommands_are_removed() {
    assert!(parse(&[
        "mail",
        "draft",
        "create",
        "--to",
        "alice@example.com",
        "--subject",
        "Draft subject",
    ])
    .is_err());
    assert!(parse(&[
        "mail",
        "draft",
        "edit",
        "draft-123",
        "--to",
        "alice@example.com",
        "--subject",
        "Draft subject",
    ])
    .is_err());
}

#[test]
fn mail_draft_body_file_flag_is_removed() {
    assert!(parse(&[
        "mail",
        "draft",
        "--to",
        "alice@example.com",
        "--subject",
        "Draft subject",
        "--body-file",
        "message.txt",
    ])
    .is_err());
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
        "--range",
        "Sheet1!A1:B2",
        "--range",
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
                            ranges,
                            value_render_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range.as_deref(), Some("Sheet1!A1:B2"));
            assert!(ranges.is_empty());
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
                            ranges,
                            value_render_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range.as_deref(), Some("Sheet1!C1:C3"));
            assert!(ranges.is_empty());
            assert_eq!(value_render_option, SheetsValueRenderOption::Formula);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_get_accepts_repeated_ranges_and_render_option() {
    let cli = parse(&[
        "sheets",
        "values",
        "get",
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
                        SheetsValuesCommand::Get {
                            spreadsheet_id,
                            range,
                            ranges,
                            value_render_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert!(range.is_none());
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
fn sheets_values_get_requires_range() {
    assert!(parse(&["sheets", "values", "get", "spreadsheet-123"]).is_err());
}

#[test]
fn sheets_values_batch_get_command_is_removed() {
    assert!(parse(&[
        "sheets",
        "values",
        "batch-get",
        "spreadsheet-123",
        "--range",
        "Sheet1!A1:B2",
    ])
    .is_err());
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
            assert_eq!(range.as_deref(), Some("Sheet1!A1:B2"));
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
fn sheets_values_update_without_range_accepts_batch_update_body() {
    let cli = parse(&[
        "sheets",
        "values",
        "update",
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
                        SheetsValuesCommand::Update {
                            spreadsheet_id,
                            range,
                            values,
                            value_input_option,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert!(range.is_none());
            assert_eq!(values, "values.json");
            assert_eq!(value_input_option, SheetsValueInputOption::UserEntered);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_batch_update_command_is_removed() {
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
                            ranges,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range.as_deref(), Some("Sheet1!A1:B2"));
            assert!(ranges.is_empty());
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_clear_accepts_repeated_ranges() {
    let cli = parse(&[
        "sheets",
        "values",
        "clear",
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
                        SheetsValuesCommand::Clear {
                            spreadsheet_id,
                            range,
                            ranges,
                        },
                },
        } => {
            assert_eq!(spreadsheet_id, "spreadsheet-123");
            assert_eq!(range, None);
            assert_eq!(ranges, vec!["Sheet1!A1:B2", "Summary!A:A"]);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn sheets_values_clear_requires_range() {
    assert!(parse(&["sheets", "values", "clear", "spreadsheet-123"]).is_err());
}

#[test]
fn sheets_values_batch_clear_command_is_removed() {
    assert!(parse(&[
        "sheets",
        "values",
        "batch-clear",
        "spreadsheet-123",
        "--range",
        "Sheet1!A1:B2",
    ])
    .is_err());
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
fn sheets_help_uses_short_spreadsheet_id_wording() {
    for args in [
        &["sheets", "get", "--help"][..],
        &["sheets", "batch-update", "--help"],
        &["sheets", "values", "get", "--help"],
        &["sheets", "values", "update", "--help"],
        &["sheets", "values", "append", "--help"],
        &["sheets", "values", "clear", "--help"],
    ] {
        let help = help(args);

        assert!(help.contains("Spreadsheet ID"));
        assert!(!help.contains("Google Sheets Spreadsheet ID"));
    }

    let get_help = help(&["sheets", "get", "--help"]);
    assert!(get_help.contains("Read raw spreadsheet metadata"));
    assert!(get_help.contains("Spreadsheet ID to read"));
    assert!(get_help.contains("Limit returned grid data to an A1 range"));
    assert!(!get_help.contains("Fetch raw Google Sheets Spreadsheet metadata"));
    assert!(!get_help.contains("Google Sheets A1 Range"));

    let values_get_help = help(&["sheets", "values", "get", "--help"]);
    assert!(values_get_help.contains("Read raw sheet values"));
    assert!(values_get_help.contains("Spreadsheet ID to read"));
    assert!(values_get_help.contains("Single A1 range to read"));
    assert!(values_get_help.contains("A1 range to read. Repeat for multiple ranges"));
    assert!(!values_get_help.contains("Fetch raw Google Sheets values"));
    assert!(!values_get_help.contains("Single Google Sheets A1 Range to fetch"));
    assert!(!values_get_help.contains("Google Sheets A1 Range to fetch"));

    for args in [
        &["sheets", "values", "get", "--help"][..],
        &["sheets", "values", "update", "--help"],
        &["sheets", "values", "append", "--help"],
        &["sheets", "values", "clear", "--help"],
    ] {
        let help = help(args);

        assert!(!help.contains("Google Sheets A1 range"));
        assert!(!help.contains("Google Sheets A1 Range"));
    }

    let batch_update_help = help(&["sheets", "batch-update", "--help"]);
    assert!(batch_update_help.contains("Apply a raw structural spreadsheet update request body"));
    assert!(!batch_update_help
        .contains("Apply a raw Google Sheets structural Batch Update request body"));

    let values_update_help = help(&["sheets", "values", "update", "--help"]);
    assert!(values_update_help.contains("Update sheet values"));
    assert!(values_update_help.contains("Path to a ValueRange JSON request body"));
    assert!(!values_update_help.contains("Update a Google Sheets ValueRange"));
    assert!(!values_update_help.contains("Path to a Google ValueRange JSON request body"));

    let values_append_help = help(&["sheets", "values", "append", "--help"]);
    assert!(values_append_help.contains("Path to a ValueRange JSON request body"));
    assert!(values_append_help.contains("How appended data should be inserted"));
    assert!(!values_append_help.contains("Path to a Google ValueRange JSON request body"));
    assert!(!values_append_help.contains("How Google Sheets should insert appended data"));
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
    let cli = parse(&["drive", "ls", "--account", "me@example.com"]).unwrap();
    assert_eq!(cli.account.as_deref(), Some("me@example.com"));
}

#[test]
fn global_quiet_flag() {
    let cli = parse(&["--quiet", "auth", "list"]).unwrap();
    assert!(cli.quiet);
}
