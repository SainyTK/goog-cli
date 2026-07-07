use std::future::Future;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::auth::config::Config;
use crate::auth::state::resource_key;
use crate::auth::unified_access::{AccessFuture, UnifiedAccess};
use crate::cli::{
    SheetsBorderEdge, SheetsBorderStyle, SheetsCommand, SheetsConditionalFormatCondition,
    SheetsDimension, SheetsHorizontalAlignment, SheetsInsertDataOption, SheetsMergeType,
    SheetsNumberFormatType, SheetsPasteOrientation, SheetsPasteType, SheetsSheetCommand,
    SheetsSortOrder, SheetsTextDirection, SheetsValueInputOption, SheetsValueRenderOption,
    SheetsValuesCommand, SheetsVerticalAlignment, SheetsWrapStrategy,
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
        SheetsSheetCommand::Move {
            spreadsheet_id,
            sheet_id,
            index,
        } => {
            let request_body = move_sheet_request_body(sheet_id, index);
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to move Google Sheets sheet")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Move sheet response",
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
        SheetsSheetCommand::Freeze {
            spreadsheet_id,
            sheet_id,
            rows,
            columns,
        } => {
            let request_body = freeze_sheet_request_body(sheet_id, rows, columns);
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to freeze Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Freeze sheet response",
            )
        }
        SheetsSheetCommand::Resize {
            spreadsheet_id,
            sheet_id,
            rows,
            columns,
        } => {
            let request_body = resize_sheet_request_body(sheet_id, rows, columns);
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to resize Google Sheets grid")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Resize sheet response",
            )
        }
        SheetsSheetCommand::AutoResize {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body =
                auto_resize_sheet_request_body(sheet_id, dimension, start_index, end_index)?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to auto-resize Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Auto-resize sheet response",
            )
        }
        SheetsSheetCommand::SetDimensionSize {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
            pixel_size,
        } => {
            let request_body = set_dimension_size_sheet_request_body(
                sheet_id,
                dimension,
                start_index,
                end_index,
                pixel_size,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets row height or column width")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Set dimension size response",
            )
        }
        SheetsSheetCommand::HideDimension {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body = dimension_visibility_sheet_request_body(
                sheet_id,
                dimension,
                start_index,
                end_index,
                true,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to hide Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Hide dimension response",
            )
        }
        SheetsSheetCommand::UnhideDimension {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body = dimension_visibility_sheet_request_body(
                sheet_id,
                dimension,
                start_index,
                end_index,
                false,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to unhide Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Unhide dimension response",
            )
        }
        SheetsSheetCommand::GroupDimension {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body =
                dimension_group_sheet_request_body(sheet_id, dimension, start_index, end_index)?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to group Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Group dimension response",
            )
        }
        SheetsSheetCommand::UngroupDimension {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body =
                dimension_ungroup_sheet_request_body(sheet_id, dimension, start_index, end_index)?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to ungroup Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Ungroup dimension response",
            )
        }
        SheetsSheetCommand::CollapseDimensionGroup {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body = dimension_group_collapsed_sheet_request_body(
                sheet_id,
                dimension,
                start_index,
                end_index,
                true,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to collapse Google Sheets row or column group")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Collapse dimension group response",
            )
        }
        SheetsSheetCommand::ExpandDimensionGroup {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body = dimension_group_collapsed_sheet_request_body(
                sheet_id,
                dimension,
                start_index,
                end_index,
                false,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to expand Google Sheets row or column group")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Expand dimension group response",
            )
        }
        SheetsSheetCommand::InsertDimension {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
            inherit_from_before,
        } => {
            let request_body = insert_dimension_sheet_request_body(
                sheet_id,
                dimension,
                start_index,
                end_index,
                inherit_from_before,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to insert Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Insert dimension response",
            )
        }
        SheetsSheetCommand::DeleteDimension {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body =
                delete_dimension_sheet_request_body(sheet_id, dimension, start_index, end_index)?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to delete Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Delete dimension response",
            )
        }
        SheetsSheetCommand::BasicFilter {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
        } => {
            let request_body = basic_filter_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets basic filter")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Basic filter response",
            )
        }
        SheetsSheetCommand::ClearBasicFilter {
            spreadsheet_id,
            sheet_id,
        } => {
            let request_body = clear_basic_filter_sheet_request_body(sheet_id);
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to clear Google Sheets basic filter")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Clear basic filter response",
            )
        }
        SheetsSheetCommand::Merge {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            merge_type,
        } => {
            let request_body = merge_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                merge_type,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to merge Google Sheets cells")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Merge cells response",
            )
        }
        SheetsSheetCommand::Unmerge {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
        } => {
            let request_body =
                unmerge_sheet_request_body(sheet_id, start_row, end_row, start_column, end_column)?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to unmerge Google Sheets cells")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Unmerge cells response",
            )
        }
        SheetsSheetCommand::SortRange {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            sort_column,
            order,
        } => {
            let request_body = sort_range_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                sort_column,
                order,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to sort Google Sheets range")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Sort range response",
            )
        }
        SheetsSheetCommand::FindReplace {
            spreadsheet_id,
            find,
            replacement,
            sheet_id,
            match_case,
            match_entire_cell,
            search_by_regex,
            include_formulas,
        } => {
            let request_body = find_replace_sheet_request_body(
                find,
                replacement,
                sheet_id,
                match_case,
                match_entire_cell,
                search_by_regex,
                include_formulas,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to find and replace Google Sheets text")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Find and replace response",
            )
        }
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
        } => {
            let request_body = copy_paste_sheet_request_body(
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
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to copy and paste Google Sheets cells")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Copy paste response",
            )
        }
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
        } => {
            let request_body = cut_paste_sheet_request_body(
                source_sheet_id,
                source_start_row,
                source_end_row,
                source_start_column,
                source_end_column,
                destination_sheet_id,
                destination_row,
                destination_column,
                paste_type,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to cut and paste Google Sheets cells")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Cut paste response",
            )
        }
        SheetsSheetCommand::BackgroundColor {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            color,
        } => {
            let request_body = background_color_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                &color,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets background color")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Background color response",
            )
        }
        SheetsSheetCommand::TextColor {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            color,
        } => {
            let request_body = text_color_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                &color,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets text color")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Text color response",
            )
        }
        SheetsSheetCommand::FontSize {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            size,
        } => {
            let request_body = font_size_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                size,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets font size")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Font size response",
            )
        }
        SheetsSheetCommand::FontFamily {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            family,
        } => {
            let request_body = font_family_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                &family,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets font family")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Font family response",
            )
        }
        SheetsSheetCommand::NumberFormat {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            format_type,
            pattern,
        } => {
            let request_body = number_format_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                format_type,
                pattern.as_deref(),
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets number format")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Number format response",
            )
        }
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
        } => {
            let request_body = borders_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                &edge,
                style,
                color.as_deref(),
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets borders")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Borders response",
            )
        }
        SheetsSheetCommand::ClearFormat {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
        } => {
            let request_body = clear_format_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to clear Google Sheets cell formatting")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Clear format response",
            )
        }
        SheetsSheetCommand::Bold {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            off,
        } => {
            let request_body = bold_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                !off,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets bold text style")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Bold text response",
            )
        }
        SheetsSheetCommand::Italic {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            off,
        } => {
            let request_body = italic_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                !off,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets italic text style")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Italic text response",
            )
        }
        SheetsSheetCommand::Underline {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            off,
        } => {
            let request_body = underline_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                !off,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets underline text style")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Underline text response",
            )
        }
        SheetsSheetCommand::Strikethrough {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            off,
        } => {
            let request_body = strikethrough_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                !off,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets strikethrough text style")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Strikethrough text response",
            )
        }
        SheetsSheetCommand::HorizontalAlign {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            alignment,
        } => {
            let request_body = horizontal_align_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                alignment,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets horizontal alignment")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Horizontal align response",
            )
        }
        SheetsSheetCommand::VerticalAlign {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            alignment,
        } => {
            let request_body = vertical_align_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                alignment,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets vertical alignment")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Vertical align response",
            )
        }
        SheetsSheetCommand::TextWrap {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            strategy,
        } => {
            let request_body = text_wrap_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                strategy,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets text wrapping")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Text wrap response",
            )
        }
        SheetsSheetCommand::TextRotation {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            angle,
        } => {
            let request_body = text_rotation_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                angle,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets text rotation")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Text rotation response",
            )
        }
        SheetsSheetCommand::TextDirection {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            direction,
        } => {
            let request_body = text_direction_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                direction,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets text direction")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Text direction response",
            )
        }
        SheetsSheetCommand::Note {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            note,
            clear,
        } => {
            let request_body = note_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                note.as_deref(),
                clear,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to update Google Sheets cell notes")?;
            write_json_line(out, &response, "failed to serialize Sheets Note response")
        }
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
        } => {
            let request_body = data_validation_list_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                &values,
                allow_invalid,
                hide_dropdown,
                input_message.as_deref(),
                clear,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to update Google Sheets data validation")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Data validation response",
            )
        }
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
        } => {
            let request_body = data_validation_checkbox_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                checked_value.as_deref(),
                unchecked_value.as_deref(),
                allow_invalid,
                input_message.as_deref(),
                clear,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to update Google Sheets checkbox validation")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Checkbox validation response",
            )
        }
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
        } => {
            let request_body = conditional_format_color_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                condition,
                &value,
                background_color.as_deref(),
                text_color.as_deref(),
                index,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to add Google Sheets conditional format rule")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Conditional format response",
            )
        }
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
        } => {
            let request_body = conditional_format_update_sheet_request_body(
                sheet_id,
                index,
                start_row,
                end_row,
                start_column,
                end_column,
                condition,
                &value,
                background_color.as_deref(),
                text_color.as_deref(),
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to update Google Sheets conditional format rule")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Conditional format update response",
            )
        }
        SheetsSheetCommand::ConditionalFormatDelete {
            spreadsheet_id,
            sheet_id,
            index,
        } => {
            let request_body = conditional_format_delete_sheet_request_body(sheet_id, index);
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to delete Google Sheets conditional format rule")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Conditional format delete response",
            )
        }
        SheetsSheetCommand::ConditionalFormatMove {
            spreadsheet_id,
            sheet_id,
            index,
            new_index,
        } => {
            let request_body =
                conditional_format_move_sheet_request_body(sheet_id, index, new_index);
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to move Google Sheets conditional format rule")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Conditional format move response",
            )
        }
        SheetsSheetCommand::ProtectRange {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            description,
            warning_only,
        } => {
            let request_body = protect_range_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                description.as_deref(),
                warning_only,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to protect Google Sheets range")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Protect range response",
            )
        }
        SheetsSheetCommand::UnprotectRange {
            spreadsheet_id,
            protected_range_id,
        } => {
            let request_body = unprotect_range_sheet_request_body(protected_range_id);
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to remove Google Sheets protected range")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Unprotect range response",
            )
        }
        SheetsSheetCommand::UpdateProtectedRange {
            spreadsheet_id,
            protected_range_id,
            description,
            warning_only,
            enforce,
        } => {
            let request_body = update_protected_range_sheet_request_body(
                protected_range_id,
                description.as_deref(),
                warning_only,
                enforce,
            )?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to update Google Sheets protected range")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Update protected range response",
            )
        }
        SheetsSheetCommand::TabColor {
            spreadsheet_id,
            sheet_id,
            color,
        } => {
            let request_body = tab_color_sheet_request_body(sheet_id, &color)?;
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to set Google Sheets tab color")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Tab color response",
            )
        }
        SheetsSheetCommand::ClearTabColor {
            spreadsheet_id,
            sheet_id,
        } => {
            let request_body = clear_tab_color_sheet_request_body(sheet_id);
            let options =
                batch_update_spreadsheet_options(spreadsheet_id, request_body, spreadsheets_url);
            let response = SheetsOperation::BatchUpdateSpreadsheet(&options)
                .execute(client)
                .await
                .context("failed to clear Google Sheets tab color")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Clear tab color response",
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
        SheetsSheetCommand::Move {
            spreadsheet_id,
            sheet_id,
            index,
        } => {
            let request_body = move_sheet_request_body(sheet_id, index);
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
            .context("failed to move Google Sheets sheet")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Move sheet response",
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
        SheetsSheetCommand::Freeze {
            spreadsheet_id,
            sheet_id,
            rows,
            columns,
        } => {
            let request_body = freeze_sheet_request_body(sheet_id, rows, columns);
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
            .context("failed to freeze Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Freeze sheet response",
            )
        }
        SheetsSheetCommand::Resize {
            spreadsheet_id,
            sheet_id,
            rows,
            columns,
        } => {
            let request_body = resize_sheet_request_body(sheet_id, rows, columns);
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
            .context("failed to resize Google Sheets grid")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Resize sheet response",
            )
        }
        SheetsSheetCommand::AutoResize {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body =
                auto_resize_sheet_request_body(sheet_id, dimension, start_index, end_index)?;
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
            .context("failed to auto-resize Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Auto-resize sheet response",
            )
        }
        SheetsSheetCommand::SetDimensionSize {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
            pixel_size,
        } => {
            let request_body = set_dimension_size_sheet_request_body(
                sheet_id,
                dimension,
                start_index,
                end_index,
                pixel_size,
            )?;
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
            .context("failed to set Google Sheets row height or column width")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Set dimension size response",
            )
        }
        SheetsSheetCommand::HideDimension {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body = dimension_visibility_sheet_request_body(
                sheet_id,
                dimension,
                start_index,
                end_index,
                true,
            )?;
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
            .context("failed to hide Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Hide dimension response",
            )
        }
        SheetsSheetCommand::UnhideDimension {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body = dimension_visibility_sheet_request_body(
                sheet_id,
                dimension,
                start_index,
                end_index,
                false,
            )?;
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
            .context("failed to unhide Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Unhide dimension response",
            )
        }
        SheetsSheetCommand::GroupDimension {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body =
                dimension_group_sheet_request_body(sheet_id, dimension, start_index, end_index)?;
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
            .context("failed to group Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Group dimension response",
            )
        }
        SheetsSheetCommand::UngroupDimension {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body =
                dimension_ungroup_sheet_request_body(sheet_id, dimension, start_index, end_index)?;
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
            .context("failed to ungroup Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Ungroup dimension response",
            )
        }
        SheetsSheetCommand::CollapseDimensionGroup {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body = dimension_group_collapsed_sheet_request_body(
                sheet_id,
                dimension,
                start_index,
                end_index,
                true,
            )?;
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
            .context("failed to collapse Google Sheets row or column group")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Collapse dimension group response",
            )
        }
        SheetsSheetCommand::ExpandDimensionGroup {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body = dimension_group_collapsed_sheet_request_body(
                sheet_id,
                dimension,
                start_index,
                end_index,
                false,
            )?;
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
            .context("failed to expand Google Sheets row or column group")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Expand dimension group response",
            )
        }
        SheetsSheetCommand::InsertDimension {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
            inherit_from_before,
        } => {
            let request_body = insert_dimension_sheet_request_body(
                sheet_id,
                dimension,
                start_index,
                end_index,
                inherit_from_before,
            )?;
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
            .context("failed to insert Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Insert dimension response",
            )
        }
        SheetsSheetCommand::DeleteDimension {
            spreadsheet_id,
            sheet_id,
            dimension,
            start_index,
            end_index,
        } => {
            let request_body =
                delete_dimension_sheet_request_body(sheet_id, dimension, start_index, end_index)?;
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
            .context("failed to delete Google Sheets rows or columns")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Delete dimension response",
            )
        }
        SheetsSheetCommand::BasicFilter {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
        } => {
            let request_body = basic_filter_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
            )?;
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
            .context("failed to set Google Sheets basic filter")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Basic filter response",
            )
        }
        SheetsSheetCommand::ClearBasicFilter {
            spreadsheet_id,
            sheet_id,
        } => {
            let request_body = clear_basic_filter_sheet_request_body(sheet_id);
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
            .context("failed to clear Google Sheets basic filter")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Clear basic filter response",
            )
        }
        SheetsSheetCommand::Merge {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            merge_type,
        } => {
            let request_body = merge_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                merge_type,
            )?;
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
            .context("failed to merge Google Sheets cells")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Merge cells response",
            )
        }
        SheetsSheetCommand::Unmerge {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
        } => {
            let request_body =
                unmerge_sheet_request_body(sheet_id, start_row, end_row, start_column, end_column)?;
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
            .context("failed to unmerge Google Sheets cells")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Unmerge cells response",
            )
        }
        SheetsSheetCommand::SortRange {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            sort_column,
            order,
        } => {
            let request_body = sort_range_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                sort_column,
                order,
            )?;
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
            .context("failed to sort Google Sheets range")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Sort range response",
            )
        }
        SheetsSheetCommand::FindReplace {
            spreadsheet_id,
            find,
            replacement,
            sheet_id,
            match_case,
            match_entire_cell,
            search_by_regex,
            include_formulas,
        } => {
            let request_body = find_replace_sheet_request_body(
                find,
                replacement,
                sheet_id,
                match_case,
                match_entire_cell,
                search_by_regex,
                include_formulas,
            )?;
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
            .context("failed to find and replace Google Sheets text")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Find and replace response",
            )
        }
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
        } => {
            let request_body = copy_paste_sheet_request_body(
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
            )?;
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
            .context("failed to copy and paste Google Sheets cells")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Copy paste response",
            )
        }
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
        } => {
            let request_body = cut_paste_sheet_request_body(
                source_sheet_id,
                source_start_row,
                source_end_row,
                source_start_column,
                source_end_column,
                destination_sheet_id,
                destination_row,
                destination_column,
                paste_type,
            )?;
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
            .context("failed to cut and paste Google Sheets cells")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Cut paste response",
            )
        }
        SheetsSheetCommand::BackgroundColor {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            color,
        } => {
            let request_body = background_color_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                &color,
            )?;
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
            .context("failed to set Google Sheets background color")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Background color response",
            )
        }
        SheetsSheetCommand::TextColor {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            color,
        } => {
            let request_body = text_color_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                &color,
            )?;
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
            .context("failed to set Google Sheets text color")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Text color response",
            )
        }
        SheetsSheetCommand::FontSize {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            size,
        } => {
            let request_body = font_size_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                size,
            )?;
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
            .context("failed to set Google Sheets font size")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Font size response",
            )
        }
        SheetsSheetCommand::FontFamily {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            family,
        } => {
            let request_body = font_family_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                &family,
            )?;
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
            .context("failed to set Google Sheets font family")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Font family response",
            )
        }
        SheetsSheetCommand::NumberFormat {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            format_type,
            pattern,
        } => {
            let request_body = number_format_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                format_type,
                pattern.as_deref(),
            )?;
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
            .context("failed to set Google Sheets number format")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Number format response",
            )
        }
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
        } => {
            let request_body = borders_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                &edge,
                style,
                color.as_deref(),
            )?;
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
            .context("failed to set Google Sheets borders")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Borders response",
            )
        }
        SheetsSheetCommand::ClearFormat {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
        } => {
            let request_body = clear_format_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
            )?;
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
            .context("failed to clear Google Sheets cell formatting")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Clear format response",
            )
        }
        SheetsSheetCommand::Bold {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            off,
        } => {
            let request_body = bold_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                !off,
            )?;
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
            .context("failed to set Google Sheets bold text style")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Bold text response",
            )
        }
        SheetsSheetCommand::Italic {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            off,
        } => {
            let request_body = italic_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                !off,
            )?;
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
            .context("failed to set Google Sheets italic text style")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Italic text response",
            )
        }
        SheetsSheetCommand::Underline {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            off,
        } => {
            let request_body = underline_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                !off,
            )?;
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
            .context("failed to set Google Sheets underline text style")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Underline text response",
            )
        }
        SheetsSheetCommand::Strikethrough {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            off,
        } => {
            let request_body = strikethrough_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                !off,
            )?;
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
            .context("failed to set Google Sheets strikethrough text style")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Strikethrough text response",
            )
        }
        SheetsSheetCommand::HorizontalAlign {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            alignment,
        } => {
            let request_body = horizontal_align_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                alignment,
            )?;
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
            .context("failed to set Google Sheets horizontal alignment")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Horizontal align response",
            )
        }
        SheetsSheetCommand::VerticalAlign {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            alignment,
        } => {
            let request_body = vertical_align_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                alignment,
            )?;
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
            .context("failed to set Google Sheets vertical alignment")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Vertical align response",
            )
        }
        SheetsSheetCommand::TextWrap {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            strategy,
        } => {
            let request_body = text_wrap_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                strategy,
            )?;
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
            .context("failed to set Google Sheets text wrapping")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Text wrap response",
            )
        }
        SheetsSheetCommand::TextRotation {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            angle,
        } => {
            let request_body = text_rotation_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                angle,
            )?;
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
            .context("failed to set Google Sheets text rotation")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Text rotation response",
            )
        }
        SheetsSheetCommand::TextDirection {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            direction,
        } => {
            let request_body = text_direction_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                direction,
            )?;
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
            .context("failed to set Google Sheets text direction")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Text direction response",
            )
        }
        SheetsSheetCommand::Note {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            note,
            clear,
        } => {
            let request_body = note_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                note.as_deref(),
                clear,
            )?;
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
            .context("failed to update Google Sheets cell notes")?;
            write_json_line(out, &response, "failed to serialize Sheets Note response")
        }
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
        } => {
            let request_body = data_validation_list_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                &values,
                allow_invalid,
                hide_dropdown,
                input_message.as_deref(),
                clear,
            )?;
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
            .context("failed to update Google Sheets data validation")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Data validation response",
            )
        }
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
        } => {
            let request_body = data_validation_checkbox_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                checked_value.as_deref(),
                unchecked_value.as_deref(),
                allow_invalid,
                input_message.as_deref(),
                clear,
            )?;
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
            .context("failed to update Google Sheets checkbox validation")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Checkbox validation response",
            )
        }
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
        } => {
            let request_body = conditional_format_color_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                condition,
                &value,
                background_color.as_deref(),
                text_color.as_deref(),
                index,
            )?;
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
            .context("failed to add Google Sheets conditional format rule")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Conditional format response",
            )
        }
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
        } => {
            let request_body = conditional_format_update_sheet_request_body(
                sheet_id,
                index,
                start_row,
                end_row,
                start_column,
                end_column,
                condition,
                &value,
                background_color.as_deref(),
                text_color.as_deref(),
            )?;
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
            .context("failed to update Google Sheets conditional format rule")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Conditional format update response",
            )
        }
        SheetsSheetCommand::ConditionalFormatDelete {
            spreadsheet_id,
            sheet_id,
            index,
        } => {
            let request_body = conditional_format_delete_sheet_request_body(sheet_id, index);
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
            .context("failed to delete Google Sheets conditional format rule")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Conditional format delete response",
            )
        }
        SheetsSheetCommand::ConditionalFormatMove {
            spreadsheet_id,
            sheet_id,
            index,
            new_index,
        } => {
            let request_body =
                conditional_format_move_sheet_request_body(sheet_id, index, new_index);
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
            .context("failed to move Google Sheets conditional format rule")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Conditional format move response",
            )
        }
        SheetsSheetCommand::ProtectRange {
            spreadsheet_id,
            sheet_id,
            start_row,
            end_row,
            start_column,
            end_column,
            description,
            warning_only,
        } => {
            let request_body = protect_range_sheet_request_body(
                sheet_id,
                start_row,
                end_row,
                start_column,
                end_column,
                description.as_deref(),
                warning_only,
            )?;
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
            .context("failed to protect Google Sheets range")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Protect range response",
            )
        }
        SheetsSheetCommand::UnprotectRange {
            spreadsheet_id,
            protected_range_id,
        } => {
            let request_body = unprotect_range_sheet_request_body(protected_range_id);
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
            .context("failed to remove Google Sheets protected range")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Unprotect range response",
            )
        }
        SheetsSheetCommand::UpdateProtectedRange {
            spreadsheet_id,
            protected_range_id,
            description,
            warning_only,
            enforce,
        } => {
            let request_body = update_protected_range_sheet_request_body(
                protected_range_id,
                description.as_deref(),
                warning_only,
                enforce,
            )?;
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
            .context("failed to update Google Sheets protected range")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Update protected range response",
            )
        }
        SheetsSheetCommand::TabColor {
            spreadsheet_id,
            sheet_id,
            color,
        } => {
            let request_body = tab_color_sheet_request_body(sheet_id, &color)?;
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
            .context("failed to set Google Sheets tab color")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Tab color response",
            )
        }
        SheetsSheetCommand::ClearTabColor {
            spreadsheet_id,
            sheet_id,
        } => {
            let request_body = clear_tab_color_sheet_request_body(sheet_id);
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
            .context("failed to clear Google Sheets tab color")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Clear tab color response",
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
        SheetsValuesCommand::UpdateTable {
            spreadsheet_id,
            range,
            data,
            value_input_option,
        } => {
            let request_body = table_value_range(&data)?;
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
                .context("failed to update Google Sheets table")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Update table response",
            )
        }
        SheetsValuesCommand::UpdateRow {
            spreadsheet_id,
            range,
            values,
            value_input_option,
        } => {
            let request_body = row_value_range(values);
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
                .context("failed to update Google Sheets row")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Update row response",
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
        SheetsValuesCommand::UpdateTable {
            spreadsheet_id,
            range,
            data,
            value_input_option,
        } => {
            let request_body = table_value_range(&data)?;
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
            .context("failed to update Google Sheets table")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Update table response",
            )
        }
        SheetsValuesCommand::UpdateRow {
            spreadsheet_id,
            range,
            values,
            value_input_option,
        } => {
            let request_body = row_value_range(values);
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
            .context("failed to update Google Sheets row")?;
            write_json_line(
                out,
                &response,
                "failed to serialize Sheets Update row response",
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

fn move_sheet_request_body(sheet_id: i64, index: i64) -> serde_json::Value {
    serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": sheet_id,
                        "index": index
                    },
                    "fields": "index"
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

fn freeze_sheet_request_body(
    sheet_id: i64,
    rows: Option<i64>,
    columns: Option<i64>,
) -> serde_json::Value {
    let mut grid_properties = serde_json::Map::new();
    let mut fields = Vec::new();

    if let Some(rows) = rows {
        grid_properties.insert("frozenRowCount".to_string(), serde_json::json!(rows));
        fields.push("gridProperties.frozenRowCount");
    }
    if let Some(columns) = columns {
        grid_properties.insert("frozenColumnCount".to_string(), serde_json::json!(columns));
        fields.push("gridProperties.frozenColumnCount");
    }

    serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": sheet_id,
                        "gridProperties": grid_properties
                    },
                    "fields": fields.join(",")
                }
            }
        ]
    })
}

