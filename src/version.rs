use serde::Serialize;

pub const DISPLAY_VERSION: &str = env!("GOOG_DISPLAY_VERSION");

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    pub semantic_version: &'static str,
    pub display_version: &'static str,
    pub git_commit: &'static str,
    pub dirty: bool,
    pub distance_from_tag: Option<u64>,
    pub source_tag: Option<&'static str>,
    pub release_channel: &'static str,
    pub target: &'static str,
}

pub fn build_info() -> BuildInfo {
    BuildInfo {
        semantic_version: env!("GOOG_SEMANTIC_VERSION"),
        display_version: DISPLAY_VERSION,
        git_commit: env!("GOOG_GIT_COMMIT"),
        dirty: env!("GOOG_GIT_DIRTY") == "true",
        distance_from_tag: parse_optional_u64(env!("GOOG_GIT_DISTANCE")),
        source_tag: nonempty(env!("GOOG_SOURCE_TAG")),
        release_channel: env!("GOOG_RELEASE_CHANNEL"),
        target: env!("GOOG_BUILD_TARGET"),
    }
}

pub fn print(json: bool) -> anyhow::Result<()> {
    let info = build_info();
    if json {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("goog {}", info.display_version);
    }
    Ok(())
}

fn nonempty(value: &'static str) -> Option<&'static str> {
    (!value.is_empty()).then_some(value)
}

fn parse_optional_u64(value: &str) -> Option<u64> {
    (!value.is_empty()).then(|| value.parse().ok()).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_build_info_contains_safe_provenance() {
        let value = serde_json::to_value(build_info()).unwrap();

        assert_eq!(value["semanticVersion"], env!("CARGO_PKG_VERSION"));
        assert!(value["displayVersion"]
            .as_str()
            .unwrap()
            .starts_with(env!("CARGO_PKG_VERSION")));
        assert!(!value["gitCommit"].as_str().unwrap().is_empty());
        assert!(value["dirty"].is_boolean());
        assert!(!value["releaseChannel"].as_str().unwrap().is_empty());
        assert!(value["target"].as_str().unwrap().contains('-'));
        assert!(!value.as_object().unwrap().contains_key("workspacePath"));
        assert!(!value.as_object().unwrap().contains_key("hostname"));
    }
}
