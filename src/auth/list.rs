use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AccountRow {
    pub email: String,
    pub active: bool,
}

pub fn rows_from_config(accounts: &[String], active: Option<&str>) -> Vec<AccountRow> {
    accounts
        .iter()
        .map(|email| AccountRow {
            email: email.clone(),
            active: active == Some(email.as_str()),
        })
        .collect()
}

pub fn render_table(rows: &[AccountRow]) -> String {
    if rows.is_empty() {
        return "No accounts logged in. Run `goog auth login` to add one.\n".to_string();
    }

    let email_width = rows
        .iter()
        .map(|r| r.email.len())
        .max()
        .unwrap_or(0)
        .max("EMAIL".len());

    let mut out = String::new();
    out.push_str(&format!("  {:<width$}  ACTIVE\n", "EMAIL", width = email_width));
    for row in rows {
        let marker = if row.active { "*" } else { " " };
        let active_col = if row.active { "*" } else { "" };
        out.push_str(&format!(
            "{} {:<width$}  {}\n",
            marker,
            row.email,
            active_col,
            width = email_width
        ));
    }
    out
}

pub fn render_ndjson(rows: &[AccountRow]) -> String {
    let mut out = String::new();
    for row in rows {
        out.push_str(&serde_json::to_string(row).expect("AccountRow is always serializable"));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_marks_the_active_account() {
        let accounts = vec!["alice@example.com".to_string(), "bob@example.com".to_string()];
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
}
