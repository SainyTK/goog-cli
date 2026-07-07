use std::future::Future;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::auth::config::Config;
use crate::auth::state::resource_key;
use crate::auth::unified_access::{AccessFuture, UnifiedAccess};
use crate::cli::{
    SheetsCommand, SheetsInsertDataOption, SheetsSheetCommand, SheetsValueInputOption,
    SheetsValueRenderOption, SheetsValuesCommand,
};
use crate::sheets::{
    create_spreadsheet, AppendValuesOptions, BatchClearValuesOptions, BatchGetValuesOptions,
    BatchUpdateSpreadsheetOptions, BatchUpdateValuesOptions, ClearValuesOptions,
    CreateSpreadsheetOptions, GetSpreadsheetOptions, GetValuesOptions, InsertDataOption,
    SheetsError, SheetsOperation, UpdateValuesOptions, ValueInputOption, ValueRenderOption,
};

pub fn run<S: AccountStore>(
    cmd: SheetsCommand,
    config: &Config,
    store: &S,
    account_override: Option<&str>,
) -> Result<()> {
    match cmd {
        SheetsCommand::Create { title } => {
            let client = AuthClient::from_config(config.clone(), store, account_override)?;
            run_with_runtime(run_create_to(&client, title, &mut std::io::stdout(), None))
        }
        SheetsCommand::Get {
            spreadsheet_id,
            fields,
            include_grid_data,
            ranges,
        } => run_with_runtime(run_get_unified_to(
            config,
            store,
            account_override,
            spreadsheet_id,
            fields,
            include_grid_data,
            ranges,
            &mut std::io::stdout(),
            None,
            None,
        )),
        SheetsCommand::Values { command } => {
            let mut stdin = std::io::stdin();
            run_with_runtime(run_values_unified_to(
                config,
                store,
                account_override,
                command,
                &mut stdin,
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        SheetsCommand::Sheet { command } => run_with_runtime(run_sheet_unified_to(
            config,
            store,
            account_override,
            command,
            &mut std::io::stdout(),
            None,
            None,
        )),
        SheetsCommand::BatchUpdate {
            spreadsheet_id,
            requests,
        } => {
            let mut stdin = std::io::stdin();
            run_with_runtime(run_batch_update_unified_to(
                config,
                store,
                account_override,
                spreadsheet_id,
                requests,
                &mut stdin,
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
    }
}

fn run_with_runtime(future: impl Future<Output = Result<()>>) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().context("failed to start async runtime")?;
    runtime.block_on(future)
}

#[cfg(test)]
pub(super) async fn run_sheet_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    cmd: SheetsSheetCommand,
    out: &mut impl Write,
    spreadsheets_url: Option<&str>,
) -> Result<()> {
    match cmd {
        SheetsSheetCommand::Add {
            spreadsheet_id,
            title,
            sheet_id,
            index,
        } => {
            let request_body = add_sheet_request_body(title, sheet_id, index);
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to add Google Sheets sheet")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Add sheet response",
            )
        }
        SheetsSheetCommand::Delete {
            spreadsheet_id,
            sheet_id,
        } => {
            let request_body = delete_sheet_request_body(sheet_id);
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to delete Google Sheets sheet")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Delete sheet response",
            )
        }
        SheetsSheetCommand::Rename {
            spreadsheet_id,
            sheet_id,
            title,
        } => {
            let request_body = rename_sheet_request_body(sheet_id, title);
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to rename Google Sheets sheet")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Rename sheet response",
            )
        }
        SheetsSheetCommand::Duplicate {
            spreadsheet_id,
            source_sheet_id,
            title,
            sheet_id,
            index,
        } => {
            let request_body =
                duplicate_sheet_request_body(source_sheet_id, title, sheet_id, index);
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to duplicate Google Sheets sheet")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Duplicate sheet response",
            )
        }
        SheetsSheetCommand::Hide {
            spreadsheet_id,
            sheet_id,
        } => {
            let request_body = set_sheet_hidden_request_body(sheet_id, true);
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to hide Google Sheets sheet")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Hide sheet response",
            )
        }
        SheetsSheetCommand::Unhide {
            spreadsheet_id,
            sheet_id,
        } => {
            let request_body = set_sheet_hidden_request_body(sheet_id, false);
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to unhide Google Sheets sheet")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Unhide sheet response",
            )
        }
    }
}

