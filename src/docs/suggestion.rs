use anyhow::{bail, Result};
use serde_json::Value;

pub(crate) const SUGGESTION_THREAD_FIELDS: &str = "suggestions(suggestionId,status,summaryText,headPost(author(displayName,me),createTime),replies(author(displayName,me),createTime,content,suggestionAction))";
const OPEN_STATUS: &str = "OPEN";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SuggestionAction {
    Accept,
    Reject,
    Delete,
}

impl SuggestionAction {
    pub(crate) fn command_name(self) -> &'static str {
        match self {
            SuggestionAction::Accept => "suggestion-accept",
            SuggestionAction::Reject => "suggestion-reject",
            SuggestionAction::Delete => "suggestion-delete",
        }
    }

    fn request_name(self) -> &'static str {
        match self {
            SuggestionAction::Accept => "acceptSuggestion",
            SuggestionAction::Reject => "rejectSuggestion",
            SuggestionAction::Delete => "deleteSuggestion",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SuggestionSelection {
    Ids(Vec<String>),
    AllOpen,
}

impl SuggestionSelection {
    pub(crate) fn from_flags(suggestion_ids: Vec<String>, all: bool) -> Result<Self> {
        match (suggestion_ids.is_empty(), all) {
            (false, false) => Ok(SuggestionSelection::Ids(suggestion_ids)),
            (true, true) => Ok(SuggestionSelection::AllOpen),
            _ => bail!("provide either --suggestion-id or --all"),
        }
    }
}

pub(crate) fn suggestion_threads(document: &Value, open_only: bool) -> Value {
    let threads = document_suggestion_threads(document)
        .filter(|thread| !open_only || is_open(thread))
        .cloned()
        .collect::<Vec<_>>();
    serde_json::json!({ "suggestions": threads })
}

pub(crate) fn select_suggestion_ids(
    document: &Value,
    selection: &SuggestionSelection,
) -> Result<Vec<String>> {
    match selection {
        SuggestionSelection::AllOpen => Ok(document_suggestion_threads(document)
            .filter(|thread| is_open(thread))
            .filter_map(suggestion_id)
            .map(str::to_string)
            .collect()),
        SuggestionSelection::Ids(requested) => {
            let known = document_suggestion_threads(document)
                .filter_map(suggestion_id)
                .collect::<Vec<_>>();
            let missing = requested
                .iter()
                .filter(|id| !known.contains(&id.as_str()))
                .map(String::as_str)
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                bail!(
                    "the Document has no suggestion with ID {}; list suggestion IDs with `goog docs suggestions`",
                    missing.join(", ")
                );
            }
            let mut ids = Vec::new();
            for id in requested {
                if !ids.contains(id) {
                    ids.push(id.clone());
                }
            }
            Ok(ids)
        }
    }
}

pub(crate) fn suggestion_action_request_body(
    action: SuggestionAction,
    suggestion_ids: &[String],
) -> Value {
    let requests = suggestion_ids
        .iter()
        .map(|id| serde_json::json!({ action.request_name(): { "suggestionId": id } }))
        .collect::<Vec<_>>();
    serde_json::json!({ "requests": requests })
}

fn document_suggestion_threads(document: &Value) -> impl Iterator<Item = &Value> {
    document
        .get("suggestions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn is_open(thread: &Value) -> bool {
    thread.get("status").and_then(Value::as_str) == Some(OPEN_STATUS)
}

fn suggestion_id(thread: &Value) -> Option<&str> {
    thread.get("suggestionId").and_then(Value::as_str)
}
