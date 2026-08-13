use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use semver::Version;
use serde::{Deserialize, Serialize};

const RELEASES_URL: &str = "https://api.github.com/repos/SainyTK/goog-cli/releases?per_page=100";
const INSTALLER_URL: &str = "https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh";
const KNOWN_UPDATE_CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const NO_UPDATE_CACHE_TTL: Duration = Duration::from_secs(15 * 60);
const FAILURE_RETRY_TTL: Duration = Duration::from_secs(60 * 60);

pub struct UpdateCheck {
    handle: thread::JoinHandle<Option<UpdateNotice>>,
}

impl UpdateCheck {
    pub fn finish(self) {
        if let Ok(Some(notice)) = self.handle.join() {
            eprintln!("\n{notice}");
        }
    }
}

#[derive(Debug)]
struct UpdateNotice {
    current: Version,
    latest: Version,
}

impl fmt::Display for UpdateNotice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Update available: goog {} (current: {})\nUpdate with:\n  curl -fsSL {INSTALLER_URL} | sh -s -- --version v{}",
            self.latest, self.current, self.latest
        )
    }
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCache {
    includes_preview: Option<bool>,
    last_checked_at: Option<u64>,
    last_attempt_at: Option<u64>,
    latest_version: Option<String>,
}

pub fn start() -> UpdateCheck {
    UpdateCheck {
        handle: thread::spawn(check),
    }
}

fn check() -> Option<UpdateNotice> {
    let current = Version::parse(env!("CARGO_PKG_VERSION")).ok()?;
    let includes_preview = !current.pre.is_empty();
    let cache_path = cache_path()?;
    let now = unix_timestamp()?;
    let mut cache = load_cache(&cache_path).unwrap_or_default();
    if cache.includes_preview != Some(includes_preview) {
        cache = UpdateCache {
            includes_preview: Some(includes_preview),
            ..UpdateCache::default()
        };
    }
    let latest = if should_use_cache(&cache, &current, now) {
        cached_version(&cache)
    } else {
        cache.last_attempt_at = Some(now);
        match fetch_latest_release(includes_preview) {
            Some(latest) => {
                cache.last_checked_at = Some(now);
                cache.last_attempt_at = None;
                cache.latest_version = Some(latest.to_string());
                save_cache(&cache_path, &cache);
                Some(latest)
            }
            None => {
                save_cache(&cache_path, &cache);
                cached_version(&cache)
            }
        }
    }?;

    (latest > current).then_some(UpdateNotice { current, latest })
}

fn fetch_latest_release(includes_preview: bool) -> Option<Version> {
    let release_url =
        std::env::var("GOOG_UPDATE_CHECK_URL").unwrap_or_else(|_| RELEASES_URL.into());
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .user_agent(concat!("goog/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    let release = client
        .get(release_url)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json::<Vec<GitHubRelease>>()
        .ok()?;
    select_latest_release(release, includes_preview)
}

fn select_latest_release(releases: Vec<GitHubRelease>, includes_preview: bool) -> Option<Version> {
    releases
        .into_iter()
        .filter(|release| !release.draft)
        .filter_map(|release| Version::parse(release.tag_name.trim_start_matches('v')).ok())
        .filter(|version| includes_preview || version.pre.is_empty())
        .max()
}

fn should_use_cache(cache: &UpdateCache, current: &Version, now: u64) -> bool {
    let checked_result_is_fresh = cache.last_checked_at.is_some_and(|checked| {
        cached_version(cache).is_some_and(|latest| {
            let ttl = if latest > *current {
                KNOWN_UPDATE_CACHE_TTL
            } else {
                NO_UPDATE_CACHE_TTL
            };
            now.saturating_sub(checked) < ttl.as_secs()
        })
    });
    let recent_failed_attempt = cache
        .last_attempt_at
        .is_some_and(|attempt| now.saturating_sub(attempt) < FAILURE_RETRY_TTL.as_secs());

    checked_result_is_fresh || recent_failed_attempt
}

fn cached_version(cache: &UpdateCache) -> Option<Version> {
    Version::parse(cache.latest_version.as_deref()?).ok()
}

fn cache_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("GOOG_UPDATE_CACHE_PATH") {
        return Some(PathBuf::from(path));
    }

    Some(dirs::cache_dir()?.join("goog").join("update-check.json"))
}

fn load_cache(path: &Path) -> Option<UpdateCache> {
    let contents = std::fs::read(path).ok()?;
    serde_json::from_slice(&contents).ok()
}

fn save_cache(path: &Path, cache: &UpdateCache) {
    let Some(parent) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(contents) = serde_json::to_vec(cache) else {
        return;
    };
    let Ok(mut temporary) = tempfile::NamedTempFile::new_in(parent) else {
        return;
    };
    if temporary.write_all(&contents).is_ok() {
        let _ = temporary.persist(path);
    }
}

fn unix_timestamp() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|age| age.as_secs())
}

#[cfg(test)]
mod tests {
    use super::{select_latest_release, GitHubRelease};
    use semver::Version;

    fn release(tag_name: &str) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag_name.to_owned(),
            draft: false,
        }
    }

    #[test]
    fn stable_release_checks_ignore_preview_candidates() {
        let latest = select_latest_release(
            vec![release("v999.0.0-preview.2"), release("v998.0.0")],
            false,
        );

        assert_eq!(latest, Some(Version::parse("998.0.0").unwrap()));
    }

    #[test]
    fn preview_release_checks_consider_preview_and_stable_candidates() {
        let latest = select_latest_release(
            vec![release("v999.0.0-preview.2"), release("v998.0.0")],
            true,
        );

        assert_eq!(latest, Some(Version::parse("999.0.0-preview.2").unwrap()));
    }
}
