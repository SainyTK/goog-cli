use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    for name in [
        "GOOG_BUILD_GIT_COMMIT",
        "GOOG_BUILD_GIT_DIRTY",
        "GOOG_BUILD_GIT_DISTANCE",
        "GOOG_BUILD_SOURCE_TAG",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }

    track_git_state();

    let semantic_version = env::var("CARGO_PKG_VERSION").expect("Cargo provides package version");
    let target = env::var("TARGET").expect("Cargo provides build target");
    let commit = build_value("GOOG_BUILD_GIT_COMMIT", &["rev-parse", "HEAD"])
        .unwrap_or_else(|| "unknown".to_owned());
    let source_tag = build_value(
        "GOOG_BUILD_SOURCE_TAG",
        &["describe", "--tags", "--match", "v*", "--abbrev=0"],
    );
    let distance = env::var("GOOG_BUILD_GIT_DISTANCE")
        .ok()
        .or_else(|| {
            source_tag
                .as_deref()
                .and_then(|tag| git(&["rev-list", "--count", &format!("{tag}..HEAD")]))
        })
        .and_then(|value| value.parse::<u64>().ok());
    let dirty = env::var("GOOG_BUILD_GIT_DIRTY")
        .ok()
        .and_then(|value| parse_bool(&value))
        .unwrap_or_else(git_is_dirty);
    let short_commit = commit.chars().take(7).collect::<String>();
    let exact_source_tag = if dirty {
        None
    } else {
        git(&["describe", "--tags", "--match", "v*", "--exact-match"])
            .or_else(|| exact_source_tag_from_environment(source_tag.as_deref(), distance))
    };

    let (display_version, release_channel) = match exact_source_tag.as_deref() {
        Some(tag) if tag == format!("v{semantic_version}") => (semantic_version.clone(), "stable"),
        Some(tag) if tag.starts_with(&format!("v{semantic_version}-preview.")) => {
            (tag.trim_start_matches('v').to_owned(), "preview")
        }
        _ => {
            let distance = distance
                .map(|value| format!(".{value}"))
                .unwrap_or_default();
            let dirty = if dirty { ".dirty" } else { "" };
            (
                format!("{semantic_version}-dev{distance}+{short_commit}{dirty}"),
                "development",
            )
        }
    };

    emit("GOOG_SEMANTIC_VERSION", &semantic_version);
    emit("GOOG_DISPLAY_VERSION", &display_version);
    emit("GOOG_GIT_COMMIT", &commit);
    emit("GOOG_GIT_DIRTY", if dirty { "true" } else { "false" });
    emit(
        "GOOG_GIT_DISTANCE",
        &distance.map(|value| value.to_string()).unwrap_or_default(),
    );
    emit("GOOG_SOURCE_TAG", source_tag.as_deref().unwrap_or_default());
    emit("GOOG_RELEASE_CHANNEL", release_channel);
    emit("GOOG_BUILD_TARGET", &target);
}

fn exact_source_tag_from_environment(
    source_tag: Option<&str>,
    distance: Option<u64>,
) -> Option<String> {
    let environment_supplied = env::var("GOOG_BUILD_SOURCE_TAG").is_ok()
        && env::var("GOOG_BUILD_GIT_DISTANCE").is_ok()
        && env::var("GOOG_BUILD_GIT_COMMIT").is_ok();

    (environment_supplied && distance == Some(0))
        .then(|| source_tag.map(str::to_owned))
        .flatten()
}

fn build_value(environment_name: &str, git_args: &[&str]) -> Option<String> {
    env::var(environment_name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| git(git_args))
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn git_is_dirty() -> bool {
    git(&["status", "--porcelain=v1", "--untracked-files=no"])
        .is_some_and(|status| !status.is_empty())
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

fn track_git_state() {
    let Some(git_dir) = git(&["rev-parse", "--git-dir"]) else {
        return;
    };
    let git_dir = PathBuf::from(git_dir);
    for path in [git_dir.join("HEAD"), git_dir.join("index")] {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    if let Some(tracked_files) = git(&["ls-files"]) {
        for path in tracked_files.lines() {
            println!("cargo:rerun-if-changed={path}");
        }
    }

    if let Some(common_dir) = git(&["rev-parse", "--git-common-dir"]) {
        let common_dir = PathBuf::from(common_dir);
        println!(
            "cargo:rerun-if-changed={}",
            common_dir.join("packed-refs").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            common_dir.join("refs/tags").display()
        );
    }
}

fn emit(name: &str, value: &str) {
    println!("cargo:rustc-env={name}={value}");
}
