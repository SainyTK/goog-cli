use std::io::{self, Write};

use super::source::{read_deck_source, SlideColumnsDefinition};

#[test]
fn reads_the_responsible_ai_benchmark_from_yaml() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/slides/responsible-ai.yaml"
    );

    let source = read_deck_source(path, &mut io::empty()).unwrap();

    assert_eq!(source.schema_version, 1);
    assert_eq!(source.presentation.aspect_ratio.as_deref(), Some("wide"));
    assert_eq!(source.presentation.language.as_deref(), Some("en"));
    assert_eq!(source.presentation.speaker_notes.as_deref(), Some("omit"));
    assert_eq!(
        source
            .presentation
            .metadata
            .get("subject")
            .map(String::as_str),
        Some("Responsible AI measurement for banking virtual assistants")
    );
    assert_eq!(source.quality.minimum_font_size, Some(9.0));
    assert_eq!(source.quality.minimum_text_contrast, Some(4.5));
    assert_eq!(source.quality.required_alt_text, Some(true));
    assert_eq!(source.quality.allowed_overlap_groups, Vec::<String>::new());
    let safe_area = source.quality.safe_area.as_ref().unwrap();
    assert_eq!(safe_area.top, 24.0);
    assert_eq!(safe_area.right, 24.0);
    assert_eq!(safe_area.bottom, 24.0);
    assert_eq!(safe_area.left, 24.0);
    assert_eq!(
        source.theme.colors.get("accent").map(String::as_str),
        Some("#FF6B35")
    );
    let heading_font = &source.theme.fonts["heading"];
    assert_eq!(heading_font.family, "Arial");
    assert_eq!(heading_font.fallbacks, ["sans-serif"]);
    let title_style = &source.theme.type_styles["title"];
    assert_eq!(title_style.font, "heading");
    assert_eq!(title_style.size, 30.0);
    assert_eq!(title_style.weight.as_deref(), Some("bold"));
    assert_eq!(title_style.line_spacing, 1.05);
    assert_eq!(title_style.alignment, "start");
    assert_eq!(title_style.color, "ink");
    assert_eq!(source.theme.spacing["pageMargin"], 42.0);
    assert_eq!(source.theme.spacing["sectionGap"], 18.0);
    assert_eq!(source.theme.spacing["itemGap"], 12.0);
    assert_eq!(
        source.theme.fills.get("canvas").map(String::as_str),
        Some("canvas")
    );
    assert_eq!(
        source.theme.fills.get("panel").map(String::as_str),
        Some("panel")
    );
    assert_eq!(
        source.theme.fills.get("accent").map(String::as_str),
        Some("accent")
    );
    let panel_outline = &source.theme.outlines["panel"];
    assert_eq!(panel_outline.color, "rule");
    assert_eq!(panel_outline.width, 1.0);
    let rule_line = &source.theme.lines["rule"];
    assert_eq!(rule_line.color, "rule");
    assert_eq!(rule_line.width, 1.0);
    let geometry = source.theme.geometry.as_ref().unwrap();
    let safe_area = geometry.safe_area.as_ref().unwrap();
    assert_eq!(safe_area.top, 24.0);
    assert_eq!(safe_area.right, 24.0);
    assert_eq!(safe_area.bottom, 24.0);
    assert_eq!(safe_area.left, 24.0);
    let footer = geometry.footer.as_ref().unwrap();
    assert_eq!(footer.height, 18.0);
    assert_eq!(footer.gap, 12.0);
    let pattern_defaults = source.theme.pattern_defaults.as_ref().unwrap();
    let footer_defaults = pattern_defaults.footer.as_ref().unwrap();
    assert!(footer_defaults.show_slide_number);
    assert_eq!(footer_defaults.line, "rule");
    assert_eq!(source.slides.len(), 14);
    assert_eq!(source.slides[0].key, "cover");
    assert_eq!(
        source.slides[0].eyebrow.as_deref(),
        Some("BANKING VIRTUAL ASSISTANT")
    );
    assert_eq!(
        source.slides[0].title.as_deref(),
        Some("Responsible AI measurement for banking virtual assistants")
    );
    assert_eq!(
        source.slides[0].subtitle.as_deref(),
        Some("A practical framework for testing, release approval, and live monitoring")
    );
    assert_eq!(
        source.slides[0].footer.as_deref(),
        Some("Risk-tiered evidence | Independent challenge | Customer outcome monitoring")
    );
    assert!(source.slides[0].content.is_empty());
    assert_eq!(
        source.slides[1].statement.as_deref(),
        Some("A correct sentence can still create an unsafe banking outcome.")
    );
    assert_eq!(
        source.slides[1].body.as_deref(),
        Some(
            "Wrong information conflicts with approved fees, limits, rates, or product terms. Improper guidance can steer a customer toward unsuitable credit or suppress a complaint. Unauthorized actions change money, access, or obligations without valid confirmation. Failed recovery leaves the assistant unable to hand off, reverse, or contain a harmful interaction."
        )
    );
    assert_eq!(
        source.slides[1].takeaway.as_deref(),
        Some("Measure the conversation, the action, and the customer outcome.")
    );
    assert_eq!(
        source.slides[4].owner.as_deref(),
        Some("Product quality and Model Risk | Release and weekly review")
    );
    assert_eq!(source.slides[9].items.len(), 4);
    assert_eq!(
        source.slides[9].items[0].key.as_deref(),
        Some("critical-harms")
    );
    assert_eq!(source.slides[9].items[0].title, "Critical harms");
    assert_eq!(
        source.slides[9].items[0].body,
        "No unresolved critical safety, conduct, security, privacy, or action failure"
    );
    let SlideColumnsDefinition::Definitions(risk_columns) =
        source.slides[2].columns.as_ref().unwrap()
    else {
        panic!("expected structured comparison columns");
    };
    assert_eq!(risk_columns.len(), 3);
    assert_eq!(risk_columns[0].key, "information");
    assert_eq!(
        risk_columns[0].title.as_deref(),
        Some("Tier 1 | Information")
    );
    assert_eq!(
        risk_columns[0].summary.as_deref(),
        Some("Branch hours, navigation, and approved product facts")
    );
    assert_eq!(risk_columns[0].sections.len(), 2);
    assert_eq!(risk_columns[0].sections[0].label, "PRIMARY CONTROL");
    assert_eq!(
        risk_columns[0].sections[0].body,
        "Approved knowledge, citation checks, and safe uncertainty"
    );
    let SlideColumnsDefinition::Definitions(table_columns) =
        source.slides[3].columns.as_ref().unwrap()
    else {
        panic!("expected structured evidence-table columns");
    };
    assert_eq!(table_columns[0].label.as_deref(), Some("What to measure"));
    assert_eq!(table_columns[0].width, Some(0.23));
    assert_eq!(source.slides[3].rows.len(), 7);
    assert_eq!(source.slides[3].rows[0].key, "valid-reliable");
    assert_eq!(
        source.slides[3].rows[0].cells["dimension"],
        "Valid and reliable"
    );
    assert_eq!(
        source.slides[3].rows[0].cells["method"],
        "Risk-tiered golden-set replay and dual review"
    );
    assert_eq!(
        source.slides[3].rows[0].cells["evidence"],
        "Task success; unsupported claims | weekly"
    );
    assert_eq!(source.slides[6].rows.len(), 5);
    assert_eq!(source.slides[4].stages.len(), 4);
    assert_eq!(source.slides[4].stages[0].key, "stratify");
    assert_eq!(source.slides[4].stages[0].title, "Stratify");
    assert_eq!(
        source.slides[4].stages[0].body.as_deref(),
        Some("Cover journey, risk tier, language, channel, customer state, and edge cases.")
    );
    assert_eq!(source.slides[8].stages.len(), 4);
    assert_eq!(source.slides[8].stages[0].key, "disclose-ai");
    assert_eq!(source.slides[8].stages[0].title, "Disclose AI");
    assert_eq!(
        source.slides[8].stages[0].test.as_deref(),
        Some("Customer recognizes the assistant and its limits")
    );
    assert_eq!(
        source.slides[8].stages[0].measure.as_deref(),
        Some("Disclosure comprehension")
    );
    let evidence = source.slides[4].evidence.as_ref().unwrap();
    assert_eq!(evidence.title, "Score four layers, then inspect failures");
    assert_eq!(evidence.items.len(), 4);
    assert_eq!(evidence.items[0].key.as_deref(), Some("automated"));
    assert_eq!(evidence.items[0].title, "Automated");
    assert_eq!(
        evidence.items[0].body,
        "Retrieval precision, groundedness, and policy rule checks"
    );
    assert_eq!(source.slides[5].groups.len(), 2);
    assert_eq!(source.slides[5].groups[0].key, "unacceptable-outcomes");
    assert_eq!(source.slides[5].groups[0].title, "Unacceptable outcomes");
    assert_eq!(source.slides[5].groups[0].cards.len(), 4);
    assert_eq!(source.slides[5].groups[0].cards[0].key, "customer-harm");
    assert_eq!(source.slides[5].groups[0].cards[0].title, "Customer harm");
    assert_eq!(
        source.slides[5].groups[0].cards[0].body,
        "Misleading fees, unsuitable guidance, and denied complaint access"
    );
    assert_eq!(source.slides[7].steps.len(), 4);
    assert_eq!(source.slides[7].steps[0].key, "sample");
    assert_eq!(source.slides[7].steps[0].title, "Sample");
    assert_eq!(
        source.slides[7].steps[0].body,
        "Stratify by journey, risk tier, language, dialect, accessibility, and lawful cohort."
    );
    assert_eq!(source.slides[10].signals.len(), 4);
    assert_eq!(source.slides[10].signals[0].key, "customer-outcome");
    assert_eq!(source.slides[10].signals[0].title, "Customer outcome");
    assert_eq!(
        source.slides[10].signals[0].items,
        [
            "Task completion",
            "Repeat contact",
            "Complaint and detriment"
        ]
    );
    assert_eq!(source.slides[10].milestones.len(), 5);
    assert_eq!(source.slides[10].milestones[0].key, "detect");
    assert_eq!(source.slides[10].milestones[0].title, "Detect");
    assert_eq!(source.slides[10].milestones[0].body, None);
    assert_eq!(source.slides[10].milestones[0].exit, None);
    assert_eq!(source.slides[11].milestones.len(), 3);
    assert_eq!(source.slides[11].milestones[0].key, "baseline");
    assert_eq!(
        source.slides[11].milestones[0].title,
        "Baseline one journey"
    );
    assert_eq!(
        source.slides[11].milestones[0].body.as_deref(),
        Some(
            "Choose a bounded customer journey. Define risk tier, approved sources, harm taxonomy, owners, and golden set."
        )
    );
    assert_eq!(
        source.slides[11].milestones[0].exit.as_deref(),
        Some("Independent validation confirms the evidence is complete.")
    );
    assert_eq!(
        source.slides[5].columns,
        Some(SlideColumnsDefinition::Count(2))
    );
    assert_eq!(
        source.slides[12].columns,
        Some(SlideColumnsDefinition::Count(2))
    );
    assert_eq!(source.slides[12].sources.len(), 8);
    assert_eq!(source.slides[12].sources[0].key, "nist-ai-rmf");
    assert_eq!(
        source.slides[12].sources[0].title,
        "NIST AI RMF 1.0 and Core"
    );
    assert_eq!(
        source.slides[12].sources[0].note,
        "Trustworthiness characteristics and the Govern, Map, Measure, and Manage lifecycle."
    );
    assert_eq!(source.slides[13].questions.len(), 4);
    assert_eq!(source.slides[13].questions[0].key, "capability");
    assert_eq!(source.slides[13].questions[0].title, "What can it do?");
    assert_eq!(
        source.slides[13].questions[0].body,
        "Scope, risk tier, permissions, and prohibited outcomes"
    );
    assert!(!source.slides[1].content.contains_key("statement"));
    assert!(!source.slides[1].content.contains_key("body"));
    assert!(!source.slides[1].content.contains_key("takeaway"));
    assert!(!source.slides[4].content.contains_key("owner"));
    assert!(!source.slides[9].content.contains_key("items"));
    for slide in &source.slides {
        assert!(!slide.content.contains_key("columns"));
        assert!(!slide.content.contains_key("rows"));
        assert!(!slide.content.contains_key("stages"));
        assert!(!slide.content.contains_key("evidence"));
        assert!(!slide.content.contains_key("groups"));
        assert!(!slide.content.contains_key("steps"));
        assert!(!slide.content.contains_key("signals"));
        assert!(!slide.content.contains_key("milestones"));
        assert!(!slide.content.contains_key("sources"));
        assert!(!slide.content.contains_key("questions"));
    }
    assert_eq!(source.slides[13].key, "operating-principle");
}

