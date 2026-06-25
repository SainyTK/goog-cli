use std::io::{Read, Write};

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::cli::DocsCommand;
use crate::docs::{
    batch_update_document, get_document, map::build_document_map, map::search_document_text,
    map::DocumentMap, map::DocumentMapEntry, map::DocumentMapEntryKind, map::DocumentRange,
    BatchUpdateDocumentOptions, GetDocumentOptions,
};

pub fn run<S: AccountStore>(cmd: DocsCommand, client: &AuthClient<'_, S>) -> Result<()> {
    match cmd {
        DocsCommand::Map { document_id, json } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_map_to(
                client,
                document_id,
                json,
                &mut std::io::stdout(),
                None,
            ))
        }
        DocsCommand::SearchText {
            document_id,
            text,
            json,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_search_text_to(
                client,
                document_id,
                text,
                json,
                &mut std::io::stdout(),
                None,
            ))
        }
        DocsCommand::GetContent {
            document_id,
            index,
            entry,
            page,
            line,
            heading,
            json,
        } => {
            let selector = content_selector(index, entry, page, line, heading)?;
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_get_content_to(
                client,
                document_id,
                selector,
                json,
                &mut std::io::stdout(),
                None,
            ))
        }
        DocsCommand::InsertText {
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
        } => {
            let selector = insert_text_selector(
                index,
                entry,
                page,
                line,
                after_heading,
                before_heading,
                after_text,
                before_text,
            )?;
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_insert_text_to(
                client,
                InsertTextCommand {
                    document_id,
                    text,
                    selector,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
            ))
        }
        DocsCommand::ReplaceText {
            document_id,
            old_text,
            new_text,
            match_number,
            all,
            dry_run,
            json,
            required_revision_id,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_replace_text_to(
                client,
                ReplaceTextCommand {
                    document_id,
                    old_text,
                    new_text,
                    match_number,
                    all,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
            ))
        }
        DocsCommand::Get {
            document_id,
            fields,
            include_tabs_content,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_get_to(
                client,
                document_id,
                fields,
                include_tabs_content,
                &mut std::io::stdout(),
                None,
            ))
        }
        DocsCommand::BatchUpdate {
            document_id,
            requests,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            let mut stdin = std::io::stdin();
            runtime.block_on(run_batch_update_to(
                client,
                document_id,
                requests,
                &mut stdin,
                &mut std::io::stdout(),
                None,
            ))
        }
    }
}

