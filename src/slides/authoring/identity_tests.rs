use super::identity::{managed_element_object_id, managed_slide_object_id};

#[test]
fn derives_stable_valid_google_object_ids_from_managed_keys() {
    let slide_id = managed_slide_object_id("risk-tiering");
    let element_id = managed_element_object_id("risk-tiering", "title");

    assert_eq!(slide_id, managed_slide_object_id("risk-tiering"));
    assert_eq!(
        element_id,
        managed_element_object_id("risk-tiering", "title")
    );
    assert_eq!(slide_id, "goog_s_c4ee875e3ba8ba68dd0a541eeacc5cbe1f7a308f");
    assert_eq!(
        element_id,
        "goog_e_55f108d7ede2216ca423c6c7030a8ca62c783a0c"
    );
    assert_ne!(slide_id, element_id);

    for object_id in [slide_id, element_id] {
        assert!(object_id.starts_with("goog_"));
        assert!((5..=50).contains(&object_id.len()));
        assert!(object_id
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_'));
        assert!(object_id.chars().skip(1).all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | ':')
        }));
    }
}

#[test]
fn keeps_object_kinds_and_key_boundaries_in_the_hash_identity() {
    assert_ne!(
        managed_slide_object_id("risk-tiering"),
        managed_element_object_id("", "risk-tiering")
    );
    assert_ne!(
        managed_element_object_id("risk", "tiering:title"),
        managed_element_object_id("risk:tiering", "title")
    );
}