#[test]
fn reads_typed_slide_sources_from_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: source-basis
    pattern: sources
    sources:
      - key: framework
        title: Framework title
        note: Framework note
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "source-basis",
                "pattern": "sources",
                "sources": [{
                    "key": "framework",
                    "title": "Framework title",
                    "note": "Framework note"
                }]
            }]
        }"#,
    ];

    for source in sources {
        let source = read_deck_source("-", &mut io::Cursor::new(source)).unwrap();
        let entry = &source.slides[0].sources[0];

        assert_eq!(entry.key, "framework");
        assert_eq!(entry.title, "Framework title");
        assert_eq!(entry.note, "Framework note");
        assert!(!source.slides[0].content.contains_key("sources"));
    }
}

#[test]
fn reads_closing_questions_from_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: operating-principle
    pattern: closing
    questions:
      - key: capability
        title: What can it do?
        body: Scope, risk tier, permissions, and prohibited outcomes
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "operating-principle",
                "pattern": "closing",
                "questions": [{
                    "key": "capability",
                    "title": "What can it do?",
                    "body": "Scope, risk tier, permissions, and prohibited outcomes"
                }]
            }]
        }"#,
    ];

    for source in sources {
        let source = read_deck_source("-", &mut io::Cursor::new(source)).unwrap();
        let question = &source.slides[0].questions[0];

        assert_eq!(question.key, "capability");
        assert_eq!(question.title, "What can it do?");
        assert_eq!(
            question.body,
            "Scope, risk tier, permissions, and prohibited outcomes"
        );
        assert!(!source.slides[0].content.contains_key("questions"));
    }
}

#[test]
fn rejects_malformed_closing_questions_with_exact_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: operating-principle
    pattern: closing
    questions:
      - key: 42
        title: What can it do?
        body: Scope and permissions
"#,
            "slides[0].questions[0].key",
        ),
        (
            r#"{
                "schemaVersion": 1,
                "presentation": {},
                "theme": {},
                "quality": {},
                "slides": [{
                    "key": "operating-principle",
                    "pattern": "closing",
                    "questions": [{
                        "key": "capability",
                        "title": ["What can it do?"],
                        "body": "Scope and permissions"
                    }]
                }]
            }"#,
            "slides[0].questions[0].title",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: operating-principle
    pattern: closing
    questions:
      - key: capability
        title: What can it do?
        body: 42
"#,
            "slides[0].questions[0].body",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_unknown_closing_question_fields() {
    let source = r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: operating-principle
    pattern: closing
    questions:
      - key: capability
        title: What can it do?
        body: Scope and permissions
        answer: Named capability inventory
"#;

    let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("slides[0].questions[0].answer"),
        "{message}"
    );
    assert!(message.contains("unknown field"), "{message}");
}

