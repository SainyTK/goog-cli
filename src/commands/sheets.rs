use std::future::Future;
use std::io::{Read, Write};

use anyhow::{Context, Result};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::cli::{
    SheetsCommand, SheetsInsertDataOption, SheetsValueInputOption, SheetsValueRenderOption,
    SheetsValuesCommand,
};
use crate::sheets::{
    append_values, batch_clear_values, batch_get_values, batch_update_spreadsheet,
    batch_update_values, clear_values, get_spreadsheet, get_values, update_values,
    AppendValuesOptions, BatchClearValuesOptions, BatchGetValuesOptions,
    BatchUpdateSpreadsheetOptions, BatchUpdateValuesOptions, ClearValuesOptions,
    GetSpreadsheetOptions, GetValuesOptions, InsertDataOption, UpdateValuesOptions,
    ValueInputOption, ValueRenderOption,
};

pub fn run<S: AccountStore>(cmd: SheetsCommand, client: &AuthClient<'_, S>) -> Result<()> {
    match cmd {
        SheetsCommand::Get {
            spreadsheet_id,
            fields,
            include_grid_data,
            ranges,
        } => run_with_runtime(run_get_to(
            client,
            spreadsheet_id,
            fields,
            include_grid_data,
            ranges,
            &mut std::io::stdout(),
            None,
        )),
        SheetsCommand::Values { command } => {
            let mut stdin = std::io::stdin();
            run_with_runtime(run_values_to(
                client,
                command,
                &mut stdin,
                &mut std::io::stdout(),
                None,
            ))
        }
        SheetsCommand::BatchUpdate {
            spreadsheet_id,
            requests,
        } => {
            let mut stdin = std::io::stdin();
            run_with_runtime(run_batch_update_to(
                client,
                spreadsheet_id,
                requests,
                &mut stdin,
                &mut std::io::stdout(),
                None,
            ))
        }
    }
}

fn run_with_runtime(future: impl Future<Output = Result<()>>) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().context("failed to start async runtime")?;
    runtime.block_on(future)
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

pub(super) async fn run_values_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    cmd: SheetsValuesCommand,
    input: &mut impl Read,
    out: &mut impl Write,
    spreadsheets_url: Option<&str>,
) -> Result<()> {
    match cmd {
        SheetsValuesCommand::Get {
            spreadsheet_id,
            range,
            value_render_option,
        } => {
            let options = get_values_options(
                spreadsheet_id,
                range,
                value_render_option.into(),
                spreadsheets_url,
            );
            let response = get_values(client, &options)
                .await
                .context("failed to fetch Google Sheets ValueRange")?;
            write_json_line(out, &response, "failed to serialize Sheets ValueRange")
        }
        SheetsValuesCommand::BatchGet {
            spreadsheet_id,
            ranges,
            value_render_option,
        } => {
            let options = batch_get_values_options(
                spreadsheet_id,
                ranges,
                value_render_option.into(),
                spreadsheets_url,
            );
            let response = batch_get_values(client, &options)
                .await
                .context("failed to fetch Google Sheets ValueRanges")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Batch Get values response",
            )
        }
        SheetsValuesCommand::Update {
            spreadsheet_id,
            range,
            values,
            value_input_option,
        } => {
            let request_body =
                read_request_body(&values, input, "Google Sheets Values request body")?;
            let options = update_values_options(
                spreadsheet_id,
                range,
                request_body,
                value_input_option.into(),
                spreadsheets_url,
            );
            let response = update_values(client, &options)
                .await
                .context("failed to update Google Sheets values")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Update values response",
            )
        }
        SheetsValuesCommand::BatchUpdate {
            spreadsheet_id,
            values,
        } => {
            let request_body =
                read_request_body(&values, input, "Google Sheets Values request body")?;
            let options =
                batch_update_values_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = batch_update_values(client, &options)
                .await
                .context("failed to batch update Google Sheets values")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Batch Update values response",
            )
        }
        SheetsValuesCommand::Append {
            spreadsheet_id,
            range,
            values,
            value_input_option,
            insert_data_option,
        } => {
            let request_body =
                read_request_body(&values, input, "Google Sheets Values request body")?;
            let options = append_values_options(
                spreadsheet_id,
                range,
                request_body,
                value_input_option.into(),
                insert_data_option.into(),
                spreadsheets_url,
            );
            let response = append_values(client, &options)
                .await
                .context("failed to append Google Sheets values")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Append values response",
            )
        }
        SheetsValuesCommand::Clear {
            spreadsheet_id,
            range,
        } => {
            let options = clear_values_options(spreadsheet_id, range, spreadsheets_url);
            let response = clear_values(client, &options)
                .await
                .context("failed to clear Google Sheets values")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Clear values response",
            )
        }
        SheetsValuesCommand::BatchClear {
            spreadsheet_id,
            ranges,
        } => {
            let options = batch_clear_values_options(spreadsheet_id, ranges, spreadsheets_url);
            let response = batch_clear_values(client, &options)
                .await
                .context("failed to batch clear Google Sheets values")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Batch Clear values response",
            )
        }
    }
}

pub(super) async fn run_batch_update_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    spreadsheet_id: String,
    requests: String,
    input: &mut impl Read,
    out: &mut impl Write,
    spreadsheets_url: Option<&str>,
) -> Result<()> {
    let request_body =
        read_request_body(&requests, input, "Google Sheets Batch Update request body")?;
    let options = batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);

    let response = batch_update_spreadsheet(client, &options)
        .await
        .context("failed to apply Google Sheets Batch Update")?;
    write_json_line(
        out,
        &response,
        "failed to serialize Sheets Batch Update response",
    )
}

