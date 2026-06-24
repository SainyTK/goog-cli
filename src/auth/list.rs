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
    out.push_str(&format!(
        "  {:<width$}  ACTIVE\n",
        "EMAIL",
        width = email_width
    ));
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