fn resize_sheet_request_body(
    sheet_id: i64,
    rows: Option<i64>,
    columns: Option<i64>,
) -> serde_json::Value {
    let mut grid_properties = serde_json::Map::new();
    let mut fields = Vec::new();

    if let Some(rows) = rows {
        grid_properties.insert("rowCount".to_string(), serde_json::json!(rows));
        fields.push("gridProperties.rowCount");
    }
    if let Some(columns) = columns {
        grid_properties.insert("columnCount".to_string(), serde_json::json!(columns));
        fields.push("gridProperties.columnCount");
    }

    serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": sheet_id,
                        "gridProperties": grid_properties
                    },
                    "fields": fields.join(",")
                }
            }
        ]
    })
}

fn auto_resize_sheet_request_body(
    sheet_id: i64,
    dimension: SheetsDimension,
    start_index: i64,
    end_index: i64,
) -> Result<serde_json::Value> {
    validate_dimension_range(start_index, end_index)?;
    let dimension = dimension_name(dimension);

    Ok(serde_json::json!({
        "requests": [
            {
                "autoResizeDimensions": {
                    "dimensions": {
                        "sheetId": sheet_id,
                        "dimension": dimension,
                        "startIndex": start_index,
                        "endIndex": end_index
                    }
                }
            }
        ]
    }))
}

