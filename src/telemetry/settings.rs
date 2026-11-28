use crate::auth::config::load_config;

const PROJECT_KEY: Option<&str> = option_env!("GOOG_POSTHOG_PROJECT_KEY");
const DEFAULT_HOST: &str = "https://us.i.posthog.com";
const HOST_ENV_VAR: &str = "GOOG_TELEMETRY_HOST";
const OPT_OUT_ENV_VAR: &str = "GOOG_TELEMETRY";
const DO_NOT_TRACK_ENV_VAR: &str = "DO_NOT_TRACK";
const CI_ENV_VAR: &str = "CI";

pub const NOTICE_VERSION: u32 = 2;

pub const NOTICE: &str = "goog sends usage counts (command name, result, duration, local hour and weekday, version and OS) to help decide what to build next.
Devices are counted by a random ID and Google accounts by a one-way hash of the account number.
Arguments, file IDs, file contents and email addresses are never sent.
Turn it off with GOOG_TELEMETRY=off or `telemetry = false` under [settings] in ~/.goog/config.toml.";

pub struct Project {
    pub key: String,
    pub host: String,
}

pub fn enabled_project() -> Option<Project> {
    let key = PROJECT_KEY.filter(|key| !key.is_empty())?;
    if opted_out_by_environment() || opted_out_by_config() {
        return None;
    }
    let host = std::env::var(HOST_ENV_VAR)
        .ok()
        .filter(|host| !host.is_empty())
        .or_else(|| option_env!("GOOG_POSTHOG_HOST").map(str::to_string))
        .unwrap_or_else(|| DEFAULT_HOST.to_string());
    Some(Project {
        key: key.to_string(),
        host,
    })
}

fn opted_out_by_environment() -> bool {
    let opted_out = std::env::var(OPT_OUT_ENV_VAR).is_ok_and(|value| is_off(&value));
    let do_not_track =
        std::env::var(DO_NOT_TRACK_ENV_VAR).is_ok_and(|value| !value.is_empty() && !is_off(&value));
    let in_ci = std::env::var_os(CI_ENV_VAR).is_some_and(|value| !value.is_empty());
    opted_out || do_not_track || in_ci
}

fn opted_out_by_config() -> bool {
    load_config()
        .ok()
        .and_then(|config| config.settings)
        .and_then(|settings| settings.telemetry)
        == Some(false)
}

fn is_off(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "off" | "false" | "no"
    )
}