#[test]
fn rejects_malformed_slide_sources_with_exact_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: source-basis
    pattern: sources
    sources:
      - key: 42
        title: Framework title
        note: Framework note
"#,
            "slides[0].sources[0].key",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "source-basis",
                "pattern": "sources",
                "sources": [{
                    "key": "framework",
                    "title": ["Framework title"],
                    "note": "Framework note"
                }]
            }]
        }"#,
            "slides[0].sources[0].title",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: source-basis
    pattern: sources
    sources:
      - key: framework
        title: Framework title
        note: 42
"#,
            "slides[0].sources[0].note",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_unknown_slide_source_fields() {
    let source = r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: source-basis
    pattern: sources
    sources:
      - key: framework
        title: Framework title
        note: Framework note
        url: https://example.test/framework
"#;

    let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("slides[0].sources[0].url"), "{message}");
    assert!(message.contains("unknown field"), "{message}");
}

#[test]
fn rejects_unsupported_schema_versions() {
    let mut source = io::Cursor::new(
        r#"{
            "schemaVersion": 2,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [],
            "futureField": {}
        }"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();

    assert_eq!(
        error.to_string(),
        "unsupported Deck Source schemaVersion 2 in stdin; supported version: 1"
    );
}

#[test]
fn rejects_duplicate_slide_keys_with_both_source_paths() {
    let mut source = io::Cursor::new(
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: repeated
    pattern: cover
  - key: repeated
    pattern: closing
"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();

    assert_eq!(
        error.to_string(),
        "duplicate slide key 'repeated' at slides[1].key in stdin; first declared at slides[0].key"
    );
}

#[test]
fn rejects_non_string_slide_keys_in_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: 42
    pattern: cover
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{"key": 42, "pattern": "cover"}]
        }"#,
    ];

    for source in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("slides[0].key"), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_non_string_slide_patterns_in_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: cover
    pattern: 42
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{"key": "cover", "pattern": ["cover"]}]
        }"#,
    ];

    for source in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("slides[0].pattern"), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_non_string_slide_frame_text_fields_in_yaml_and_json() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: cover
    pattern: cover
    title: 42
"#,
            "slides[0].title",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "cover",
                "pattern": "cover",
                "title": "Hello",
                "footer": ["Internal"]
            }]
        }"#,
            "slides[0].footer",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: cover
    pattern: cover
    eyebrow: false
"#,
            "slides[0].eyebrow",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "cover",
                "pattern": "cover",
                "subtitle": {"text": "Hello"}
            }]
        }"#,
            "slides[0].subtitle",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_non_string_slide_statements_in_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: why-measurement-changes
    pattern: statement
    statement: 42
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "why-measurement-changes",
                "pattern": "statement",
                "statement": ["Measure the outcome"]
            }]
        }"#,
    ];

    for source in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("slides[0].statement"), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_non_string_slide_bodies_in_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: why-measurement-changes
    pattern: statement
    body: 42
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "why-measurement-changes",
                "pattern": "statement",
                "body": ["Wrong information"]
            }]
        }"#,
    ];

    for source in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("slides[0].body"), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_non_string_slide_takeaways_in_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: why-measurement-changes
    pattern: statement
    takeaway: 42
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "why-measurement-changes",
                "pattern": "statement",
                "takeaway": ["Measure the outcome"]
            }]
        }"#,
    ];

    for source in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("slides[0].takeaway"), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_non_string_slide_owners_in_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: valid-and-reliable
    pattern: process
    owner: 42
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "valid-and-reliable",
                "pattern": "process",
                "owner": ["Product quality"]
            }]
        }"#,
    ];

    for source in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("slides[0].owner"), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn reads_slide_emphasis_from_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: safe-and-fair-conduct
    pattern: cards
    emphasis: high
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "safe-and-fair-conduct",
                "pattern": "cards",
                "emphasis": "high"
            }]
        }"#,
    ];

    for source in sources {
        let source = read_deck_source("-", &mut io::Cursor::new(source)).unwrap();

        assert_eq!(source.slides[0].emphasis.as_deref(), Some("high"));
        assert!(!source.slides[0].content.contains_key("emphasis"));
    }
}

#[test]
fn rejects_non_string_slide_emphasis_in_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: safe-and-fair-conduct
    pattern: cards
    emphasis: 42
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "safe-and-fair-conduct",
                "pattern": "cards",
                "emphasis": ["high"]
            }]
        }"#,
    ];

    for source in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("slides[0].emphasis"), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn reads_slide_density_from_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: safe-and-fair-conduct
    pattern: cards
    density: compact
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "safe-and-fair-conduct",
                "pattern": "cards",
                "density": "compact"
            }]
        }"#,
    ];

    for source in sources {
        let source = read_deck_source("-", &mut io::Cursor::new(source)).unwrap();

        assert_eq!(source.slides[0].density.as_deref(), Some("compact"));
        assert!(!source.slides[0].content.contains_key("density"));
    }
}

#[test]
fn rejects_non_string_slide_density_in_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: safe-and-fair-conduct
    pattern: cards
    density: 42
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "safe-and-fair-conduct",
                "pattern": "cards",
                "density": ["compact"]
            }]
        }"#,
    ];

    for source in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("slides[0].density"), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_malformed_slide_items_in_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: release-governance
    pattern: statement-and-list
    items:
      - key: 42
        title: Critical harms
        body: No unresolved critical failures
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "release-governance",
                "pattern": "statement-and-list",
                "items": [{
                    "key": "critical-harms",
                    "title": "Critical harms",
                    "body": ["No unresolved critical failures"]
                }]
            }]
        }"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "release-governance",
                "pattern": "statement-and-list",
                "items": [{
                    "key": "critical-harms",
                    "title": ["Critical harms"],
                    "body": "No unresolved critical failures"
                }]
            }]
        }"#,
    ];
    let expected_paths = [
        "slides[0].items[0].key",
        "slides[0].items[0].body",
        "slides[0].items[0].title",
    ];

    for (source, expected_path) in sources.into_iter().zip(expected_paths) {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_unknown_slide_item_fields() {
    let source = r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: release-governance
    pattern: statement-and-list
    items:
      - title: Critical harms
        body: No unresolved critical failures
        emphasis: high
"#;

    let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("slides[0].items[0].emphasis"), "{message}");
    assert!(message.contains("unknown field"), "{message}");
}

#[test]
fn rejects_invalid_slide_columns() {
    let source = r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: safe-and-fair-conduct
    pattern: cards
    columns: true
"#;

    let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("slides[0].columns"), "{message}");
    assert!(message.contains("invalid type"), "{message}");
}

#[test]
fn rejects_malformed_structured_slide_columns() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: risk-tiering
    pattern: comparison
    columns:
      - key: 42
        title: Tier 1
"#,
            "slides[0].columns[0].key",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "measurement-framework",
                "pattern": "evidence-table",
                "columns": [{
                    "key": "dimension",
                    "label": "What to measure",
                    "width": "wide"
                }]
            }]
        }"#,
            "slides[0].columns[0].width",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: risk-tiering
    pattern: comparison
    columns:
      - key: information
        sections:
          - label: PRIMARY CONTROL
            body: 42
"#,
            "slides[0].columns[0].sections[0].body",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_unknown_structured_slide_column_fields() {
    let source = r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: risk-tiering
    pattern: comparison
    columns:
      - key: information
        heading: Tier 1
"#;

    let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("slides[0].columns[0].heading"),
        "{message}"
    );
    assert!(message.contains("unknown field"), "{message}");
}