fn set_dimension_size_sheet_request_body(
    sheet_id: i64,
    dimension: SheetsDimension,
    start_index: i64,
    end_index: i64,
    pixel_size: i64,
) -> Result<serde_json::Value> {
    validate_dimension_range(start_index, end_index)?;

    Ok(serde_json::json!({
        "requests": [
            {
                "updateDimensionProperties": {
                    "range": {
                        "sheetId": sheet_id,
                        "dimension": dimension_name(dimension),
                        "startIndex": start_index,
                        "endIndex": end_index
                    },
                    "properties": {
                        "pixelSize": pixel_size
                    },
                    "fields": "pixelSize"
                }
            }
        ]
    }))
}

fn dimension_visibility_sheet_request_body(
    sheet_id: i64,
    dimension: SheetsDimension,
    start_index: i64,
    end_index: i64,
    hidden: bool,
) -> Result<serde_json::Value> {
    validate_dimension_range(start_index, end_index)?;

    Ok(serde_json::json!({
        "requests": [
            {
                "updateDimensionProperties": {
                    "range": {
                        "sheetId": sheet_id,
                        "dimension": dimension_name(dimension),
                        "startIndex": start_index,
                        "endIndex": end_index
                    },
                    "properties": {
                        "hiddenByUser": hidden
                    },
                    "fields": "hiddenByUser"
                }
            }
        ]
    }))
}

