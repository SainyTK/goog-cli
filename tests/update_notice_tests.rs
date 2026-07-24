use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn successful_commands_show_a_newer_release_without_corrupting_json_output() {
    let (releases_url, server) = serve_release(r#"{"tag_name":"v999.0.0"}"#);
    let cache_dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_goog"))
        .args(["version", "--json"])
        .env("GOOG_UPDATE_CHECK_URL", releases_url)
        .env(
            "GOOG_UPDATE_CACHE_PATH",
            cache_dir.path().join("update-check.json"),
        )
        .output()
        .unwrap();
    server.join().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let version: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(version["semanticVersion"], env!("CARGO_PKG_VERSION"));
    assert!(stderr.contains(&format!(
        "Update available: goog 999.0.0 (current: {})",
        env!("CARGO_PKG_VERSION")
    )));
    assert!(stderr.contains(
        "curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh"
    ));
}

#[test]
fn known_newer_release_is_shown_from_cache_without_network() {
    let (releases_url, server) = serve_release(r#"{"tag_name":"v999.0.0"}"#);
    let cache_dir = tempfile::tempdir().unwrap();
    let cache_path = cache_dir.path().join("update-check.json");
    let first = run_version(&releases_url, &cache_path);
    server.join().unwrap();

    assert!(first.status.success());

    let second = run_version(&releases_url, &cache_path);
    let stderr = String::from_utf8(second.stderr).unwrap();

    assert!(second.status.success());
    assert!(stderr.contains("Update available: goog 999.0.0"));
}

#[test]
fn help_output_also_shows_a_newer_release() {
    let (releases_url, server) = serve_release(r#"{"tag_name":"v999.0.0"}"#);
    let cache_dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_goog"))
        .arg("--help")
        .env("GOOG_UPDATE_CHECK_URL", releases_url)
        .env(
            "GOOG_UPDATE_CACHE_PATH",
            cache_dir.path().join("update-check.json"),
        )
        .output()
        .unwrap();
    server.join().unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("Usage: goog"));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("Update available: goog 999.0.0"));
}

#[test]
fn current_release_does_not_show_an_update_notice() {
    let (releases_url, server) = serve_release(format!(
        r#"{{"tag_name":"v{}"}}"#,
        env!("CARGO_PKG_VERSION")
    ));
    let cache_dir = tempfile::tempdir().unwrap();
    let output = run_version(&releases_url, &cache_dir.path().join("update-check.json"));
    server.join().unwrap();

    assert!(output.status.success());
    assert!(!String::from_utf8(output.stderr)
        .unwrap()
        .contains("Update available:"));
}

#[test]
fn command_errors_also_show_a_known_newer_release() {
    let (releases_url, server) = serve_release(r#"{"tag_name":"v999.0.0"}"#);
    let cache_dir = tempfile::tempdir().unwrap();
    let cache_path = cache_dir.path().join("update-check.json");
    let first = run_version(&releases_url, &cache_path);
    server.join().unwrap();

    assert!(first.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_goog"))
        .arg("not-a-command")
        .env("GOOG_UPDATE_CHECK_URL", releases_url)
        .env("GOOG_UPDATE_CACHE_PATH", cache_path)
        .output()
        .unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(!output.status.success());
    assert!(stderr.contains("unrecognized subcommand"));
    assert!(stderr.contains("Update available: goog 999.0.0"));
}

fn run_version(releases_url: &str, cache_path: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_goog"))
        .arg("version")
        .env("GOOG_UPDATE_CHECK_URL", releases_url)
        .env("GOOG_UPDATE_CACHE_PATH", cache_path)
        .output()
        .unwrap()
}

fn serve_release(body: impl Into<String>) -> (String, thread::JoinHandle<()>) {
    let body = body.into();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < deadline,
                        "the CLI did not request the latest release"
                    );
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("failed to accept update request: {error}"),
            }
        };
        let mut request = [0; 2048];
        let _ = stream.read(&mut request).unwrap();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });

    (format!("http://{address}/releases/latest"), server)
}