pub(super) async fn run_map_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, document_id, documents_url).await?;
    if json {
        write_json_line(out, &document_map, "failed to serialize Docs Document Map")
    } else {
        write_document_map_table(out, &document_map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ContentSelector {
    Index(i64),
    Entry(usize),
    PageLine { page: usize, line: usize },
    Heading(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InsertTextSelector {
    Index(i64),
    Entry(usize),
    PageLine { page: usize, line: usize },
    AfterHeading(String),
    BeforeHeading(String),
    AfterText(String),
    BeforeText(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InsertTextCommand {
    pub document_id: String,
    pub text: String,
    pub selector: InsertTextSelector,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReplaceTextCommand {
    pub document_id: String,
    pub old_text: String,
    pub new_text: String,
    pub match_number: Option<usize>,
    pub all: bool,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

pub(super) async fn run_search_text_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    text: String,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, document_id, documents_url).await?;
    let ranges = search_document_text(&document_map, &text);
    if json {
        write_json_line(out, &ranges, "failed to serialize Docs text matches")
    } else {
        write_search_text_table(out, &ranges)
    }
}

pub(super) async fn run_get_content_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    selector: ContentSelector,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, document_id, documents_url).await?;
    let entry = resolve_content_entry(&document_map, &selector)?;
    if json {
        write_json_line(out, entry, "failed to serialize Docs content entry")
    } else {
        write_document_map_table(out, &document_map_with_entry(&document_map, entry))
    }
}

pub(super) async fn run_insert_text_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: InsertTextCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let resolved = resolve_insert_text_location(&document_map, &command.selector)?;
    let request_body = insert_text_request_body(
        resolved.location.index,
        &command.text,
        command.required_revision_id.as_deref(),
    );
    let preview = InsertTextPreview {
        before: resolved.preview_before.clone(),
        after: insert_preview_text(
            &resolved.preview_before,
            resolved.preview_offset,
            &command.text,
        ),
    };

    if command.dry_run {
        let dry_run = InsertTextDryRun {
            revision_id: document_map.revision_id.clone(),
            location: resolved.location,
            request_body,
            preview,
        };
        if command.json {
            write_json_line(out, &dry_run, "failed to serialize Docs insert-text dry run")
        } else {
            write_insert_text_preview(out, &dry_run)
        }
    } else {
        let options =
            batch_update_document_options(command.document_id, request_body, documents_url);
        let response = batch_update_document(client, &options)
            .await
            .context("failed to apply Google Docs insert-text")?;
        write_json_line(out, &response, "failed to serialize Docs insert-text response")
    }
}

pub(super) async fn run_replace_text_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: ReplaceTextCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let ranges = resolve_replace_text_ranges(&document_map, &command)?;
    let request_body = replace_text_request_body(
        &ranges,
        &command.new_text,
        command.required_revision_id.as_deref(),
    );
    let preview = replace_text_preview(
        &document_map,
        &ranges,
        &command.old_text,
        &command.new_text,
    );

    if command.dry_run {
        let dry_run = ReplaceTextDryRun {
            revision_id: document_map.revision_id.clone(),
            ranges,
            request_body,
            preview,
        };
        if command.json {
            write_json_line(out, &dry_run, "failed to serialize Docs replace-text dry run")
        } else {
            write_replace_text_preview(out, &dry_run)
        }
    } else {
        let options =
            batch_update_document_options(command.document_id, request_body, documents_url);
        let response = batch_update_document(client, &options)
            .await
            .context("failed to apply Google Docs replace-text")?;
        write_json_line(out, &response, "failed to serialize Docs replace-text response")
    }
}

pub(super) async fn run_get_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    fields: Option<String>,
    include_tabs_content: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let options = get_document_options(document_id, fields, include_tabs_content, documents_url);

    let document = get_document(client, &options)
        .await
        .context("failed to fetch Google Docs Document")?;
    write_json_line(out, &document, "failed to serialize Docs Document")
}

pub(super) async fn run_batch_update_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    requests: String,
    input: &mut impl Read,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let request_body = read_request_body(&requests, input)?;
    let options = batch_update_document_options(document_id, request_body, documents_url);

    let response = batch_update_document(client, &options)
        .await
        .context("failed to apply Google Docs Batch Update")?;
    write_json_line(
        out,
        &response,
        "failed to serialize Docs Batch Update response",
    )
}

fn read_request_body(path_or_stdin: &str, input: &mut impl Read) -> Result<serde_json::Value> {
    let (body, request_source) = if path_or_stdin == "-" {
        let mut body = String::new();
        input
            .read_to_string(&mut body)
            .context("failed to read Google Docs Batch Update request body from stdin")?;
        (body, "stdin".to_string())
    } else {
        let body = std::fs::read_to_string(path_or_stdin).with_context(|| {
            format!("failed to read Google Docs Batch Update request body: {path_or_stdin}")
        })?;
        (body, path_or_stdin.to_string())
    };

    serde_json::from_str(&body).with_context(|| {
        format!("failed to parse Google Docs Batch Update request body from {request_source}")
    })
}

async fn get_document_map<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    documents_url: Option<&str>,
) -> Result<DocumentMap> {
    let options = get_document_options(document_id, None, true, documents_url);
    let document = get_document(client, &options)
        .await
        .context("failed to fetch Google Docs Document")?;
    Ok(build_document_map(&document))
}

