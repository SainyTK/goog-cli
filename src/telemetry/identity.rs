use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::auth::account::{resolve_account_store, AccountStore};
use crate::auth::login::GOOGLE_USERINFO_URL;

const STATE_PATH_ENV_VAR: &str = "GOOG_TELEMETRY_STATE_PATH";
const ACCOUNT_ID_SALT: &str = "goog-cli-telemetry:";
const ACCOUNT_ID_LENGTH: usize = 32;
const USERINFO_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub device_id: String,
    #[serde(default)]
    notice_version: u32,
    #[serde(default)]
    account_ids: HashMap<String, String>,
}

#[derive(Deserialize)]
struct UserInfo {
    sub: String,
}

pub fn load() -> Identity {
    let mut identity = state_path()
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|contents| serde_json::from_slice::<Identity>(&contents).ok())
        .unwrap_or_default();
    if identity.device_id.is_empty() {
        identity.device_id = uuid::Uuid::new_v4().to_string();
        identity.save();
    }
    identity
}

impl Identity {
    pub fn show_notice_once(&mut self, version: u32) -> bool {
        if self.notice_version >= version {
            return false;
        }
        self.notice_version = version;
        self.save();
        true
    }

    pub fn account_id(&mut self, email: &str) -> Option<String> {
        if let Some(account_id) = self.account_ids.get(email) {
            return Some(account_id.clone());
        }
        let account_id = hash_subject(&fetch_subject(email)?);
        self.account_ids
            .insert(email.to_string(), account_id.clone());
        self.save();
        Some(account_id)
    }

    fn save(&self) {
        let Some(path) = state_path() else {
            return;
        };
        let Some(parent) = path.parent() else {
            return;
        };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        let Ok(contents) = serde_json::to_vec_pretty(self) else {
            return;
        };
        let Ok(mut temporary) = tempfile::NamedTempFile::new_in(parent) else {
            return;
        };
        if temporary.write_all(&contents).is_ok() {
            let _ = temporary.persist(path);
        }
    }
}

fn hash_subject(subject: &str) -> String {
    let digest = Sha256::digest(format!("{ACCOUNT_ID_SALT}{subject}"));
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .chars()
        .take(ACCOUNT_ID_LENGTH)
        .collect()
}

fn fetch_subject(email: &str) -> Option<String> {
    let token = resolve_account_store().ok()?.load_token(email).ok()??;
    let client = reqwest::blocking::Client::builder()
        .timeout(USERINFO_TIMEOUT)
        .build()
        .ok()?;
    let info = client
        .get(GOOGLE_USERINFO_URL)
        .bearer_auth(token.access_token)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json::<UserInfo>()
        .ok()?;
    Some(info.sub)
}

fn state_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(STATE_PATH_ENV_VAR).filter(|path| !path.is_empty()) {
        return Some(PathBuf::from(path));
    }
    Some(dirs::home_dir()?.join(".goog").join("telemetry.json"))
}