#[test]
fn reads_structured_slide_columns_from_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: risk-tiering
    pattern: comparison
    columns:
      - key: information
        title: Tier 1
        summary: Approved product facts
        sections:
          - label: PRIMARY CONTROL
            body: Approved knowledge
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "risk-tiering",
                "pattern": "comparison",
                "columns": [{
                    "key": "information",
                    "title": "Tier 1",
                    "summary": "Approved product facts",
                    "sections": [{
                        "label": "PRIMARY CONTROL",
                        "body": "Approved knowledge"
                    }]
                }]
            }]
        }"#,
    ];

    for source in sources {
        let source = read_deck_source("-", &mut io::Cursor::new(source)).unwrap();
        let SlideColumnsDefinition::Definitions(columns) =
            source.slides[0].columns.as_ref().unwrap()
        else {
            panic!("expected structured comparison columns");
        };

        assert_eq!(columns[0].key, "information");
        assert_eq!(columns[0].title.as_deref(), Some("Tier 1"));
        assert_eq!(
            columns[0].summary.as_deref(),
            Some("Approved product facts")
        );
        assert_eq!(columns[0].sections[0].label, "PRIMARY CONTROL");
        assert_eq!(columns[0].sections[0].body, "Approved knowledge");
    }
}

#[test]
fn reads_evidence_table_rows_from_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: measurement-framework
    pattern: evidence-table
    rows:
      - key: safe
        dimension: Safe
        method: Red-team harmful actions
        evidence: Severe breach | live
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "measurement-framework",
                "pattern": "evidence-table",
                "rows": [{
                    "key": "safe",
                    "dimension": "Safe",
                    "method": "Red-team harmful actions",
                    "evidence": "Severe breach | live"
                }]
            }]
        }"#,
    ];

    for source in sources {
        let source = read_deck_source("-", &mut io::Cursor::new(source)).unwrap();
        let row = &source.slides[0].rows[0];

        assert_eq!(row.key, "safe");
        assert_eq!(row.cells["dimension"], "Safe");
        assert_eq!(row.cells["method"], "Red-team harmful actions");
        assert_eq!(row.cells["evidence"], "Severe breach | live");
        assert!(!source.slides[0].content.contains_key("rows"));
    }
}

#[test]
fn rejects_malformed_evidence_table_rows_with_exact_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: measurement-framework
    pattern: evidence-table
    rows:
      - key: 42
        dimension: Safe
"#,
            "slides[0].rows[0].key",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: measurement-framework
    pattern: evidence-table
    rows:
      - key: safe
        dimension: 42
"#,
            "slides[0].rows[0].dimension",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "measurement-framework",
                "pattern": "evidence-table",
                "rows": [{
                    "key": "safe",
                    "dimension": ["Safe"]
                }]
            }]
        }"#,
            "slides[0].rows[0].dimension",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_malformed_process_stages_with_exact_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: valid-and-reliable
    pattern: process
    stages:
      - key: 42
        title: Stratify
"#,
            "slides[0].stages[0].key",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "valid-and-reliable",
                "pattern": "process",
                "stages": [{"key": "stratify", "title": ["Stratify"]}]
            }]
        }"#,
            "slides[0].stages[0].title",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: valid-and-reliable
    pattern: process
    stages:
      - key: stratify
        title: Stratify
        body: 42
"#,
            "slides[0].stages[0].body",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: human-control
    pattern: process
    stages:
      - key: disclose-ai
        title: Disclose AI
        test: 42
"#,
            "slides[0].stages[0].test",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "human-control",
                "pattern": "process",
                "stages": [{
                    "key": "disclose-ai",
                    "title": "Disclose AI",
                    "measure": {"name": "Disclosure comprehension"}
                }]
            }]
        }"#,
            "slides[0].stages[0].measure",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn reads_process_stages_from_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: human-control
    pattern: process
    stages:
      - key: disclose-ai
        title: Disclose AI
        body: Explain the assistant's limits.
        test: Customer recognizes the assistant and its limits
        measure: Disclosure comprehension
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "human-control",
                "pattern": "process",
                "stages": [{
                    "key": "disclose-ai",
                    "title": "Disclose AI",
                    "body": "Explain the assistant's limits.",
                    "test": "Customer recognizes the assistant and its limits",
                    "measure": "Disclosure comprehension"
                }]
            }]
        }"#,
    ];

    for source in sources {
        let source = read_deck_source("-", &mut io::Cursor::new(source)).unwrap();
        let stage = &source.slides[0].stages[0];

        assert_eq!(stage.key, "disclose-ai");
        assert_eq!(stage.title, "Disclose AI");
        assert_eq!(
            stage.body.as_deref(),
            Some("Explain the assistant's limits.")
        );
        assert_eq!(
            stage.test.as_deref(),
            Some("Customer recognizes the assistant and its limits")
        );
        assert_eq!(stage.measure.as_deref(), Some("Disclosure comprehension"));
        assert!(!source.slides[0].content.contains_key("stages"));
    }
}

#[test]
fn rejects_unknown_process_stage_fields() {
    let source = r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: valid-and-reliable
    pattern: process
    stages:
      - key: stratify
        title: Stratify
        description: Cover the representative cases.
"#;

    let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("slides[0].stages[0].description"),
        "{message}"
    );
    assert!(message.contains("unknown field"), "{message}");
}

#[test]
fn reads_slide_evidence_from_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: valid-and-reliable
    pattern: process
    evidence:
      title: Score four layers, then inspect failures
      items:
        - key: automated
          title: Automated
          body: Retrieval precision, groundedness, and policy rule checks
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "valid-and-reliable",
                "pattern": "process",
                "evidence": {
                    "title": "Score four layers, then inspect failures",
                    "items": [{
                        "key": "automated",
                        "title": "Automated",
                        "body": "Retrieval precision, groundedness, and policy rule checks"
                    }]
                }
            }]
        }"#,
    ];

    for source in sources {
        let source = read_deck_source("-", &mut io::Cursor::new(source)).unwrap();
        let evidence = source.slides[0].evidence.as_ref().unwrap();

        assert_eq!(evidence.title, "Score four layers, then inspect failures");
        assert_eq!(evidence.items.len(), 1);
        assert_eq!(evidence.items[0].key.as_deref(), Some("automated"));
        assert_eq!(evidence.items[0].title, "Automated");
        assert_eq!(
            evidence.items[0].body,
            "Retrieval precision, groundedness, and policy rule checks"
        );
        assert!(!source.slides[0].content.contains_key("evidence"));
    }
}

#[test]
fn rejects_malformed_slide_evidence_with_exact_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: valid-and-reliable
    pattern: process
    evidence:
      title: 42
      items: []
"#,
            "slides[0].evidence.title",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "valid-and-reliable",
                "pattern": "process",
                "evidence": {
                    "title": "Score four layers",
                    "items": [{
                        "key": "automated",
                        "title": "Automated",
                        "body": ["Retrieval precision"]
                    }]
                }
            }]
        }"#,
            "slides[0].evidence.items[0].body",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_unknown_slide_evidence_fields() {
    let source = r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: valid-and-reliable
    pattern: process
    evidence:
      title: Score four layers
      items: []
      description: Inspect failures by severity.
"#;

    let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("slides[0].evidence.description"),
        "{message}"
    );
    assert!(message.contains("unknown field"), "{message}");
}

#[test]
fn rejects_malformed_card_groups_with_exact_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: safe-and-fair-conduct
    pattern: cards
    groups:
      - key: 42
        title: Unacceptable outcomes
        cards: []