fn dimension_group_sheet_request_body(
    sheet_id: i64,
    dimension: SheetsDimension,
    start_index: i64,
    end_index: i64,
) -> Result<serde_json::Value> {
    validate_dimension_range(start_index, end_index)?;

    Ok(serde_json::json!({
        "requests": [
            {
                "addDimensionGroup": {
                    "range": {
                        "sheetId": sheet_id,
                        "dimension": dimension_name(dimension),
                        "startIndex": start_index,
                        "endIndex": end_index
                    }
                }
            }
        ]
    }))
}

fn dimension_ungroup_sheet_request_body(
    sheet_id: i64,
    dimension: SheetsDimension,
    start_index: i64,
    end_index: i64,
) -> Result<serde_json::Value> {
    validate_dimension_range(start_index, end_index)?;

    Ok(serde_json::json!({
        "requests": [
            {
                "deleteDimensionGroup": {
                    "range": {
                        "sheetId": sheet_id,
                        "dimension": dimension_name(dimension),
                        "startIndex": start_index,
                        "endIndex": end_index
                    }
                }
            }
        ]
    }))
}

fn dimension_group_collapsed_sheet_request_body(
    sheet_id: i64,
    dimension: SheetsDimension,
    start_index: i64,
    end_index: i64,
    collapsed: bool,
) -> Result<serde_json::Value> {
    validate_dimension_range(start_index, end_index)?;

    Ok(serde_json::json!({
        "requests": [
            {
                "updateDimensionGroup": {
                    "dimensionGroup": {
                        "range": {
                            "sheetId": sheet_id,
                            "dimension": dimension_name(dimension),
                            "startIndex": start_index,
                            "endIndex": end_index
                        },
                        "collapsed": collapsed
                    },
                    "fields": "collapsed"
                }
            }
        ]
    }))
}

