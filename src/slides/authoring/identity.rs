use std::fmt::Write;

use sha2::{Digest, Sha256};

const COMPACT_HASH_BYTES: usize = 20;

pub(super) fn managed_slide_object_id(slide_key: &str) -> String {
    managed_object_id("s", &[slide_key])
}

pub(super) fn managed_element_object_id(slide_key: &str, element_key: &str) -> String {
    managed_object_id("e", &[slide_key, element_key])
}

fn managed_object_id(kind: &str, key_parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, kind);
    for key_part in key_parts {
        hash_field(&mut hasher, key_part);
    }
    let hash = hasher.finalize();
    let mut object_id = format!("goog_{kind}_");
    for byte in &hash[..COMPACT_HASH_BYTES] {
        write!(object_id, "{byte:02x}").expect("writing to a String cannot fail");
    }
    object_id
}

fn hash_field(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
}