"#,
            "slides[0].groups[0].key",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "safe-and-fair-conduct",
                "pattern": "cards",
                "groups": [{
                    "key": "unacceptable-outcomes",
                    "title": ["Unacceptable outcomes"],
                    "cards": []
                }]
            }]
        }"#,
            "slides[0].groups[0].title",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: safe-and-fair-conduct
    pattern: cards
    groups:
      - key: unacceptable-outcomes
        title: Unacceptable outcomes
        cards:
          - key: 42
            title: Customer harm
            body: Misleading fees
"#,
            "slides[0].groups[0].cards[0].key",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "safe-and-fair-conduct",
                "pattern": "cards",
                "groups": [{
                    "key": "unacceptable-outcomes",
                    "title": "Unacceptable outcomes",
                    "cards": [{
                        "key": "customer-harm",
                        "title": {"text": "Customer harm"},
                        "body": "Misleading fees"
                    }]
                }]
            }]
        }"#,
            "slides[0].groups[0].cards[0].title",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: safe-and-fair-conduct
    pattern: cards
    groups:
      - key: unacceptable-outcomes
        title: Unacceptable outcomes
        cards:
          - key: customer-harm
            title: Customer harm
            body: 42
"#,
            "slides[0].groups[0].cards[0].body",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn reads_card_groups_from_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: safe-and-fair-conduct
    pattern: cards
    groups:
      - key: unacceptable-outcomes
        title: Unacceptable outcomes
        cards:
          - key: customer-harm
            title: Customer harm
            body: Misleading fees and unsuitable guidance
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "safe-and-fair-conduct",
                "pattern": "cards",
                "groups": [{
                    "key": "unacceptable-outcomes",
                    "title": "Unacceptable outcomes",
                    "cards": [{
                        "key": "customer-harm",
                        "title": "Customer harm",
                        "body": "Misleading fees and unsuitable guidance"
                    }]
                }]
            }]
        }"#,
    ];

    for source in sources {
        let source = read_deck_source("-", &mut io::Cursor::new(source)).unwrap();
        let group = &source.slides[0].groups[0];

        assert_eq!(group.key, "unacceptable-outcomes");
        assert_eq!(group.title, "Unacceptable outcomes");
        assert_eq!(group.cards.len(), 1);
        assert_eq!(group.cards[0].key, "customer-harm");
        assert_eq!(group.cards[0].title, "Customer harm");
        assert_eq!(
            group.cards[0].body,
            "Misleading fees and unsuitable guidance"
        );
        assert!(!source.slides[0].content.contains_key("groups"));
    }
}

#[test]
fn rejects_unknown_card_group_fields() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: safe-and-fair-conduct
    pattern: cards
    groups:
      - key: unacceptable-outcomes
        title: Unacceptable outcomes
        description: Severe customer harms
        cards: []
"#,
            "slides[0].groups[0].description",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "safe-and-fair-conduct",
                "pattern": "cards",
                "groups": [{
                    "key": "unacceptable-outcomes",
                    "title": "Unacceptable outcomes",
                    "cards": [{
                        "key": "customer-harm",
                        "title": "Customer harm",
                        "body": "Misleading fees",
                        "severity": "critical"
                    }]
                }]
            }]
        }"#,
            "slides[0].groups[0].cards[0].severity",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("unknown field"), "{message}");
    }
}

#[test]
fn reads_steps_from_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: fairness
    pattern: comparison
    steps:
      - key: sample
        title: Sample
        body: Stratify by journey and risk tier.
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "fairness",
                "pattern": "comparison",
                "steps": [{
                    "key": "sample",
                    "title": "Sample",
                    "body": "Stratify by journey and risk tier."
                }]
            }]
        }"#,
    ];

    for source in sources {
        let source = read_deck_source("-", &mut io::Cursor::new(source)).unwrap();
        let step = &source.slides[0].steps[0];

        assert_eq!(step.key, "sample");
        assert_eq!(step.title, "Sample");
        assert_eq!(step.body, "Stratify by journey and risk tier.");
        assert!(!source.slides[0].content.contains_key("steps"));
    }
}

#[test]
fn rejects_malformed_steps_with_exact_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: fairness
    pattern: comparison
    steps:
      - key: 42
        title: Sample
        body: Stratify by journey and risk tier.
"#,
            "slides[0].steps[0].key",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "fairness",
                "pattern": "comparison",
                "steps": [{
                    "key": "sample",
                    "title": ["Sample"],
                    "body": "Stratify by journey and risk tier."
                }]
            }]
        }"#,
            "slides[0].steps[0].title",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: fairness
    pattern: comparison
    steps:
      - key: sample
        title: Sample
        body: 42
"#,
            "slides[0].steps[0].body",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_unknown_step_fields() {
    let source = r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: fairness
    pattern: comparison
    steps:
      - key: sample
        title: Sample
        body: Stratify by journey and risk tier.
        owner: Responsible AI
"#;

    let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("slides[0].steps[0].owner"), "{message}");
    assert!(message.contains("unknown field"), "{message}");
}

#[test]
fn reads_timeline_signals_from_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: ongoing-monitoring
    pattern: timeline
    signals:
      - key: customer-outcome
        title: Customer outcome
        items:
          - Task completion
          - Repeat contact
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "ongoing-monitoring",
                "pattern": "timeline",
                "signals": [{
                    "key": "customer-outcome",
                    "title": "Customer outcome",
                    "items": ["Task completion", "Repeat contact"]
                }]
            }]
        }"#,
    ];

    for source in sources {
        let source = read_deck_source("-", &mut io::Cursor::new(source)).unwrap();
        let signal = &source.slides[0].signals[0];

        assert_eq!(signal.key, "customer-outcome");
        assert_eq!(signal.title, "Customer outcome");
        assert_eq!(signal.items, ["Task completion", "Repeat contact"]);
        assert!(!source.slides[0].content.contains_key("signals"));
    }
}

#[test]
fn rejects_malformed_timeline_signals_with_exact_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: ongoing-monitoring
    pattern: timeline
    signals:
      - key: 42
        title: Customer outcome
        items: []
"#,
            "slides[0].signals[0].key",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "ongoing-monitoring",
                "pattern": "timeline",
                "signals": [{
                    "key": "customer-outcome",
                    "title": ["Customer outcome"],
                    "items": []
                }]
            }]
        }"#,
            "slides[0].signals[0].title",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: ongoing-monitoring
    pattern: timeline
    signals:
      - key: customer-outcome
        title: Customer outcome
        items:
          - 42
"#,
            "slides[0].signals[0].items[0]",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_unknown_timeline_signal_fields() {
    let source = r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: ongoing-monitoring
    pattern: timeline
    signals:
      - key: customer-outcome
        title: Customer outcome
        items: []
        cadence: weekly
"#;

    let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("slides[0].signals[0].cadence"),
        "{message}"
    );
    assert!(message.contains("unknown field"), "{message}");
}

#[test]
fn reads_timeline_milestones_from_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: implementation-path
    pattern: timeline
    milestones:
      - key: baseline
        title: Baseline one journey
        body: Choose a bounded customer journey.
        exit: Independent validation confirms the evidence is complete.
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "implementation-path",
                "pattern": "timeline",
                "milestones": [{
                    "key": "baseline",
                    "title": "Baseline one journey",
                    "body": "Choose a bounded customer journey.",
                    "exit": "Independent validation confirms the evidence is complete."
                }]
            }]
        }"#,
    ];

    for source in sources {
        let source = read_deck_source("-", &mut io::Cursor::new(source)).unwrap();
        let milestone = &source.slides[0].milestones[0];

        assert_eq!(milestone.key, "baseline");
        assert_eq!(milestone.title, "Baseline one journey");
        assert_eq!(
            milestone.body.as_deref(),
            Some("Choose a bounded customer journey.")
        );
        assert_eq!(
            milestone.exit.as_deref(),
            Some("Independent validation confirms the evidence is complete.")
        );
        assert!(!source.slides[0].content.contains_key("milestones"));
    }
}