fn insert_dimension_sheet_request_body(
    sheet_id: i64,
    dimension: SheetsDimension,
    start_index: i64,
    end_index: i64,
    inherit_from_before: bool,
) -> Result<serde_json::Value> {
    validate_dimension_range(start_index, end_index)?;
    if inherit_from_before && start_index == 0 {
        bail!("--inherit-from-before requires --start-index greater than 0");
    }

    Ok(serde_json::json!({
        "requests": [
            {
                "insertDimension": {
                    "range": {
                        "sheetId": sheet_id,
                        "dimension": dimension_name(dimension),
                        "startIndex": start_index,
                        "endIndex": end_index
                    },
                    "inheritFromBefore": inherit_from_before
                }
            }
        ]
    }))
}

fn delete_dimension_sheet_request_body(
    sheet_id: i64,
    dimension: SheetsDimension,
    start_index: i64,
    end_index: i64,
) -> Result<serde_json::Value> {
    validate_dimension_range(start_index, end_index)?;

    Ok(serde_json::json!({
        "requests": [
            {
                "deleteDimension": {
                    "range": {
                        "sheetId": sheet_id,
                        "dimension": dimension_name(dimension),
                        "startIndex": start_index,
                        "endIndex": end_index
                    }
                }
            }
        ]
    }))
}

fn basic_filter_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
) -> Result<serde_json::Value> {
    if end_row <= start_row {
        bail!("--end-row must be greater than --start-row");
    }
    if end_column <= start_column {
        bail!("--end-column must be greater than --start-column");
    }

    Ok(serde_json::json!({
        "requests": [
            {
                "setBasicFilter": {
                    "filter": {
                        "range": {
                            "sheetId": sheet_id,
                            "startRowIndex": start_row,
                            "endRowIndex": end_row,
                            "startColumnIndex": start_column,
                            "endColumnIndex": end_column
                        }
                    }
                }
            }
        ]
    }))
}

fn clear_basic_filter_sheet_request_body(sheet_id: i64) -> serde_json::Value {
    serde_json::json!({
        "requests": [
            {
                "clearBasicFilter": {
                    "sheetId": sheet_id
                }
            }
        ]
    })
}

fn merge_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    merge_type: SheetsMergeType,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;

    let merge_type = match merge_type {
        SheetsMergeType::All => "MERGE_ALL",
        SheetsMergeType::Rows => "MERGE_ROWS",
        SheetsMergeType::Columns => "MERGE_COLUMNS",
    };

    Ok(serde_json::json!({
        "requests": [
            {
                "mergeCells": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "mergeType": merge_type
                }
            }
        ]
    }))
}

fn unmerge_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;

    Ok(serde_json::json!({
        "requests": [
            {
                "unmergeCells": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column)
                }
            }
        ]
    }))
}

fn sort_range_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    sort_column: i64,
    order: SheetsSortOrder,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    if sort_column < start_column || sort_column >= end_column {
        bail!("--sort-column must be inside the selected column range");
    }

    let sort_order = match order {
        SheetsSortOrder::Ascending => "ASCENDING",
        SheetsSortOrder::Descending => "DESCENDING",
    };

    Ok(serde_json::json!({
        "requests": [
            {
                "sortRange": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "sortSpecs": [
                        {
                            "dimensionIndex": sort_column,
                            "sortOrder": sort_order
                        }
                    ]
                }
            }
        ]
    }))
}

fn find_replace_sheet_request_body(
    find: String,
    replacement: String,
    sheet_id: Option<i64>,
    match_case: bool,
    match_entire_cell: bool,
    search_by_regex: bool,
    include_formulas: bool,
) -> Result<serde_json::Value> {
    if find.is_empty() {
        bail!("find text must not be empty");
    }

    let mut find_replace = serde_json::Map::new();
    find_replace.insert("find".to_string(), serde_json::Value::String(find));
    find_replace.insert(
        "replacement".to_string(),
        serde_json::Value::String(replacement),
    );
    find_replace.insert("matchCase".to_string(), serde_json::json!(match_case));
    find_replace.insert(
        "matchEntireCell".to_string(),
        serde_json::json!(match_entire_cell),
    );
    find_replace.insert(
        "searchByRegex".to_string(),
        serde_json::json!(search_by_regex),
    );
    find_replace.insert(
        "includeFormulas".to_string(),
        serde_json::json!(include_formulas),
    );
    if let Some(sheet_id) = sheet_id {
        find_replace.insert("sheetId".to_string(), serde_json::json!(sheet_id));
    } else {
        find_replace.insert("allSheets".to_string(), serde_json::json!(true));
    }

    Ok(serde_json::json!({
        "requests": [
            {
                "findReplace": find_replace
            }
        ]
    }))
}