pub(super) async fn run_sheet_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    cmd: SheetsSheetCommand,
    out: &mut impl Write,
    spreadsheets_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    match cmd {
        SheetsSheetCommand::Add {
            spreadsheet_id,
            title,
            sheet_id,
            index,
        } => {
            let request_body = add_sheet_request_body(title, sheet_id, index);
            let options = batch_update_spreadsheet_options(
                spreadsheet_id.clone(),
                request_body,
                spreadsheets_url,
            );
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::BatchUpdateSpreadsheet(&options),
                state_path,
            )
            .await
            .context("failed to add Google Sheets sheet")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Add sheet response",
            )
        }
        SheetsSheetCommand::Delete {
            spreadsheet_id,
            sheet_id,
        } => {
            let request_body = delete_sheet_request_body(sheet_id);
            let options = batch_update_spreadsheet_options(
                spreadsheet_id.clone(),
                request_body,
                spreadsheets_url,
            );
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::BatchUpdateSpreadsheet(&options),
                state_path,
            )
            .await
            .context("failed to delete Google Sheets sheet")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Delete sheet response",
            )
        }
        SheetsSheetCommand::Rename {
            spreadsheet_id,
            sheet_id,
            title,
        } => {
            let request_body = rename_sheet_request_body(sheet_id, title);
            let options = batch_update_spreadsheet_options(
                spreadsheet_id.clone(),
                request_body,
                spreadsheets_url,
            );
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::BatchUpdateSpreadsheet(&options),
                state_path,
            )
            .await
            .context("failed to rename Google Sheets sheet")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Rename sheet response",
            )
        }
        SheetsSheetCommand::Duplicate {
            spreadsheet_id,
            source_sheet_id,
            title,
            sheet_id,
            index,
        } => {
            let request_body =
                duplicate_sheet_request_body(source_sheet_id, title, sheet_id, index);
            let options = batch_update_spreadsheet_options(
                spreadsheet_id.clone(),
                request_body,
                spreadsheets_url,
            );
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::BatchUpdateSpreadsheet(&options),
                state_path,
            )
            .await
            .context("failed to duplicate Google Sheets sheet")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Duplicate sheet response",
            )
        }
        SheetsSheetCommand::Hide {
            spreadsheet_id,
            sheet_id,
        } => {
            let request_body = set_sheet_hidden_request_body(sheet_id, true);
            let options = batch_update_spreadsheet_options(
                spreadsheet_id.clone(),
                request_body,
                spreadsheets_url,
            );
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::BatchUpdateSpreadsheet(&options),
                state_path,
            )
            .await
            .context("failed to hide Google Sheets sheet")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Hide sheet response",
            )
        }
        SheetsSheetCommand::Unhide {
            spreadsheet_id,
            sheet_id,
        } => {
            let request_body = set_sheet_hidden_request_body(sheet_id, false);
            let options = batch_update_spreadsheet_options(
                spreadsheet_id.clone(),
                request_body,
                spreadsheets_url,
            );
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::BatchUpdateSpreadsheet(&options),
                state_path,
            )
            .await
            .context("failed to unhide Google Sheets sheet")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Unhide sheet response",
            )
        }
    }
}