#[test]
fn rejects_malformed_timeline_milestones_with_exact_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: ongoing-monitoring
    pattern: timeline
    milestones:
      - key: 42
        title: Detect
"#,
            "slides[0].milestones[0].key",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "ongoing-monitoring",
                "pattern": "timeline",
                "milestones": [{"key": "detect", "title": ["Detect"]}]
            }]
        }"#,
            "slides[0].milestones[0].title",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: implementation-path
    pattern: timeline
    milestones:
      - key: baseline
        title: Baseline one journey
        body: 42
"#,
            "slides[0].milestones[0].body",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [{
                "key": "implementation-path",
                "pattern": "timeline",
                "milestones": [{
                    "key": "baseline",
                    "title": "Baseline one journey",
                    "exit": ["Evidence is complete"]
                }]
            }]
        }"#,
            "slides[0].milestones[0].exit",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_unknown_timeline_milestone_fields() {
    let source = r#"
schemaVersion: 1
presentation: {}
theme: {}
quality: {}
slides:
  - key: ongoing-monitoring
    pattern: timeline
    milestones:
      - key: detect
        title: Detect
        cadence: continuous
"#;

    let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("slides[0].milestones[0].cadence"),
        "{message}"
    );
    assert!(message.contains("unknown field"), "{message}");
}

#[test]
fn reads_json_deck_sources_from_json_paths() {
    let mut file = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
    write!(
        file,
        r#"{{
            "schemaVersion": 1,
            "presentation": {{"aspectRatio": "wide"}},
            "theme": {{}},
            "quality": {{}},
            "slides": [{{"key": "cover", "pattern": "cover", "title": "Hello"}}]
        }}"#
    )
    .unwrap();

    let source = read_deck_source(file.path().to_str().unwrap(), &mut io::empty()).unwrap();

    assert_eq!(source.slides[0].key, "cover");
    assert_eq!(source.slides[0].title.as_deref(), Some("Hello"));
}

#[test]
fn yaml_and_json_sources_have_semantic_parity() {
    let mut yaml = tempfile::Builder::new().suffix(".yaml").tempfile().unwrap();
    write!(
        yaml,
        "schemaVersion: 1\npresentation: {{}}\ntheme:\n  fonts:\n    heading:\n      family: Arial\n      fallbacks: [sans-serif]\n  typeStyles:\n    title:\n      font: heading\n      size: 30\n      weight: bold\n      lineSpacing: 1.05\n      alignment: start\n      color: ink\n  spacing:\n    pageMargin: 42\n  fills:\n    panel: panel\n  outlines:\n    panel:\n      color: rule\n      width: 1\n  lines:\n    rule:\n      color: rule\n      width: 1\n  geometry:\n    safeArea: {{top: 24, right: 24, bottom: 24, left: 24}}\n    footer: {{height: 18, gap: 12}}\n  patternDefaults:\n    footer: {{showSlideNumber: true, line: rule}}\nquality:\n  minimumFontSize: 9\n  minimumTextContrast: 4.5\n  safeArea: {{top: 24, right: 24, bottom: 24, left: 24}}\n  requiredAltText: true\n  allowedOverlapGroups: [intentional]\nslides:\n  - key: cover\n    pattern: cover\n    eyebrow: INTRODUCTION\n    title: Hello\n    subtitle: A concise overview\n    footer: Internal\n    statement: Measure the outcome\n    body: Explain the evidence\n    takeaway: Act on the evidence\n    owner: Product quality\n    items:\n      - title: Critical harms\n        body: No unresolved critical failures\n    columns: 2\n"
    )
    .unwrap();
    let mut json = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
    write!(
        json,
        r#"{{
            "schemaVersion": 1,
            "presentation": {{}},
            "theme": {{
                "fonts": {{
                    "heading": {{"family": "Arial", "fallbacks": ["sans-serif"]}}
                }},
                "typeStyles": {{
                    "title": {{
                        "font": "heading",
                        "size": 30,
                        "weight": "bold",
                        "lineSpacing": 1.05,
                        "alignment": "start",
                        "color": "ink"
                    }}
                }},
                "spacing": {{"pageMargin": 42}},
                "fills": {{"panel": "panel"}},
                "outlines": {{"panel": {{"color": "rule", "width": 1}}}},
                "lines": {{"rule": {{"color": "rule", "width": 1}}}},
                "geometry": {{
                    "safeArea": {{"top": 24, "right": 24, "bottom": 24, "left": 24}},
                    "footer": {{"height": 18, "gap": 12}}
                }},
                "patternDefaults": {{
                    "footer": {{"showSlideNumber": true, "line": "rule"}}
                }}
            }},
            "quality": {{
                "minimumFontSize": 9,
                "minimumTextContrast": 4.5,
                "safeArea": {{"top": 24, "right": 24, "bottom": 24, "left": 24}},
                "requiredAltText": true,
                "allowedOverlapGroups": ["intentional"]
            }},
            "slides": [{{
                "key": "cover",
                "pattern": "cover",
                "eyebrow": "INTRODUCTION",
                "title": "Hello",
                "subtitle": "A concise overview",
                "footer": "Internal",
                "statement": "Measure the outcome",
                "body": "Explain the evidence",
                "takeaway": "Act on the evidence",
                "owner": "Product quality",
                "items": [{{
                    "title": "Critical harms",
                    "body": "No unresolved critical failures"
                }}],
                "columns": 2
            }}]
        }}"#
    )
    .unwrap();

    let yaml_source = read_deck_source(yaml.path().to_str().unwrap(), &mut io::empty()).unwrap();
    let json_source = read_deck_source(json.path().to_str().unwrap(), &mut io::empty()).unwrap();

    assert_eq!(yaml_source, json_source);
}

#[test]
fn rejects_non_string_allowed_overlap_groups_in_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality:
  allowedOverlapGroups: [42]
slides: []
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {"allowedOverlapGroups": [42]},
            "slides": []
        }"#,
    ];

    for source in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(
            message.contains("quality.allowedOverlapGroups[0]"),
            "{message}"
        );
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn reads_typed_assets_from_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme: {}
assets:
  hero:
    url: https://example.com/hero.png
    checksum: sha256:0123456789abcdef
    altText: A customer reviewing account information
    placementPolicy: cover
quality: {}
slides: []
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "assets": {
                "hero": {
                    "url": "https://example.com/hero.png",
                    "checksum": "sha256:0123456789abcdef",
                    "altText": "A customer reviewing account information",
                    "placementPolicy": "cover"
                }
            },
            "quality": {},
            "slides": []
        }"#,
    ];

    for source in sources {
        let source = read_deck_source("-", &mut io::Cursor::new(source)).unwrap();
        let hero = &source.assets["hero"];

        assert_eq!(hero.url, "https://example.com/hero.png");
        assert_eq!(hero.checksum, "sha256:0123456789abcdef");
        assert_eq!(
            hero.alt_text.as_deref(),
            Some("A customer reviewing account information")
        );
        assert_eq!(hero.placement_policy, "cover");
    }
}

#[test]
fn allows_missing_asset_alt_text_for_later_quality_policy_validation() {
    let mut source = io::Cursor::new(
        r#"
schemaVersion: 1
presentation: {}
theme: {}
assets:
  hero:
    url: https://example.com/hero.png
    checksum: sha256:0123456789abcdef
    placementPolicy: cover
quality:
  requiredAltText: false
slides: []
"#,
    );

    let source = read_deck_source("-", &mut source).unwrap();

    assert!(source.assets["hero"].alt_text.is_none());
}