fn copy_paste_sheet_request_body(
    source_sheet_id: i64,
    source_start_row: i64,
    source_end_row: i64,
    source_start_column: i64,
    source_end_column: i64,
    destination_sheet_id: i64,
    destination_start_row: i64,
    destination_end_row: i64,
    destination_start_column: i64,
    destination_end_column: i64,
    paste_type: SheetsPasteType,
    paste_orientation: SheetsPasteOrientation,
) -> Result<serde_json::Value> {
    validate_grid_range(
        source_start_row,
        source_end_row,
        source_start_column,
        source_end_column,
    )?;
    validate_grid_range(
        destination_start_row,
        destination_end_row,
        destination_start_column,
        destination_end_column,
    )?;

    Ok(serde_json::json!({
        "requests": [
            {
                "copyPaste": {
                    "source": grid_range(
                        source_sheet_id,
                        source_start_row,
                        source_end_row,
                        source_start_column,
                        source_end_column
                    ),
                    "destination": grid_range(
                        destination_sheet_id,
                        destination_start_row,
                        destination_end_row,
                        destination_start_column,
                        destination_end_column
                    ),
                    "pasteType": paste_type_name(paste_type),
                    "pasteOrientation": paste_orientation_name(paste_orientation)
                }
            }
        ]
    }))
}

fn cut_paste_sheet_request_body(
    source_sheet_id: i64,
    source_start_row: i64,
    source_end_row: i64,
    source_start_column: i64,
    source_end_column: i64,
    destination_sheet_id: i64,
    destination_row: i64,
    destination_column: i64,
    paste_type: SheetsPasteType,
) -> Result<serde_json::Value> {
    validate_grid_range(
        source_start_row,
        source_end_row,
        source_start_column,
        source_end_column,
    )?;

    Ok(serde_json::json!({
        "requests": [
            {
                "cutPaste": {
                    "source": grid_range(
                        source_sheet_id,
                        source_start_row,
                        source_end_row,
                        source_start_column,
                        source_end_column
                    ),
                    "destination": grid_coordinate(
                        destination_sheet_id,
                        destination_row,
                        destination_column
                    ),
                    "pasteType": paste_type_name(paste_type)
                }
            }
        ]
    }))
}

fn background_color_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    color: &str,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    let (red, green, blue) = parse_hex_color(color)?;
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {
                            "backgroundColor": {
                                "red": red,
                                "green": green,
                                "blue": blue
                            }
                        }
                    },
                    "fields": "userEnteredFormat.backgroundColor"
                }
            }
        ]
    }))
}

fn text_color_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    color: &str,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    let (red, green, blue) = parse_hex_color(color)?;
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "foregroundColor": {
                                    "red": red,
                                    "green": green,
                                    "blue": blue
                                }
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.foregroundColor"
                }
            }
        ]
    }))
}

fn conditional_format_color_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    condition: SheetsConditionalFormatCondition,
    value: &str,
    background_color: Option<&str>,
    text_color: Option<&str>,
    index: i64,
) -> Result<serde_json::Value> {
    let rule = conditional_format_color_rule(
        sheet_id,
        start_row,
        end_row,
        start_column,
        end_column,
        condition,
        value,
        background_color,
        text_color,
    )?;

    Ok(serde_json::json!({
        "requests": [
            {
                "addConditionalFormatRule": {
                    "rule": rule,
                    "index": index
                }
            }
        ]
    }))
}

fn conditional_format_update_sheet_request_body(
    sheet_id: i64,
    index: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    condition: SheetsConditionalFormatCondition,
    value: &str,
    background_color: Option<&str>,
    text_color: Option<&str>,
) -> Result<serde_json::Value> {
    let rule = conditional_format_color_rule(
        sheet_id,
        start_row,
        end_row,
        start_column,
        end_column,
        condition,
        value,
        background_color,
        text_color,
    )?;

    Ok(serde_json::json!({
        "requests": [
            {
                "updateConditionalFormatRule": {
                    "sheetId": sheet_id,
                    "index": index,
                    "rule": rule
                }
            }
        ]
    }))
}

fn conditional_format_color_rule(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    condition: SheetsConditionalFormatCondition,
    value: &str,
    background_color: Option<&str>,
    text_color: Option<&str>,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    if value.trim().is_empty() {
        bail!("--value must not be empty");
    }
    if background_color.is_none() && text_color.is_none() {
        bail!("at least one of --background-color or --text-color is required");
    }

    let mut format = serde_json::json!({});
    if let Some(background_color) = background_color {
        let (red, green, blue) = parse_hex_color(background_color)?;
        format["backgroundColor"] = serde_json::json!({
            "red": red,
            "green": green,
            "blue": blue
        });
    }
    if let Some(text_color) = text_color {
        let (red, green, blue) = parse_hex_color(text_color)?;
        format["textFormat"] = serde_json::json!({
            "foregroundColor": {
                "red": red,
                "green": green,
                "blue": blue
            }
        });
    }

    Ok(serde_json::json!({
        "ranges": [
            grid_range(sheet_id, start_row, end_row, start_column, end_column)
        ],
        "booleanRule": {
            "condition": {
                "type": conditional_format_condition_name(condition),
                "values": [
                    {
                        "userEnteredValue": value
                    }
                ]
            },
            "format": format
        }
    }))
}

fn conditional_format_delete_sheet_request_body(sheet_id: i64, index: i64) -> serde_json::Value {
    serde_json::json!({
        "requests": [
            {
                "deleteConditionalFormatRule": {
                    "sheetId": sheet_id,
                    "index": index
                }
            }
        ]
    })
}

fn conditional_format_move_sheet_request_body(
    sheet_id: i64,
    index: i64,
    new_index: i64,
) -> serde_json::Value {
    serde_json::json!({
        "requests": [
            {
                "updateConditionalFormatRule": {
                    "sheetId": sheet_id,
                    "index": index,
                    "newIndex": new_index
                }
            }
        ]
    })
}

fn protect_range_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    description: Option<&str>,
    warning_only: bool,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    if description.is_some_and(|description| description.trim().is_empty()) {
        bail!("--description must not be empty");
    }

    let mut protected_range = serde_json::json!({
        "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
        "warningOnly": warning_only
    });
    if let Some(description) = description {
        protected_range["description"] = serde_json::json!(description);
    }

    Ok(serde_json::json!({
        "requests": [
            {
                "addProtectedRange": {
                    "protectedRange": protected_range
                }
            }
        ]
    }))
}

fn unprotect_range_sheet_request_body(protected_range_id: i64) -> serde_json::Value {
    serde_json::json!({
        "requests": [
            {
                "deleteProtectedRange": {
                    "protectedRangeId": protected_range_id
                }
            }
        ]
    })
}

fn update_protected_range_sheet_request_body(
    protected_range_id: i64,
    description: Option<&str>,
    warning_only: bool,
    enforce: bool,
) -> Result<serde_json::Value> {
    if description.is_some_and(|description| description.trim().is_empty()) {
        bail!("--description must not be empty");
    }

    let mut protected_range = serde_json::json!({
        "protectedRangeId": protected_range_id
    });
    let mut fields = Vec::new();

    if let Some(description) = description {
        protected_range["description"] = serde_json::json!(description);
        fields.push("description");
    }
    if warning_only || enforce {
        protected_range["warningOnly"] = serde_json::json!(warning_only);
        fields.push("warningOnly");
    }
    if fields.is_empty() {
        bail!("provide --description, --warning-only, or --enforce");
    }

    Ok(serde_json::json!({
        "requests": [
            {
                "updateProtectedRange": {
                    "protectedRange": protected_range,
                    "fields": fields.join(",")
                }
            }
        ]
    }))
}

fn font_size_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    size: i64,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "fontSize": size
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.fontSize"
                }
            }
        ]
    }))
}