pub(super) async fn run_create_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    title: String,
    out: &mut impl Write,
    spreadsheets_url: Option<&str>,
) -> Result<()> {
    let mut options = CreateSpreadsheetOptions::new(title);
    if let Some(spreadsheets_url) = spreadsheets_url {
        options = options.with_spreadsheets_url(spreadsheets_url);
    }

    let spreadsheet = create_spreadsheet(client, &options)
        .await
        .context("failed to create Google Sheets Spreadsheet")?;
    let spreadsheet_id = spreadsheet
        .get("spreadsheetId")
        .and_then(serde_json::Value::as_str)
        .context("Google Sheets create response did not include a spreadsheetId")?;

    writeln!(
        out,
        "{}\thttps://docs.google.com/spreadsheets/d/{}/edit",
        spreadsheet_id, spreadsheet_id
    )
    .context("failed to write output")?;
    Ok(())
}

#[cfg(test)]
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

    let spreadsheet = SheetsOperation::GetSpreadsheet(&options)
        .execute(client)
        .await
        .context("failed to fetch Google Sheets Spreadsheet")?;
    write_json_line(out, &spreadsheet, "failed to serialize Sheets Spreadsheet")
}

pub(super) async fn run_get_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    spreadsheet_id: String,
    fields: Option<String>,
    include_grid_data: bool,
    ranges: Vec<String>,
    out: &mut impl Write,
    spreadsheets_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let options = get_spreadsheet_options(
        spreadsheet_id.clone(),
        fields,
        include_grid_data,
        ranges,
        spreadsheets_url,
    );
    let spreadsheet = run_spreadsheet_attempt(
        config,
        store,
        account_override,
        &SheetsOperation::GetSpreadsheet(&options),
        state_path,
    )
    .await
    .context("failed to fetch Google Sheets Spreadsheet")?;

    write_json_line(out, &spreadsheet, "failed to serialize Sheets Spreadsheet")
}

