use super::list::{render_ndjson, render_table, rows_from_config, AccountRow};

#[test]
fn rows_marks_the_active_account() {
    let accounts = vec![
        "alice@example.com".to_string(),
        "bob@example.com".to_string(),
    ];
    let rows = rows_from_config(&accounts, Some("bob@example.com"));
    assert_eq!(
        rows,
        vec![
            AccountRow {
                email: "alice@example.com".into(),
                active: false
            },
            AccountRow {
                email: "bob@example.com".into(),
                active: true
            },
        ]
    );
}

#[test]
fn rows_marks_nothing_when_no_active_account() {
    let accounts = vec!["alice@example.com".to_string()];
    let rows = rows_from_config(&accounts, None);
    assert!(!rows[0].active);
}

#[test]
fn table_marks_active_with_asterisk() {
    let rows = vec![
        AccountRow {
            email: "alice@example.com".into(),
            active: false,
        },
        AccountRow {
            email: "bob@example.com".into(),
            active: true,
        },
    ];
    let table = render_table(&rows);
    assert!(table.contains("alice@example.com"));
    assert!(table.contains("bob@example.com"));
    let bob_line = table.lines().find(|l| l.contains("bob")).unwrap();
    assert!(bob_line.starts_with("*"));
    let alice_line = table.lines().find(|l| l.contains("alice")).unwrap();
    assert!(alice_line.starts_with(" "));
}

#[test]
fn table_shows_helpful_message_when_no_accounts() {
    let table = render_table(&[]);
    assert!(table.contains("No accounts"));
    assert!(table.contains("goog auth login"));
}

#[test]
fn ndjson_emits_one_object_per_line() {
    let rows = vec![
        AccountRow {
            email: "alice@example.com".into(),
            active: true,
        },
        AccountRow {
            email: "bob@example.com".into(),
            active: false,
        },
    ];
    let json = render_ndjson(&rows);
    let lines: Vec<&str> = json.lines().collect();
    assert_eq!(lines.len(), 2);

    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["email"], "alice@example.com");
    assert_eq!(first["active"], true);

    let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(second["email"], "bob@example.com");
    assert_eq!(second["active"], false);
}

#[test]
fn ndjson_is_empty_for_no_accounts() {
    assert_eq!(render_ndjson(&[]), "");
}