fn font_family_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    family: &str,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    if family.trim().is_empty() {
        bail!("--family must not be empty");
    }
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "fontFamily": family
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.fontFamily"
                }
            }
        ]
    }))
}

fn number_format_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    format_type: SheetsNumberFormatType,
    pattern: Option<&str>,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    if let Some(pattern) = pattern {
        if pattern.trim().is_empty() {
            bail!("--pattern must not be empty");
        }
    }

    let mut number_format = serde_json::json!({
        "type": number_format_type_name(format_type)
    });
    if let Some(pattern) = pattern {
        number_format["pattern"] = serde_json::Value::String(pattern.to_string());
    }

    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {
                            "numberFormat": number_format
                        }
                    },
                    "fields": "userEnteredFormat.numberFormat"
                }
            }
        ]
    }))
}

fn borders_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    edges: &[SheetsBorderEdge],
    style: SheetsBorderStyle,
    color: Option<&str>,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    let border = border_value(style, color)?;
    let mut update_borders = serde_json::json!({
        "range": grid_range(sheet_id, start_row, end_row, start_column, end_column)
    });

    let mut set_edge = |name: &str| {
        update_borders[name] = border.clone();
    };
    let effective_edges = if edges.is_empty() {
        &[SheetsBorderEdge::All][..]
    } else {
        edges
    };
    for edge in effective_edges {
        match edge {
            SheetsBorderEdge::All => {
                set_edge("top");
                set_edge("bottom");
                set_edge("left");
                set_edge("right");
                set_edge("innerHorizontal");
                set_edge("innerVertical");
            }
            SheetsBorderEdge::Outer => {
                set_edge("top");
                set_edge("bottom");
                set_edge("left");
                set_edge("right");
            }
            SheetsBorderEdge::Inner => {
                set_edge("innerHorizontal");
                set_edge("innerVertical");
            }
            SheetsBorderEdge::Top => set_edge("top"),
            SheetsBorderEdge::Bottom => set_edge("bottom"),
            SheetsBorderEdge::Left => set_edge("left"),
            SheetsBorderEdge::Right => set_edge("right"),
        }
    }

    Ok(serde_json::json!({
        "requests": [
            {
                "updateBorders": update_borders
            }
        ]
    }))
}

fn clear_format_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {}
                    },
                    "fields": "userEnteredFormat"
                }
            }
        ]
    }))
}

fn border_value(style: SheetsBorderStyle, color: Option<&str>) -> Result<serde_json::Value> {
    let mut border = serde_json::json!({
        "style": border_style_name(style)
    });
    if let Some(color) = color {
        let (red, green, blue) = parse_hex_color(color)?;
        border["color"] = serde_json::json!({
            "red": red,
            "green": green,
            "blue": blue
        });
    }
    Ok(border)
}

fn bold_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    bold: bool,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "bold": bold
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.bold"
                }
            }
        ]
    }))
}

fn italic_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    italic: bool,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "italic": italic
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.italic"
                }
            }
        ]
    }))
}

fn underline_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    underline: bool,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "underline": underline
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.underline"
                }
            }
        ]
    }))
}

fn strikethrough_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    strikethrough: bool,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "strikethrough": strikethrough
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.strikethrough"
                }
            }
        ]
    }))
}

fn horizontal_align_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    alignment: SheetsHorizontalAlignment,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {
                            "horizontalAlignment": horizontal_alignment_name(alignment)
                        }
                    },
                    "fields": "userEnteredFormat.horizontalAlignment"
                }
            }
        ]
    }))
}

fn vertical_align_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    alignment: SheetsVerticalAlignment,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {
                            "verticalAlignment": vertical_alignment_name(alignment)
                        }
                    },
                    "fields": "userEnteredFormat.verticalAlignment"
                }
            }
        ]
    }))
}

fn text_wrap_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    strategy: SheetsWrapStrategy,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {
                            "wrapStrategy": wrap_strategy_name(strategy)
                        }
                    },
                    "fields": "userEnteredFormat.wrapStrategy"
                }
            }
        ]
    }))
}

fn text_rotation_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    angle: i64,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {
                            "textRotation": {
                                "angle": angle
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textRotation"
                }
            }
        ]
    }))
}

fn text_direction_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    direction: SheetsTextDirection,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "userEnteredFormat": {
                            "textDirection": text_direction_name(direction)
                        }
                    },
                    "fields": "userEnteredFormat.textDirection"
                }
            }
        ]
    }))
}

fn note_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    note: Option<&str>,
    clear: bool,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    if clear {
        return Ok(serde_json::json!({
            "requests": [
                {
                    "repeatCell": {
                        "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                        "cell": {},
                        "fields": "note"
                    }
                }
            ]
        }));
    }

    let Some(note) = note else {
        bail!("note text is required unless --clear is passed");
    };
    if note.trim().is_empty() {
        bail!("note text must not be empty");
    }
    Ok(serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": grid_range(sheet_id, start_row, end_row, start_column, end_column),
                    "cell": {
                        "note": note
                    },
                    "fields": "note"
                }
            }
        ]
    }))
}

fn data_validation_list_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    values: &[String],
    allow_invalid: bool,
    hide_dropdown: bool,
    input_message: Option<&str>,
    clear: bool,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    let range = grid_range(sheet_id, start_row, end_row, start_column, end_column);
    if clear {
        return Ok(serde_json::json!({
            "requests": [
                {
                    "setDataValidation": {
                        "range": range
                    }
                }
            ]
        }));
    }

    if values.is_empty() {
        bail!("at least one --value is required unless --clear is passed");
    }
    if values.iter().any(|value| value.trim().is_empty()) {
        bail!("data validation values must not be empty");
    }
    if input_message.is_some_and(|message| message.trim().is_empty()) {
        bail!("--input-message must not be empty");
    }

    let condition_values = values
        .iter()
        .map(|value| serde_json::json!({ "userEnteredValue": value }))
        .collect::<Vec<_>>();
    let mut rule = serde_json::json!({
        "condition": {
            "type": "ONE_OF_LIST",
            "values": condition_values
        },
        "strict": !allow_invalid,
        "showCustomUi": !hide_dropdown
    });
    if let Some(input_message) = input_message {
        rule["inputMessage"] = serde_json::json!(input_message);
    }

    Ok(serde_json::json!({
        "requests": [
            {
                "setDataValidation": {
                    "range": range,
                    "rule": rule
                }
            }
        ]
    }))
}

fn data_validation_checkbox_sheet_request_body(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
    checked_value: Option<&str>,
    unchecked_value: Option<&str>,
    allow_invalid: bool,
    input_message: Option<&str>,
    clear: bool,
) -> Result<serde_json::Value> {
    validate_grid_range(start_row, end_row, start_column, end_column)?;
    let range = grid_range(sheet_id, start_row, end_row, start_column, end_column);
    if clear {
        return Ok(serde_json::json!({
            "requests": [
                {
                    "setDataValidation": {
                        "range": range
                    }
                }
            ]
        }));
    }

    if checked_value.is_some_and(|value| value.trim().is_empty()) {
        bail!("--checked-value must not be empty");
    }
    if unchecked_value.is_some_and(|value| value.trim().is_empty()) {
        bail!("--unchecked-value must not be empty");
    }
    if input_message.is_some_and(|message| message.trim().is_empty()) {
        bail!("--input-message must not be empty");
    }

    let mut condition = serde_json::json!({
        "type": "BOOLEAN"
    });
    let mut condition_values = Vec::new();
    if let Some(checked_value) = checked_value {
        condition_values.push(serde_json::json!({ "userEnteredValue": checked_value }));
    }
    if let Some(unchecked_value) = unchecked_value {
        condition_values.push(serde_json::json!({ "userEnteredValue": unchecked_value }));
    }
    if !condition_values.is_empty() {
        condition["values"] = serde_json::json!(condition_values);
    }

    let mut rule = serde_json::json!({
        "condition": condition,
        "strict": !allow_invalid
    });
    if let Some(input_message) = input_message {
        rule["inputMessage"] = serde_json::json!(input_message);
    }

    Ok(serde_json::json!({
        "requests": [
            {
                "setDataValidation": {
                    "range": range,
                    "rule": rule
                }
            }
        ]
    }))
}