#[cfg(test)]
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
            let response = SheetsOperation::GetValues(&options)
                .execute(client)
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
            let response = SheetsOperation::BatchGetValues(&options)
                .execute(client)
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
            let response = SheetsOperation::UpdateValues(&options)
                .execute(client)
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
            let response = SheetsOperation::BatchUpdateValues(&options)
                .execute(client)
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
            let response = SheetsOperation::AppendValues(&options)
                .execute(client)
                .await
                .context("failed to append Google Sheets values")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Append values response",
            )
        }
        SheetsValuesCommand::AppendRow {
            spreadsheet_id,
            range,
            values,
            value_input_option,
            insert_data_option,
        } => {
            let request_body = row_value_range(values);
            let options = append_values_options(
                spreadsheet_id,
                range,
                request_body,
                value_input_option.into(),
                insert_data_option.into(),
                spreadsheets_url,
            );
            let response = SheetsOperation::AppendValues(&options)
                .execute(client)
                .await
                .context("failed to append Google Sheets row")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Append row response",
            )
        }
        SheetsValuesCommand::AppendTable {
            spreadsheet_id,
            range,
            data,
            value_input_option,
            insert_data_option,
        } => {
            let request_body = table_value_range(&data)?;
            let options = append_values_options(
                spreadsheet_id,
                range,
                request_body,
                value_input_option.into(),
                insert_data_option.into(),
                spreadsheets_url,
            );
            let response = SheetsOperation::AppendValues(&options)
                .execute(client)
                .await
                .context("failed to append Google Sheets table")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Append table response",
            )
        }
        SheetsValuesCommand::Clear {
            spreadsheet_id,
            range,
        } => {
            let options = clear_values_options(spreadsheet_id, range, spreadsheets_url);
            let response = SheetsOperation::ClearValues(&options)
                .execute(client)
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
            let response = SheetsOperation::BatchClearValues(&options)
                .execute(client)
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

pub(super) async fn run_values_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    cmd: SheetsValuesCommand,
    input: &mut impl Read,
    out: &mut impl Write,
    spreadsheets_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    match cmd {
        SheetsValuesCommand::Get {
            spreadsheet_id,
            range,
            value_render_option,
        } => {
            let options = get_values_options(
                spreadsheet_id.clone(),
                range,
                value_render_option.into(),
                spreadsheets_url,
            );
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::GetValues(&options),
                state_path,
            )
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
                spreadsheet_id.clone(),
                ranges,
                value_render_option.into(),
                spreadsheets_url,
            );
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::BatchGetValues(&options),
                state_path,
            )
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
                spreadsheet_id.clone(),
                range,
                request_body,
                value_input_option.into(),
                spreadsheets_url,
            );
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::UpdateValues(&options),
                state_path,
            )
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
                batch_update_values_options(spreadsheet_id.clone(), request_body, spreadsheets_url);
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::BatchUpdateValues(&options),
                state_path,
            )
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
                spreadsheet_id.clone(),
                range,
                request_body,
                value_input_option.into(),
                insert_data_option.into(),
                spreadsheets_url,
            );
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::AppendValues(&options),
                state_path,
            )
            .await
            .context("failed to append Google Sheets values")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Append values response",
            )
        }
        SheetsValuesCommand::AppendRow {
            spreadsheet_id,
            range,
            values,
            value_input_option,
            insert_data_option,
        } => {
            let request_body = row_value_range(values);
            let options = append_values_options(
                spreadsheet_id.clone(),
                range,
                request_body,
                value_input_option.into(),
                insert_data_option.into(),
                spreadsheets_url,
            );
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::AppendValues(&options),
                state_path,
            )
            .await
            .context("failed to append Google Sheets row")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Append row response",
            )
        }
        SheetsValuesCommand::AppendTable {
            spreadsheet_id,
            range,
            data,
            value_input_option,
            insert_data_option,
        } => {
            let request_body = table_value_range(&data)?;
            let options = append_values_options(
                spreadsheet_id.clone(),
                range,
                request_body,
                value_input_option.into(),
                insert_data_option.into(),
                spreadsheets_url,
            );
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::AppendValues(&options),
                state_path,
            )
            .await
            .context("failed to append Google Sheets table")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Append table response",
            )
        }
        SheetsValuesCommand::Clear {
            spreadsheet_id,
            range,
        } => {
            let options = clear_values_options(spreadsheet_id.clone(), range, spreadsheets_url);
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::ClearValues(&options),
                state_path,
            )
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
            let options =
                batch_clear_values_options(spreadsheet_id.clone(), ranges, spreadsheets_url);
            let response = run_spreadsheet_attempt(
                config,
                store,
                account_override,
                &SheetsOperation::BatchClearValues(&options),
                state_path,
            )
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

#[cfg(test)]
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

    let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
        .execute(client)
        .await
        .context("failed to apply Google Sheets Batch Update")?;
    write_json_line(
        out,
        &response,
        "failed to serialize Sheets Batch Update response",
    )
}

pub(super) async fn run_batch_update_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    spreadsheet_id: String,
    requests: String,
    input: &mut impl Read,
    out: &mut impl Write,
    spreadsheets_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let request_body =
        read_request_body(&requests, input, "Google Sheets Batch Update request body")?;
    let options =
        batch_update_spreadsheet_options(spreadsheet_id.clone(), request_body, spreadsheets_url);
    let response = run_spreadsheet_attempt(
        config,
        store,
        account_override,
        &SheetsOperation::BatchUpdateSpreadsheet(&options),
        state_path,
    )
    .await
    .context("failed to apply Google Sheets Batch Update")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Sheets Batch Update response",
    )
}

async fn run_spreadsheet_attempt<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    operation: &SheetsOperation<'_>,
    state_path: Option<&Path>,
) -> Result<serde_json::Value, SheetsError> {
    let target_resource_key = resource_key("sheets", operation.spreadsheet_id());
    run_with_sheets_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        operation,
        state_path,
    )
    .await
}

