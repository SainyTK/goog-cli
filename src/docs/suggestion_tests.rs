use serde_json::json;

use super::suggestion::{
    select_suggestion_ids, suggestion_action_request_body, suggestion_threads, SuggestionAction,
    SuggestionSelection,
};

fn document_with_suggestions() -> serde_json::Value {
    json!({
        "suggestions": [
            { "suggestionId": "suggest.open-1", "status": "OPEN", "summaryText": "Add: \"very\"" },
            { "suggestionId": "suggest.accepted", "status": "ACCEPTED", "summaryText": "Delete: \"fox\"" },
            { "suggestionId": "suggest.open-2", "status": "OPEN", "summaryText": "Format: bold" }
        ]
    })
}

#[test]
fn suggestion_threads_keeps_every_thread_or_only_open_ones() {
    let document = document_with_suggestions();

    assert_eq!(
        suggestion_threads(&document, false)["suggestions"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let open = suggestion_threads(&document, true);
    let ids = open["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|thread| thread["suggestionId"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["suggest.open-1", "suggest.open-2"]);
}

#[test]
fn suggestion_threads_returns_an_empty_list_when_the_document_has_none() {
    assert_eq!(
        suggestion_threads(&json!({}), false),
        json!({ "suggestions": [] })
    );
}

#[test]
fn select_all_open_skips_accepted_and_rejected_threads() {
    let ids =
        select_suggestion_ids(&document_with_suggestions(), &SuggestionSelection::AllOpen).unwrap();

    assert_eq!(ids, ["suggest.open-1", "suggest.open-2"]);
}

#[test]
fn select_ids_keeps_order_and_drops_duplicates() {
    let selection = SuggestionSelection::Ids(vec![
        "suggest.open-2".into(),
        "suggest.accepted".into(),
        "suggest.open-2".into(),
    ]);

    let ids = select_suggestion_ids(&document_with_suggestions(), &selection).unwrap();

    assert_eq!(ids, ["suggest.open-2", "suggest.accepted"]);
}

#[test]
fn select_ids_rejects_unknown_suggestion_ids() {
    let selection = SuggestionSelection::Ids(vec!["suggest.open-1".into(), "suggest.gone".into()]);

    let err = select_suggestion_ids(&document_with_suggestions(), &selection).unwrap_err();

    assert!(
        err.to_string()
            .contains("no suggestion with ID suggest.gone"),
        "{err}"
    );
}

#[test]
fn selection_requires_exactly_one_of_ids_or_all() {
    assert!(SuggestionSelection::from_flags(Vec::new(), false).is_err());
    assert!(SuggestionSelection::from_flags(vec!["suggest.a".into()], true).is_err());
    assert_eq!(
        SuggestionSelection::from_flags(Vec::new(), true).unwrap(),
        SuggestionSelection::AllOpen
    );
}

#[test]
fn action_request_body_has_one_request_per_suggestion() {
    let ids = vec!["suggest.a".to_string(), "suggest.b".to_string()];

    assert_eq!(
        suggestion_action_request_body(SuggestionAction::Accept, &ids),
        json!({ "requests": [
            { "acceptSuggestion": { "suggestionId": "suggest.a" } },
            { "acceptSuggestion": { "suggestionId": "suggest.b" } }
        ] })
    );
    assert_eq!(
        suggestion_action_request_body(SuggestionAction::Reject, &ids[..1]),
        json!({ "requests": [{ "rejectSuggestion": { "suggestionId": "suggest.a" } }] })
    );
    assert_eq!(
        suggestion_action_request_body(SuggestionAction::Delete, &ids[1..]),
        json!({ "requests": [{ "deleteSuggestion": { "suggestionId": "suggest.b" } }] })
    );
}
