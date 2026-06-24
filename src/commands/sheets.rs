use std::io::Write;

use anyhow::{Context, Result};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::cli::SheetsCommand;
use crate::sheets::{get_spreadsheet, GetSpreadsheetOptions};

pub fn run<S: AccountStore>(cmd: SheetsCommand, client: &AuthClient<'_, S>) -> Result<()> {
    match cmd {
        SheetsCommand::Get {
            spreadsheet_id,
            fields,
            include_grid_data,
            ranges,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_get_to(
                client,
                spreadsheet_id,
                fields,
                include_grid_data,
                ranges,
                &mut std::io::stdout(),
                None,
            ))
        }
    }
}

pub(super) async fn run_get_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    spreadsheet_id: String,
    fields: Option<String>,
    include_grid_data: bool,
    ranges: Vec<String>,
    out: &mut impl Write,
    spreadsheets_url: Option<&str>,
) -> Result<()> {
    let options = get_spreadsheet_options(
        spreadsheet_id,
        fields,
        include_grid_data,
        ranges,
        spreadsheets_url,
    );

    let spreadsheet = get_spreadsheet(client, &options)
        .await
        .context("failed to fetch Google Sheets Spreadsheet")?;
    write_json_line(out, &spreadsheet, "failed to serialize Sheets Spreadsheet")
}

fn get_spreadsheet_options(
    spreadsheet_id: String,
    fields: Option<String>,
    include_grid_data: bool,
    ranges: Vec<String>,
    spreadsheets_url: Option<&str>,
) -> GetSpreadsheetOptions {
    let mut options = GetSpreadsheetOptions::new(spreadsheet_id)
        .with_include_grid_data(include_grid_data)
        .with_ranges(ranges);
    if let Some(fields) = fields {
        options = options.with_fields(fields);
    }
    if let Some(spreadsheets_url) = spreadsheets_url {
        options = options.with_spreadsheets_url(spreadsheets_url);
    }
    options
}

fn write_json_line(out: &mut impl Write, value: &serde_json::Value, context: &str) -> Result<()> {
    serde_json::to_writer(&mut *out, value).context(context.to_string())?;
    writeln!(out).context("failed to write output")?;
    Ok(())
}