async fn run_with_sheets_unified_access<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    target_resource_key: &str,
    operation: &SheetsOperation<'_>,
    state_path: Option<&Path>,
) -> Result<serde_json::Value, SheetsError> {
    UnifiedAccess::run(
        config,
        account_override,
        target_resource_key,
        state_path,
        |account| -> AccessFuture<'_, serde_json::Value, SheetsError> {
            Box::pin(run_sheets_access_as_account(
                config, store, operation, account,
            ))
        },
        is_target_access_failure,
    )
    .await
}

async fn run_sheets_access_as_account<S: AccountStore>(
    config: &Config,
    store: &S,
    operation: &SheetsOperation<'_>,
    account: String,
) -> Result<serde_json::Value, SheetsError> {
    let client = AuthClient::from_config(config.clone(), store, Some(&account))
        .map_err(SheetsError::Auth)?;
    let result = operation.execute(&client).await?;
    Ok(result)
}

fn is_target_access_failure(err: &SheetsError) -> bool {
    matches!(err, SheetsError::NotFound | SheetsError::PermissionDenied)
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

fn row_value_range(values: Vec<String>) -> serde_json::Value {
    serde_json::json!({
        "majorDimension": "ROWS",
        "values": [values],
    })
}

fn table_value_range(path: &str) -> Result<serde_json::Value> {
    let delimiter = if path.ends_with(".tsv") { '\t' } else { ',' };
    let body = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read Google Sheets table data file: {path}"))?;
    let rows: Vec<Vec<String>> = body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split(delimiter)
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect();

    if rows.is_empty() {
        anyhow::bail!("Google Sheets table data file is empty");
    }

    let columns = rows[0].len();
    if columns == 0 || rows.iter().any(|row| row.len() != columns) {
        anyhow::bail!("Google Sheets table data must be rectangular");
    }

    Ok(serde_json::json!({
        "majorDimension": "ROWS",
        "values": rows,
    }))
}

fn add_sheet_request_body(
    title: String,
    sheet_id: Option<i64>,
    index: Option<i64>,
) -> serde_json::Value {
    let mut properties = serde_json::Map::new();
    properties.insert("title".to_string(), serde_json::Value::String(title));
    if let Some(sheet_id) = sheet_id {
        properties.insert("sheetId".to_string(), serde_json::json!(sheet_id));
    }
    if let Some(index) = index {
        properties.insert("index".to_string(), serde_json::json!(index));
    }

    serde_json::json!({
        "requests": [
            {
                "addSheet": {
                    "properties": properties
                }
            }
        ]
    })
}

fn delete_sheet_request_body(sheet_id: i64) -> serde_json::Value {
    serde_json::json!({
        "requests": [
            {
                "deleteSheet": {
                    "sheetId": sheet_id
                }
            }
        ]
    })
}

fn rename_sheet_request_body(sheet_id: i64, title: String) -> serde_json::Value {
    serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": sheet_id,
                        "title": title
                    },
                    "fields": "title"
                }
            }
        ]
    })
}

fn duplicate_sheet_request_body(
    source_sheet_id: i64,
    title: String,
    sheet_id: Option<i64>,
    index: Option<i64>,
) -> serde_json::Value {
    let mut duplicate_sheet = serde_json::Map::new();
    duplicate_sheet.insert(
        "sourceSheetId".to_string(),
        serde_json::json!(source_sheet_id),
    );
    duplicate_sheet.insert("newSheetName".to_string(), serde_json::Value::String(title));
    if let Some(sheet_id) = sheet_id {
        duplicate_sheet.insert("newSheetId".to_string(), serde_json::json!(sheet_id));
    }
    if let Some(index) = index {
        duplicate_sheet.insert("insertSheetIndex".to_string(), serde_json::json!(index));
    }

    serde_json::json!({
        "requests": [
            {
                "duplicateSheet": duplicate_sheet
            }
        ]
    })
}

fn set_sheet_hidden_request_body(sheet_id: i64, hidden: bool) -> serde_json::Value {
    serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": sheet_id,
                        "hidden": hidden
                    },
                    "fields": "hidden"
                }
            }
        ]
    })
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