#[test]
fn rejects_unknown_asset_fields_with_the_exact_source_path() {
    let mut source = io::Cursor::new(
        r#"
schemaVersion: 1
presentation: {}
theme: {}
assets:
  hero:
    url: https://example.com/hero.png
    checksum: sha256:0123456789abcdef
    altText: A customer reviewing account information
    placementPolicy: cover
    license: internal
quality: {}
slides: []
"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("assets.hero.license"), "{message}");
    assert!(message.contains("unknown field `license`"), "{message}");
}

#[test]
fn rejects_invalid_asset_values_with_exact_source_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
assets:
  hero:
    url: 42
    checksum: sha256:0123456789abcdef
    altText: A customer reviewing account information
    placementPolicy: cover
quality: {}
slides: []
"#,
            "assets.hero.url",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "assets": {
                "hero": {
                    "url": "https://example.com/hero.png",
                    "checksum": ["sha256:0123456789abcdef"],
                    "altText": "A customer reviewing account information",
                    "placementPolicy": "cover"
                }
            },
            "quality": {},
            "slides": []
        }"#,
            "assets.hero.checksum",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme: {}
assets:
  hero:
    url: https://example.com/hero.png
    checksum: sha256:0123456789abcdef
    altText: 42
    placementPolicy: cover
quality: {}
slides: []
"#,
            "assets.hero.altText",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "assets": {
                "hero": {
                    "url": "https://example.com/hero.png",
                    "checksum": "sha256:0123456789abcdef",
                    "altText": "A customer reviewing account information",
                    "placementPolicy": {"fit": "cover"}
                }
            },
            "quality": {},
            "slides": []
        }"#,
            "assets.hero.placementPolicy",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_unknown_type_style_fields_with_the_exact_source_path() {
    let mut source = io::Cursor::new(
        r#"
schemaVersion: 1
presentation: {}
theme:
  typeStyles:
    title:
      font: heading
      size: 30
      lineSpacing: 1.05
      alignment: start
      color: ink
      fontWeight: bold
quality: {}
slides: []
"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("theme.typeStyles.title.fontWeight"),
        "{message}"
    );
    assert!(message.contains("unknown field `fontWeight`"), "{message}");
}

#[test]
fn rejects_invalid_type_style_values_with_exact_source_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme:
  typeStyles:
    title:
      font: 42
      size: 30
      lineSpacing: 1.05
      alignment: start
      color: ink
quality: {}
slides: []
"#,
            "theme.typeStyles.title.font",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {
                "typeStyles": {
                    "title": {
                        "font": "heading",
                        "size": "large",
                        "lineSpacing": 1.05,
                        "alignment": "start",
                        "color": "ink"
                    }
                }
            },
            "quality": {},
            "slides": []
        }"#,
            "theme.typeStyles.title.size",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme:
  typeStyles:
    title:
      font: heading
      size: 30
      weight: 700
      lineSpacing: 1.05
      alignment: start
      color: ink
quality: {}
slides: []
"#,
            "theme.typeStyles.title.weight",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_non_finite_type_style_numbers_with_exact_source_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme:
  typeStyles:
    title:
      font: heading
      size: .nan
      lineSpacing: 1.05
      alignment: start
      color: ink
quality: {}
slides: []
"#,
            "theme.typeStyles.title.size",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme:
  typeStyles:
    title:
      font: heading
      size: 30
      lineSpacing: .nan
      alignment: start
      color: ink
quality: {}
slides: []
"#,
            "theme.typeStyles.title.lineSpacing",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("number must be finite"), "{message}");
    }
}

#[test]
fn rejects_invalid_spacing_tokens_with_the_exact_source_path() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme:
  spacing:
    pageMargin: wide
quality: {}
slides: []
"#,
            "invalid type",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {"spacing": {"pageMargin": "wide"}},
            "quality": {},
            "slides": []
        }"#,
            "invalid type",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme:
  spacing:
    pageMargin: .nan
quality: {}
slides: []
"#,
            "number must be finite",
        ),
    ];

    for (source, expected_error) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("theme.spacing.pageMargin"), "{message}");
        assert!(message.contains(expected_error), "{message}");
    }
}

#[test]
fn rejects_invalid_fill_tokens_with_the_exact_source_path() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme:
  fills:
    panel: 42
quality: {}
slides: []
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {"fills": {"panel": {"color": "panel"}}},
            "quality": {},
            "slides": []
        }"#,
    ];

    for source in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("theme.fills.panel"), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_unknown_outline_token_fields_with_the_exact_source_path() {
    let mut source = io::Cursor::new(
        r#"
schemaVersion: 1
presentation: {}
theme:
  outlines:
    panel:
      color: rule
      width: 1
      opacity: 0.5
quality: {}
slides: []
"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("theme.outlines.panel.opacity"),
        "{message}"
    );
    assert!(message.contains("unknown field `opacity`"), "{message}");
}

#[test]
fn rejects_invalid_outline_token_values_with_exact_source_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme:
  outlines:
    panel:
      color: 42
      width: 1
quality: {}
slides: []
"#,
            "theme.outlines.panel.color",
            "invalid type",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {"outlines": {"panel": {"color": "rule", "width": "thin"}}},
            "quality": {},
            "slides": []
        }"#,
            "theme.outlines.panel.width",
            "invalid type",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme:
  outlines:
    panel:
      color: rule
      width: .nan
quality: {}
slides: []
"#,
            "theme.outlines.panel.width",
            "number must be finite",
        ),
    ];

    for (source, expected_path, expected_error) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains(expected_error), "{message}");
    }
}

#[test]
fn rejects_unknown_line_token_fields_with_the_exact_source_path() {
    let mut source = io::Cursor::new(
        r#"
schemaVersion: 1
presentation: {}
theme:
  lines:
    rule:
      color: rule
      width: 1
      dash: solid
quality: {}
slides: []
"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("theme.lines.rule.dash"), "{message}");
    assert!(message.contains("unknown field `dash`"), "{message}");
}

#[test]
fn rejects_invalid_line_token_values_with_exact_source_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme:
  lines:
    rule:
      color: 42
      width: 1
quality: {}
slides: []
"#,
            "theme.lines.rule.color",
            "invalid type",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {"lines": {"rule": {"color": "rule", "width": "thin"}}},
            "quality": {},
            "slides": []
        }"#,
            "theme.lines.rule.width",
            "invalid type",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme:
  lines:
    rule:
      color: rule
      width: .nan
quality: {}
slides: []
"#,
            "theme.lines.rule.width",
            "number must be finite",
        ),
    ];

    for (source, expected_path, expected_error) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains(expected_error), "{message}");
    }
}

#[test]
fn rejects_unknown_geometry_fields_with_the_exact_source_path() {
    let mut source = io::Cursor::new(
        r#"
schemaVersion: 1
presentation: {}
theme:
  geometry:
    footer:
      height: 18
      gap: 12
      offset: 4
quality: {}
slides: []
"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("theme.geometry.footer.offset"),
        "{message}"
    );
    assert!(message.contains("unknown field `offset`"), "{message}");
}

#[test]
fn rejects_invalid_geometry_values_with_exact_source_paths() {
    let sources = [
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {
                "geometry": {
                    "safeArea": {"top": "wide", "right": 24, "bottom": 24, "left": 24}
                }
            },
            "quality": {},
            "slides": []
        }"#,
            "theme.geometry.safeArea.top",
            "invalid type",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme:
  geometry:
    footer:
      height: 18
      gap: .nan
quality: {}
slides: []
"#,
            "theme.geometry.footer.gap",
            "number must be finite",
        ),
    ];

    for (source, expected_path, expected_error) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains(expected_error), "{message}");
    }
}