fn content_selector(
    index: Option<i64>,
    entry: Option<usize>,
    page: Option<usize>,
    line: Option<usize>,
    heading: Option<String>,
) -> Result<ContentSelector> {
    let selector_count = usize::from(index.is_some())
        + usize::from(entry.is_some())
        + usize::from(page.is_some() || line.is_some())
        + usize::from(heading.is_some());
    if selector_count != 1 {
        bail!("provide exactly one content selector: --index, --entry, --page with --line, or --heading");
    }

    if let Some(index) = index {
        return Ok(ContentSelector::Index(index));
    }
    if let Some(entry) = entry {
        return Ok(ContentSelector::Entry(entry));
    }
    if page.is_some() || line.is_some() {
        let Some(page) = page else {
            bail!("--page and --line must be provided together");
        };
        let Some(line) = line else {
            bail!("--page and --line must be provided together");
        };
        return Ok(ContentSelector::PageLine { page, line });
    }
    if let Some(heading) = heading {
        return Ok(ContentSelector::Heading(heading));
    }

    unreachable!("selector count checked above")
}

fn insert_text_selector(
    index: Option<i64>,
    entry: Option<usize>,
    page: Option<usize>,
    line: Option<usize>,
    after_heading: Option<String>,
    before_heading: Option<String>,
    after_text: Option<String>,
    before_text: Option<String>,
) -> Result<InsertTextSelector> {
    let selector_count = usize::from(index.is_some())
        + usize::from(entry.is_some())
        + usize::from(page.is_some() || line.is_some())
        + usize::from(after_heading.is_some())
        + usize::from(before_heading.is_some())
        + usize::from(after_text.is_some())
        + usize::from(before_text.is_some());
    if selector_count != 1 {
        bail!("provide exactly one insert-text selector: --index, --entry, --page with --line, --after-heading, --before-heading, --after-text, or --before-text");
    }

    if let Some(index) = index {
        return Ok(InsertTextSelector::Index(index));
    }
    if let Some(entry) = entry {
        return Ok(InsertTextSelector::Entry(entry));
    }
    if page.is_some() || line.is_some() {
        let Some(page) = page else {
            bail!("--page and --line must be provided together");
        };
        let Some(line) = line else {
            bail!("--page and --line must be provided together");
        };
        return Ok(InsertTextSelector::PageLine { page, line });
    }
    if let Some(heading) = after_heading {
        return Ok(InsertTextSelector::AfterHeading(heading));
    }
    if let Some(heading) = before_heading {
        return Ok(InsertTextSelector::BeforeHeading(heading));
    }
    if let Some(text) = after_text {
        return Ok(InsertTextSelector::AfterText(text));
    }
    if let Some(text) = before_text {
        return Ok(InsertTextSelector::BeforeText(text));
    }

    unreachable!("selector count checked above")
}

fn document_map_with_entry(document_map: &DocumentMap, entry: &DocumentMapEntry) -> DocumentMap {
    DocumentMap {
        document_id: document_map.document_id.clone(),
        title: document_map.title.clone(),
        revision_id: document_map.revision_id.clone(),
        entries: vec![entry.clone()],
        document_locations: vec![entry.location.clone()],
        text_blocks: Vec::new(),
    }
}

fn resolve_content_entry<'a>(
    document_map: &'a DocumentMap,
    selector: &ContentSelector,
) -> Result<&'a DocumentMapEntry> {
    match selector {
        ContentSelector::Index(index) => document_map
            .entries
            .iter()
            .filter(|entry| entry.location.index.is_some_and(|entry_index| entry_index <= *index))
            .max_by_key(|entry| entry.location.index)
            .with_context(|| format!("no content found at Google Docs index {index}")),
        ContentSelector::Entry(entry_number) => document_map
            .entries
            .iter()
            .find(|entry| entry.entry == *entry_number)
            .with_context(|| format!("Document Map entry {entry_number} was not found")),
        ContentSelector::PageLine { page, line } => document_map
            .entries
            .iter()
            .find(|entry| {
                entry.location.page == Some(*page) && entry.location.content_line == *line
            })
            .with_context(|| format!("no content found at page {page}, line {line}")),
        ContentSelector::Heading(heading) => resolve_heading(document_map, heading),
    }
}

