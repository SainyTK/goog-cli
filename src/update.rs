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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum ReleaseTrack {
    Stable,
    Preview,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCache {
    release_track: Option<ReleaseTrack>,
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
    let release_track = release_track(&current);
    let cache_path = cache_path()?;
    let now = unix_timestamp()?;
    let mut cache = load_cache(&cache_path).unwrap_or_default();
    if cache.release_track != Some(release_track) {
        cache = UpdateCache {
            release_track: Some(release_track),
            ..UpdateCache::default()
        };
    }
    let latest = if should_use_cache(&cache, &current, now) {
        cached_version(&cache)
    } else {
        cache.last_attempt_at = Some(now);
        match fetch_latest_release(release_track) {
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

fn release_track(current: &Version) -> ReleaseTrack {
    if current.pre.is_empty() {
        ReleaseTrack::Stable
    } else {
        ReleaseTrack::Preview
    }
}

fn fetch_latest_release(release_track: ReleaseTrack) -> Option<Version> {
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
    release
        .into_iter()
        .filter(|release| !release.draft)
        .filter_map(|release| Version::parse(release.tag_name.trim_start_matches('v')).ok())
        .filter(|version| release_track == ReleaseTrack::Preview || version.pre.is_empty())
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