#[test]
fn rejects_unknown_pattern_default_fields_with_the_exact_source_path() {
    let mut source = io::Cursor::new(
        r#"
schemaVersion: 1
presentation: {}
theme:
  patternDefaults:
    footer:
      showSlideNumber: true
      line: rule
      position: bottom
quality: {}
slides: []
"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("theme.patternDefaults.footer.position"),
        "{message}"
    );
    assert!(message.contains("unknown field `position`"), "{message}");
}

#[test]
fn rejects_invalid_pattern_default_values_with_exact_source_paths() {
    let sources = [
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {
                "patternDefaults": {
                    "footer": {"showSlideNumber": "yes", "line": "rule"}
                }
            },
            "quality": {},
            "slides": []
        }"#,
            "theme.patternDefaults.footer.showSlideNumber",
        ),
        (
            r#"
schemaVersion: 1
presentation: {}
theme:
  patternDefaults:
    footer:
      showSlideNumber: true
      line: 42
quality: {}
slides: []
"#,
            "theme.patternDefaults.footer.line",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_unknown_top_level_fields() {
    let mut source = io::Cursor::new(
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {},
            "slides": [],
            "quallity": {}
        }"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("unknown field `quallity`"), "{message}");
    assert!(message.contains("stdin"), "{message}");
}

#[test]
fn rejects_unknown_presentation_fields_with_the_exact_source_path() {
    let mut source = io::Cursor::new(
        r#"{
            "schemaVersion": 1,
            "presentation": {"aspectRato": "wide"},
            "theme": {},
            "quality": {},
            "slides": []
        }"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("presentation.aspectRato"), "{message}");
    assert!(message.contains("unknown field `aspectRato`"), "{message}");
    assert!(message.contains("stdin"), "{message}");
}

#[test]
fn rejects_non_string_presentation_settings_with_exact_source_paths() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation:
  aspectRatio: 16
theme: {}
quality: {}
slides: []
"#,
            "presentation.aspectRatio",
        ),
        (
            r#"
schemaVersion: 1
presentation:
  language: 42
theme: {}
quality: {}
slides: []
"#,
            "presentation.language",
        ),
        (
            r#"
schemaVersion: 1
presentation:
  speakerNotes: false
theme: {}
quality: {}
slides: []
"#,
            "presentation.speakerNotes",
        ),
        (
            r#"
schemaVersion: 1
presentation:
  metadata:
    subject: 42
theme: {}
quality: {}
slides: []
"#,
            "presentation.metadata.subject",
        ),
        (
            r#"{
                "schemaVersion": 1,
                "presentation": {"aspectRatio": 16},
                "theme": {},
                "quality": {},
                "slides": []
            }"#,
            "presentation.aspectRatio",
        ),
        (
            r#"{
                "schemaVersion": 1,
                "presentation": {"language": 42},
                "theme": {},
                "quality": {},
                "slides": []
            }"#,
            "presentation.language",
        ),
        (
            r#"{
                "schemaVersion": 1,
                "presentation": {"speakerNotes": false},
                "theme": {},
                "quality": {},
                "slides": []
            }"#,
            "presentation.speakerNotes",
        ),
        (
            r#"{
                "schemaVersion": 1,
                "presentation": {"metadata": {"subject": 42}},
                "theme": {},
                "quality": {},
                "slides": []
            }"#,
            "presentation.metadata.subject",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_unknown_quality_fields_with_the_exact_source_path() {
    let mut source = io::Cursor::new(
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {},
            "quality": {"minimumFontSze": 9},
            "slides": []
        }"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("quality.minimumFontSze"), "{message}");
    assert!(
        message.contains("unknown field `minimumFontSze`"),
        "{message}"
    );
    assert!(message.contains("stdin"), "{message}");
}

#[test]
fn rejects_unknown_theme_groups_with_the_exact_source_path() {
    let mut source = io::Cursor::new(
        r##"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {"colour": {"ink": "#111111"}},
            "quality": {},
            "slides": []
        }"##,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("theme.colour"), "{message}");
    assert!(message.contains("unknown field `colour`"), "{message}");
    assert!(message.contains("stdin"), "{message}");
}

#[test]
fn reports_color_token_type_errors_with_the_exact_source_path() {
    let mut source = io::Cursor::new(
        r##"
schemaVersion: 1
presentation: {}
theme:
  colors:
    ink: ["#111111"]
quality: {}
slides: []
"##,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("theme.colors.ink"), "{message}");
    assert!(message.contains("invalid type"), "{message}");
}

#[test]
fn rejects_non_string_color_scalars_in_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme:
  colors:
    ink: 42
quality: {}
slides: []
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {"colors": {"ink": 42}},
            "quality": {},
            "slides": []
        }"#,
    ];

    for source in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("theme.colors.ink"), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_unknown_font_token_fields_with_the_exact_source_path() {
    let mut source = io::Cursor::new(
        r#"
schemaVersion: 1
presentation: {}
theme:
  fonts:
    heading:
      family: Arial
      fallback: [sans-serif]
quality: {}
slides: []
"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("theme.fonts.heading.fallback"),
        "{message}"
    );
    assert!(message.contains("unknown field `fallback`"), "{message}");
}

#[test]
fn supports_scalar_font_family_shorthand_in_yaml_and_json() {
    let sources = [
        r#"
schemaVersion: 1
presentation: {}
theme:
  fonts:
    heading: Arial
quality: {}
slides: []
"#,
        r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {"fonts": {"heading": "Arial"}},
            "quality": {},
            "slides": []
        }"#,
    ];

    for source in sources {
        let source = read_deck_source("-", &mut io::Cursor::new(source)).unwrap();
        let heading = &source.theme.fonts["heading"];

        assert_eq!(heading.family, "Arial");
        assert!(heading.fallbacks.is_empty());
    }
}

#[test]
fn rejects_non_string_font_values_in_yaml_and_json() {
    let sources = [
        (
            r#"
schemaVersion: 1
presentation: {}
theme:
  fonts:
    heading:
      family: 42
quality: {}
slides: []
"#,
            "theme.fonts.heading.family",
        ),
        (
            r#"{
            "schemaVersion": 1,
            "presentation": {},
            "theme": {"fonts": {"heading": {"family": "Arial", "fallbacks": [42]}}},
            "quality": {},
            "slides": []
        }"#,
            "theme.fonts.heading.fallbacks[0]",
        ),
    ];

    for (source, expected_path) in sources {
        let error = read_deck_source("-", &mut io::Cursor::new(source)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains(expected_path), "{message}");
        assert!(message.contains("invalid type"), "{message}");
    }
}

#[test]
fn rejects_unknown_safe_area_fields_with_the_exact_source_path() {
    let mut source = io::Cursor::new(
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality:
  safeArea:
    top: 24
    right: 24
    bottom: 24
    left: 24
    gutter: 12
slides: []
"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("quality.safeArea.gutter"), "{message}");
    assert!(message.contains("unknown field `gutter`"), "{message}");
    assert!(message.contains("line 11 column 5"), "{message}");
}

#[test]
fn reports_quality_value_type_errors_with_the_exact_source_path() {
    let mut source = io::Cursor::new(
        r#"
schemaVersion: 1
presentation: {}
theme: {}
quality:
  safeArea:
    top: [24]
    right: 24
    bottom: 24
    left: 24
slides: []
"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("quality.safeArea.top"), "{message}");
    assert!(message.contains("invalid type"), "{message}");
}

#[test]
fn reports_nested_yaml_type_errors_with_the_exact_source_path() {
    let mut source = io::Cursor::new(
        r#"
schemaVersion: 1
presentation:
  metadata:
    subject: [42]
theme: {}
quality: {}
slides: []
"#,
    );

    let error = read_deck_source("-", &mut source).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("presentation.metadata.subject"),
        "{message}"
    );
    assert!(message.contains("invalid type"), "{message}");
    assert!(message.contains("line 5 column 14"), "{message}");
}