fn read_request_body(
    path_or_stdin: &str,
    input: &mut impl Read,
    request_name: &str,
) -> Result<serde_json::Value> {
    let (body, request_source) = if path_or_stdin == "-" {
        let mut body = String::new();
        input
            .read_to_string(&mut body)
            .with_context(|| format!("failed to read {request_name} from stdin"))?;
        (body, "stdin".to_string())
    } else {
        let body = std::fs::read_to_string(path_or_stdin)
            .with_context(|| format!("failed to read {request_name}: {path_or_stdin}"))?;
        (body, path_or_stdin.to_string())
    };

    serde_json::from_str(&body)
        .with_context(|| format!("failed to parse {request_name} from {request_source}"))
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

fn get_values_options(
    spreadsheet_id: String,
    range: String,
    value_render_option: ValueRenderOption,
    spreadsheets_url: Option<&str>,
) -> GetValuesOptions {
    let mut options =
        GetValuesOptions::new(spreadsheet_id, range).with_value_render_option(value_render_option);
    if let Some(spreadsheets_url) = spreadsheets_url {
        options = options.with_spreadsheets_url(spreadsheets_url);
    }
    options
}

fn batch_get_values_options(
    spreadsheet_id: String,
    ranges: Vec<String>,
    value_render_option: ValueRenderOption,
    spreadsheets_url: Option<&str>,
) -> BatchGetValuesOptions {
    let mut options = BatchGetValuesOptions::new(spreadsheet_id, ranges)
        .with_value_render_option(value_render_option);
    if let Some(spreadsheets_url) = spreadsheets_url {
        options = options.with_spreadsheets_url(spreadsheets_url);
    }
    options
}

fn update_values_options(
    spreadsheet_id: String,
    range: String,
    request_body: serde_json::Value,
    value_input_option: ValueInputOption,
    spreadsheets_url: Option<&str>,
) -> UpdateValuesOptions {
    let mut options = UpdateValuesOptions::new(spreadsheet_id, range, request_body)
        .with_value_input_option(value_input_option);
    if let Some(spreadsheets_url) = spreadsheets_url {
        options = options.with_spreadsheets_url(spreadsheets_url);
    }
    options
}

fn batch_update_values_options(
    spreadsheet_id: String,
    request_body: serde_json::Value,
    spreadsheets_url: Option<&str>,
) -> BatchUpdateValuesOptions {
    let mut options = BatchUpdateValuesOptions::new(spreadsheet_id, request_body);
    if let Some(spreadsheets_url) = spreadsheets_url {
        options = options.with_spreadsheets_url(spreadsheets_url);
    }
    options
}

fn append_values_options(
    spreadsheet_id: String,
    range: String,
    request_body: serde_json::Value,
    value_input_option: ValueInputOption,
    insert_data_option: InsertDataOption,
    spreadsheets_url: Option<&str>,
) -> AppendValuesOptions {
    let mut options = AppendValuesOptions::new(spreadsheet_id, range, request_body)
        .with_value_input_option(value_input_option)
        .with_insert_data_option(insert_data_option);
    if let Some(spreadsheets_url) = spreadsheets_url {
        options = options.with_spreadsheets_url(spreadsheets_url);
    }
    options
}

fn clear_values_options(
    spreadsheet_id: String,
    range: String,
    spreadsheets_url: Option<&str>,
) -> ClearValuesOptions {
    let mut options = ClearValuesOptions::new(spreadsheet_id, range);
    if let Some(spreadsheets_url) = spreadsheets_url {
        options = options.with_spreadsheets_url(spreadsheets_url);
    }
    options
}

fn batch_clear_values_options(
    spreadsheet_id: String,
    ranges: Vec<String>,
    spreadsheets_url: Option<&str>,
) -> BatchClearValuesOptions {
    let mut options = BatchClearValuesOptions::new(spreadsheet_id, ranges);
    if let Some(spreadsheets_url) = spreadsheets_url {
        options = options.with_spreadsheets_url(spreadsheets_url);
    }
    options
}

fn batch_update_spreadsheet_options(
    spreadsheet_id: String,
    request_body: serde_json::Value,
    spreadsheets_url: Option<&str>,
) -> BatchUpdateSpreadsheetOptions {
    let mut options = BatchUpdateSpreadsheetOptions::new(spreadsheet_id, request_body);
    if let Some(spreadsheets_url) = spreadsheets_url {
        options = options.with_spreadsheets_url(spreadsheets_url);
    }
    options
}

impl From<SheetsValueRenderOption> for ValueRenderOption {
    fn from(value: SheetsValueRenderOption) -> Self {
        match value {
            SheetsValueRenderOption::FormattedValue => Self::FormattedValue,
            SheetsValueRenderOption::UnformattedValue => Self::UnformattedValue,
            SheetsValueRenderOption::Formula => Self::Formula,
        }
    }
}

impl From<SheetsValueInputOption> for ValueInputOption {
    fn from(value: SheetsValueInputOption) -> Self {
        match value {
            SheetsValueInputOption::Raw => Self::Raw,
            SheetsValueInputOption::UserEntered => Self::UserEntered,
        }
    }
}

impl From<SheetsInsertDataOption> for InsertDataOption {
    fn from(value: SheetsInsertDataOption) -> Self {
        match value {
            SheetsInsertDataOption::InsertRows => Self::InsertRows,
            SheetsInsertDataOption::Overwrite => Self::Overwrite,
        }
    }
}

fn write_json_line(out: &mut impl Write, value: &serde_json::Value, context: &str) -> Result<()> {
    serde_json::to_writer(&mut *out, value).context(context.to_string())?;
    writeln!(out).context("failed to write output")?;
    Ok(())
}