fn validate_grid_range(
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
) -> Result<()> {
    if end_row <= start_row {
        bail!("--end-row must be greater than --start-row");
    }
    if end_column <= start_column {
        bail!("--end-column must be greater than --start-column");
    }

    Ok(())
}

fn validate_dimension_range(start_index: i64, end_index: i64) -> Result<()> {
    if end_index <= start_index {
        bail!("--end-index must be greater than --start-index");
    }

    Ok(())
}

fn dimension_name(dimension: SheetsDimension) -> &'static str {
    match dimension {
        SheetsDimension::Rows => "ROWS",
        SheetsDimension::Columns => "COLUMNS",
    }
}

fn paste_type_name(paste_type: SheetsPasteType) -> &'static str {
    match paste_type {
        SheetsPasteType::Normal => "PASTE_NORMAL",
        SheetsPasteType::Values => "PASTE_VALUES",
        SheetsPasteType::Format => "PASTE_FORMAT",
        SheetsPasteType::Formula => "PASTE_FORMULA",
        SheetsPasteType::NoBorders => "PASTE_NO_BORDERS",
        SheetsPasteType::DataValidation => "PASTE_DATA_VALIDATION",
        SheetsPasteType::ConditionalFormatting => "PASTE_CONDITIONAL_FORMATTING",
    }
}

fn paste_orientation_name(paste_orientation: SheetsPasteOrientation) -> &'static str {
    match paste_orientation {
        SheetsPasteOrientation::Normal => "NORMAL",
        SheetsPasteOrientation::Transposed => "TRANSPOSE",
    }
}

fn horizontal_alignment_name(alignment: SheetsHorizontalAlignment) -> &'static str {
    match alignment {
        SheetsHorizontalAlignment::Left => "LEFT",
        SheetsHorizontalAlignment::Center => "CENTER",
        SheetsHorizontalAlignment::Right => "RIGHT",
    }
}

fn vertical_alignment_name(alignment: SheetsVerticalAlignment) -> &'static str {
    match alignment {
        SheetsVerticalAlignment::Top => "TOP",
        SheetsVerticalAlignment::Middle => "MIDDLE",
        SheetsVerticalAlignment::Bottom => "BOTTOM",
    }
}

fn wrap_strategy_name(strategy: SheetsWrapStrategy) -> &'static str {
    match strategy {
        SheetsWrapStrategy::Overflow => "OVERFLOW_CELL",
        SheetsWrapStrategy::Wrap => "WRAP",
        SheetsWrapStrategy::Clip => "CLIP",
    }
}

fn text_direction_name(direction: SheetsTextDirection) -> &'static str {
    match direction {
        SheetsTextDirection::LeftToRight => "LEFT_TO_RIGHT",
        SheetsTextDirection::RightToLeft => "RIGHT_TO_LEFT",
    }
}

fn number_format_type_name(format_type: SheetsNumberFormatType) -> &'static str {
    match format_type {
        SheetsNumberFormatType::Text => "TEXT",
        SheetsNumberFormatType::Number => "NUMBER",
        SheetsNumberFormatType::Percent => "PERCENT",
        SheetsNumberFormatType::Currency => "CURRENCY",
        SheetsNumberFormatType::Date => "DATE",
        SheetsNumberFormatType::Time => "TIME",
        SheetsNumberFormatType::DateTime => "DATE_TIME",
        SheetsNumberFormatType::Scientific => "SCIENTIFIC",
    }
}

fn border_style_name(style: SheetsBorderStyle) -> &'static str {
    match style {
        SheetsBorderStyle::None => "NONE",
        SheetsBorderStyle::Solid => "SOLID",
        SheetsBorderStyle::SolidMedium => "SOLID_MEDIUM",
        SheetsBorderStyle::SolidThick => "SOLID_THICK",
        SheetsBorderStyle::Dashed => "DASHED",
        SheetsBorderStyle::Dotted => "DOTTED",
        SheetsBorderStyle::Double => "DOUBLE",
    }
}

fn conditional_format_condition_name(condition: SheetsConditionalFormatCondition) -> &'static str {
    match condition {
        SheetsConditionalFormatCondition::NumberGreater => "NUMBER_GREATER",
        SheetsConditionalFormatCondition::NumberLess => "NUMBER_LESS",
        SheetsConditionalFormatCondition::Equal => "NUMBER_EQ",
        SheetsConditionalFormatCondition::NotEqual => "NUMBER_NOT_EQ",
        SheetsConditionalFormatCondition::TextContains => "TEXT_CONTAINS",
        SheetsConditionalFormatCondition::TextEq => "TEXT_EQ",
        SheetsConditionalFormatCondition::CustomFormula => "CUSTOM_FORMULA",
    }
}

fn grid_range(
    sheet_id: i64,
    start_row: i64,
    end_row: i64,
    start_column: i64,
    end_column: i64,
) -> serde_json::Value {
    serde_json::json!({
        "sheetId": sheet_id,
        "startRowIndex": start_row,
        "endRowIndex": end_row,
        "startColumnIndex": start_column,
        "endColumnIndex": end_column
    })
}

fn grid_coordinate(sheet_id: i64, row_index: i64, column_index: i64) -> serde_json::Value {
    serde_json::json!({
        "sheetId": sheet_id,
        "rowIndex": row_index,
        "columnIndex": column_index
    })
}

fn tab_color_sheet_request_body(sheet_id: i64, color: &str) -> Result<serde_json::Value> {
    let (red, green, blue) = parse_hex_color(color)?;

    Ok(serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": sheet_id,
                        "tabColor": {
                            "red": red,
                            "green": green,
                            "blue": blue
                        }
                    },
                    "fields": "tabColor"
                }
            }
        ]
    }))
}

fn clear_tab_color_sheet_request_body(sheet_id: i64) -> serde_json::Value {
    serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": sheet_id
                    },
                    "fields": "tabColor"
                }
            }
        ]
    })
}

fn parse_hex_color(color: &str) -> Result<(f64, f64, f64)> {
    let hex = color.strip_prefix('#').unwrap_or(color);
    if hex.len() != 6 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        bail!("color must be a hex RGB value like #3366cc or 3366cc");
    }

    let red = u8::from_str_radix(&hex[0..2], 16).context("failed to parse red channel")?;
    let green = u8::from_str_radix(&hex[2..4], 16).context("failed to parse green channel")?;
    let blue = u8::from_str_radix(&hex[4..6], 16).context("failed to parse blue channel")?;

    Ok((
        f64::from(red) / 255.0,
        f64::from(green) / 255.0,
        f64::from(blue) / 255.0,
    ))
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