fn resolve_heading<'a>(
    document_map: &'a DocumentMap,
    heading: &str,
) -> Result<&'a DocumentMapEntry> {
    let matches = document_map
        .entries
        .iter()
        .filter(|entry| entry.kind == DocumentMapEntryKind::Heading && entry.preview == heading)
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [entry] => Ok(entry),
        [] => bail!("heading selector {heading:?} did not match any Document Map entries"),
        candidates => {
            let candidate_list = candidates
                .iter()
                .map(|entry| {
                    format!(
                        "entry {} index {} page {} line {} preview {}",
                        entry.entry,
                        display_optional(entry.location.index),
                        display_optional(entry.location.page),
                        entry.location.content_line,
                        entry.preview
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            bail!("ambiguous heading selector {heading:?}; candidates: {candidate_list}")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InsertTextDryRun {
    revision_id: Option<String>,
    location: crate::docs::map::DocumentLocation,
    request_body: serde_json::Value,
    preview: InsertTextPreview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InsertTextPreview {
    before: String,
    after: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReplaceTextDryRun {
    revision_id: Option<String>,
    ranges: Vec<DocumentRange>,
    request_body: serde_json::Value,
    preview: ReplaceTextPreview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReplaceTextPreview {
    changes: Vec<ReplaceTextPreviewChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReplaceTextPreviewChange {
    range: DocumentRange,
    before: String,
    after: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedInsertTextLocation {
    location: crate::docs::map::DocumentLocation,
    preview_before: String,
    preview_offset: usize,
}

fn resolve_insert_text_location(
    document_map: &DocumentMap,
    selector: &InsertTextSelector,
) -> Result<ResolvedInsertTextLocation> {
    match selector {
        InsertTextSelector::Index(index) => resolved_for_index(document_map, *index),
        InsertTextSelector::Entry(entry_number) => {
            let entry =
                resolve_content_entry(document_map, &ContentSelector::Entry(*entry_number))?;
            resolved_for_entry_start(document_map, entry)
        }
        InsertTextSelector::PageLine { page, line } => {
            let entry = resolve_content_entry(
                document_map,
                &ContentSelector::PageLine {
                    page: *page,
                    line: *line,
                },
            )?;
            resolved_for_entry_start(document_map, entry)
        }
        InsertTextSelector::BeforeHeading(heading) => {
            let entry = resolve_heading(document_map, heading)?;
            resolved_for_entry_start(document_map, entry)
        }
        InsertTextSelector::AfterHeading(heading) => {
            let entry = resolve_heading(document_map, heading)?;
            resolved_for_entry_end(document_map, entry)
        }
        InsertTextSelector::BeforeText(text) => {
            let range = resolve_text_anchor(document_map, text)?;
            Ok(ResolvedInsertTextLocation {
                location: range.location.clone(),
                preview_before: range.preview.clone(),
                preview_offset: preview_offset_for_index(
                    &range.preview,
                    range.start_index,
                    range.start_index,
                ),
            })
        }
        InsertTextSelector::AfterText(text) => {
            let range = resolve_text_anchor(document_map, text)?;
            Ok(ResolvedInsertTextLocation {
                location: crate::docs::map::DocumentLocation {
                    index: Some(range.end_index),
                    ..range.location.clone()
                },
                preview_before: range.preview.clone(),
                preview_offset: preview_offset_for_index(
                    &range.preview,
                    range.start_index,
                    range.end_index,
                ),
            })
        }
    }
}

fn resolved_for_index(
    document_map: &DocumentMap,
    index: i64,
) -> Result<ResolvedInsertTextLocation> {
    let entry = resolve_content_entry(document_map, &ContentSelector::Index(index))?;
    let preview_offset = entry
        .location
        .index
        .map(|start| preview_offset_for_index(&entry.preview, start, index))
        .unwrap_or(0);
    Ok(ResolvedInsertTextLocation {
        location: crate::docs::map::DocumentLocation {
            index: Some(index),
            ..entry.location.clone()
        },
        preview_before: entry.preview.clone(),
        preview_offset,
    })
}

fn resolved_for_entry_start(
    _document_map: &DocumentMap,
    entry: &DocumentMapEntry,
) -> Result<ResolvedInsertTextLocation> {
    let Some(index) = entry.location.index else {
        bail!("Document Map entry {} does not have a Google Docs index", entry.entry);
    };
    Ok(ResolvedInsertTextLocation {
        location: crate::docs::map::DocumentLocation {
            index: Some(index),
            ..entry.location.clone()
        },
        preview_before: entry.preview.clone(),
        preview_offset: 0,
    })
}

fn resolved_for_entry_end(
    document_map: &DocumentMap,
    entry: &DocumentMapEntry,
) -> Result<ResolvedInsertTextLocation> {
    let Some(start_index) = entry.location.index else {
        bail!("Document Map entry {} does not have a Google Docs index", entry.entry);
    };
    let end_index = document_map
        .text_blocks
        .iter()
        .find(|block| block.start_index == start_index)
        .map(|block| block.start_index + block.text.encode_utf16().count() as i64)
        .unwrap_or(start_index);
    Ok(ResolvedInsertTextLocation {
        location: crate::docs::map::DocumentLocation {
            index: Some(end_index),
            ..entry.location.clone()
        },
        preview_before: entry.preview.clone(),
        preview_offset: entry.preview.chars().count(),
    })
}

fn resolve_text_anchor(document_map: &DocumentMap, text: &str) -> Result<DocumentRange> {
    let matches = search_document_text(document_map, text);
    match matches.as_slice() {
        [range] => Ok(range.clone()),
        [] => bail!("text selector {text:?} did not match any Document Map entries"),
        candidates => {
            let candidate_list = candidates
                .iter()
                .enumerate()
                .map(|(index, range)| {
                    format!(
                        "match {} index {} page {} line {} preview {}",
                        index + 1,
                        range.start_index,
                        display_optional(range.location.page),
                        range.location.content_line,
                        range.preview
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            bail!("ambiguous text selector {text:?}; candidates: {candidate_list}")
        }
    }
}

fn resolve_replace_text_ranges(
    document_map: &DocumentMap,
    command: &ReplaceTextCommand,
) -> Result<Vec<DocumentRange>> {
    if command.old_text.is_empty() {
        bail!("replace-text old text must not be empty");
    }
    if command.all && command.match_number.is_some() {
        bail!("provide only one replace-text disambiguator: --match or --all");
    }
    if command.match_number == Some(0) {
        bail!("--match must be 1 or greater");
    }

    let matches = search_document_text(document_map, &command.old_text);
    if matches.is_empty() {
        bail!(
            "replace-text did not match {old_text:?}",
            old_text = command.old_text.as_str()
        );
    }
    if command.all {
        return Ok(matches);
    }
    if let Some(match_number) = command.match_number {
        return matches
            .get(match_number - 1)
            .cloned()
            .map(|range| vec![range])
            .with_context(|| {
                format!(
                    "replace-text match {match_number} was not found; {} matches available",
                    matches.len()
                )
            });
    }

    match matches.as_slice() {
        [range] => Ok(vec![range.clone()]),
        candidates => {
            let candidate_list = candidates
                .iter()
                .enumerate()
                .map(|(index, range)| {
                    format!(
                        "match {} index {} page {} line {} preview {}",
                        index + 1,
                        range.start_index,
                        display_optional(range.location.page),
                        range.location.content_line,
                        range.preview
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            bail!(
                "ambiguous replace-text match {old_text:?}; candidates: {candidate_list}",
                old_text = command.old_text.as_str()
            )
        }
    }
}

fn insert_text_request_body(
    index: Option<i64>,
    text: &str,
    required_revision_id: Option<&str>,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "requests": [
            {
                "insertText": {
                    "location": { "index": index },
                    "text": text
                }
            }
        ]
    });
    if let Some(required_revision_id) = required_revision_id {
        body["writeControl"] = serde_json::json!({
            "requiredRevisionId": required_revision_id
        });
    }
    body
}

fn replace_text_request_body(
    ranges: &[DocumentRange],
    new_text: &str,
    required_revision_id: Option<&str>,
) -> serde_json::Value {
    let mut requests = Vec::new();
    let mut ranges_descending = ranges.to_vec();
    ranges_descending.sort_by_key(|range| std::cmp::Reverse(range.start_index));

    for range in ranges_descending {
        requests.push(serde_json::json!({
            "deleteContentRange": {
                "range": {
                    "startIndex": range.start_index,
                    "endIndex": range.end_index
                }
            }
        }));
        requests.push(serde_json::json!({
            "insertText": {
                "location": { "index": range.start_index },
                "text": new_text
            }
        }));
    }

    let mut body = serde_json::json!({ "requests": requests });
    if let Some(required_revision_id) = required_revision_id {
        body["writeControl"] = serde_json::json!({
            "requiredRevisionId": required_revision_id
        });
    }
    body
}

fn insert_preview_text(before: &str, char_offset: usize, inserted_text: &str) -> String {
    let byte_offset = before
        .char_indices()
        .nth(char_offset)
        .map(|(index, _)| index)
        .unwrap_or(before.len());
    let mut after = before.to_string();
    after.insert_str(byte_offset, inserted_text);
    after
}

fn replace_text_preview(
    document_map: &DocumentMap,
    ranges: &[DocumentRange],
    old_text: &str,
    new_text: &str,
) -> ReplaceTextPreview {
    ReplaceTextPreview {
        changes: ranges
            .iter()
            .map(|range| ReplaceTextPreviewChange {
                range: range.clone(),
                before: range.preview.clone(),
                after: replace_text_preview_after(document_map, range, old_text, new_text),
            })
            .collect(),
    }
}

fn replace_text_preview_after(
    document_map: &DocumentMap,
    range: &DocumentRange,
    old_text: &str,
    new_text: &str,
) -> String {
    let block = document_map.text_blocks.iter().find(|block| {
        let block_end = block.start_index + block.text.encode_utf16().count() as i64;
        block.start_index <= range.start_index && range.end_index <= block_end
    });
    let Some(block) = block else {
        return range.preview.replacen(old_text, new_text, 1);
    };

    let start_offset = utf16_byte_offset(&block.text, range.start_index - block.start_index);
    let end_offset = utf16_byte_offset(&block.text, range.end_index - block.start_index);
    let mut after = block.text.clone();
    after.replace_range(start_offset..end_offset, new_text);
    compact_preview(&after)
}

fn utf16_byte_offset(text: &str, utf16_offset: i64) -> usize {
    if utf16_offset <= 0 {
        return 0;
    }

    let mut units = 0;
    for (byte_index, character) in text.char_indices() {
        if units >= utf16_offset {
            return byte_index;
        }
        units += character.len_utf16() as i64;
    }
    text.len()
}

fn compact_preview(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    const MAX_PREVIEW_CHARS: usize = 80;
    if compact.chars().count() <= MAX_PREVIEW_CHARS {
        compact
    } else {
        let mut truncated = compact.chars().take(MAX_PREVIEW_CHARS - 3).collect::<String>();
        truncated.push_str("...");
        truncated
    }
}

fn preview_offset_for_index(preview: &str, block_start_index: i64, insertion_index: i64) -> usize {
    let requested_offset = insertion_index.saturating_sub(block_start_index) as usize;
    requested_offset.min(preview.chars().count())
}

fn write_insert_text_preview(out: &mut impl Write, dry_run: &InsertTextDryRun) -> Result<()> {
    writeln!(
        out,
        "Insert text at index {}",
        display_optional(dry_run.location.index)
    )
    .context("failed to write Docs insert-text preview header")?;
    writeln!(out, "Before: {}", dry_run.preview.before)
        .context("failed to write Docs insert-text before preview")?;
    writeln!(out, "After: {}", dry_run.preview.after)
        .context("failed to write Docs insert-text after preview")?;
    Ok(())
}

fn write_replace_text_preview(out: &mut impl Write, dry_run: &ReplaceTextDryRun) -> Result<()> {
    writeln!(out, "Replace text in {} match(es)", dry_run.ranges.len())
        .context("failed to write Docs replace-text preview header")?;
    for (index, change) in dry_run.preview.changes.iter().enumerate() {
        writeln!(
            out,
            "Match {} at index {}",
            index + 1,
            change.range.start_index
        )
        .context("failed to write Docs replace-text match preview")?;
        writeln!(out, "Before: {}", change.before)
            .context("failed to write Docs replace-text before preview")?;
        writeln!(out, "After: {}", change.after)
            .context("failed to write Docs replace-text after preview")?;
    }
    Ok(())
}

fn get_document_options(
    document_id: String,
    fields: Option<String>,
    include_tabs_content: bool,
    documents_url: Option<&str>,
) -> GetDocumentOptions {
    let mut options =
        GetDocumentOptions::new(document_id).with_include_tabs_content(include_tabs_content);
    if let Some(fields) = fields {
        options = options.with_fields(fields);
    }
    if let Some(documents_url) = documents_url {
        options = options.with_documents_url(documents_url);
    }
    options
}

fn batch_update_document_options(
    document_id: String,
    request_body: serde_json::Value,
    documents_url: Option<&str>,
) -> BatchUpdateDocumentOptions {
    let mut options = BatchUpdateDocumentOptions::new(document_id, request_body);
    if let Some(documents_url) = documents_url {
        options = options.with_documents_url(documents_url);
    }
    options
}

fn write_document_map_table(out: &mut impl Write, document_map: &DocumentMap) -> Result<()> {
    writeln!(
        out,
        "{:<5} {:<7} {:<5} {:<4} {:<20} {:<18} {:<15} Preview",
        "Entry", "Index", "Page", "Line", "Kind", "Style", "Confidence"
    )
    .context("failed to write Docs Document Map header")?;

    for entry in &document_map.entries {
        let style = entry.style.as_deref().unwrap_or("-");
        writeln!(
            out,
            "{:<5} {:<7} {:<5} {:<4} {:<20} {:<18} {:<15} {}",
            entry.entry,
            display_optional(entry.location.index),
            display_optional(entry.location.page),
            entry.location.content_line,
            format!("{:?}", entry.kind),
            style,
            format!("{:?}", entry.location.confidence),
            entry.preview
        )
        .context("failed to write Docs Document Map row")?;
    }

    Ok(())
}

fn write_search_text_table(out: &mut impl Write, ranges: &[DocumentRange]) -> Result<()> {
    writeln!(
        out,
        "{:<5} {:<5} {:<4} {:<5} {:<15} Preview",
        "Match", "Page", "Line", "Index", "Confidence"
    )
    .context("failed to write Docs text search header")?;

    for (match_number, range) in ranges.iter().enumerate() {
        writeln!(
            out,
            "{:<5} {:<5} {:<4} {:<5} {:<15} {}",
            match_number + 1,
            display_optional(range.location.page),
            range.location.content_line,
            range.start_index,
            format!("{:?}", range.location.confidence),
            range.preview
        )
        .context("failed to write Docs text search row")?;
    }

    Ok(())
}

fn display_optional<T: ToString>(value: Option<T>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".into())
}

fn write_json_line<T: serde::Serialize>(
    out: &mut impl Write,
    value: &T,
    context: &str,
) -> Result<()> {
    serde_json::to_writer(&mut *out, value).context(context.to_string())?;
    writeln!(out).context("failed to write output")?;
    Ok(())
}
