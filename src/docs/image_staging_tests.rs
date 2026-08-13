use std::fs;
use std::path::Path;

use super::image_staging::{cleanup_with_command, stage_with_command};

#[cfg(unix)]
fn adapter(dir: &Path, stage_response: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join("adapter with spaces");
    let log = dir.join("adapter.log");
    let script = format!(
        "#!/bin/sh\nread request\nprintf '%s\\n' \"$request\" >> \"{}\"\ncase \"$request\" in\n  *stage*) printf '%s\\n' '{stage_response}' ;;\n  *) printf '%s\\n' '{{}}' ;;\nesac\n",
        log.display()
    );
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

#[test]
#[cfg(unix)]
fn adapter_receives_absolute_special_character_path_and_cleanup_token_over_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let image = dir.path().join("- image 'quoted' 测试.png");
    fs::write(&image, b"image").unwrap();
    let log = dir.path().join("adapter.log");
    let command = adapter(
        dir.path(),
        r#"{"uri":"https://example.test/signed-image","cleanupToken":"opaque token","expiresAt":"2026-07-18T00:00:00Z"}"#,
    );
    let staged = stage_with_command(&command, &image, "image/png").unwrap();
    cleanup_with_command(&command, staged.cleanup_token.as_deref().unwrap()).unwrap();

    let requests: Vec<serde_json::Value> = fs::read_to_string(log)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(requests[0]["action"], "stage");
    assert_eq!(
        requests[0]["path"],
        image.canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(requests[0]["mimeType"], "image/png");
    assert_eq!(requests[1]["action"], "cleanup");
    assert_eq!(requests[1]["cleanupToken"], "opaque token");
    assert_eq!(staged.expires_at.as_deref(), Some("2026-07-18T00:00:00Z"));
}

#[test]
#[cfg(unix)]
fn adapter_rejects_non_https_and_credential_bearing_urls() {
    let dir = tempfile::tempdir().unwrap();
    let image = dir.path().join("image.png");
    fs::write(&image, b"image").unwrap();
    for uri in [
        "http://example.test/image",
        "https://user:secret@example.test/image",
    ] {
        let command = adapter(dir.path(), &format!(r#"{{"uri":"{uri}"}}"#));
        let error = stage_with_command(&command, &image, "image/png").unwrap_err();
        assert!(error.to_string().contains(if uri.starts_with("http:") {
            "absolute HTTPS"
        } else {
            "must not contain user credentials"
        }));
    }
}
