use std::io::{self, Write};

use super::source::read_deck_source;

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
    assert_eq!(source.slides.len(), 14);
    assert_eq!(source.slides[0].key, "cover");
    assert_eq!(source.slides[13].key, "operating-principle");
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
    assert_eq!(source.slides[0].content["title"], "Hello");
}

#[test]
fn yaml_and_json_sources_have_semantic_parity() {
    let mut yaml = tempfile::Builder::new().suffix(".yaml").tempfile().unwrap();
    write!(
        yaml,
        "schemaVersion: 1\npresentation: {{}}\ntheme: {{}}\nquality: {{}}\nslides:\n  - key: cover\n    pattern: cover\n    title: Hello\n"
    )
    .unwrap();
    let mut json = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
    write!(
        json,
        r#"{{
            "schemaVersion": 1,
            "presentation": {{}},
            "theme": {{}},
            "quality": {{}},
            "slides": [{{"key": "cover", "pattern": "cover", "title": "Hello"}}]
        }}"#
    )
    .unwrap();

    let yaml_source = read_deck_source(yaml.path().to_str().unwrap(), &mut io::empty()).unwrap();
    let json_source = read_deck_source(json.path().to_str().unwrap(), &mut io::empty()).unwrap();

    assert_eq!(yaml_source, json_source);
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
