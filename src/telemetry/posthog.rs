use std::time::Duration;

use chrono::{Local, Utc};
use serde_json::json;

use super::event::{hour_timestamp, CommandEvent, EVENT_NAME};
use super::settings::Project;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

pub fn capture(project: &Project, device_id: &str, event: &CommandEvent) {
    let body = json!({
        "api_key": project.key,
        "event": EVENT_NAME,
        "distinct_id": device_id,
        "timestamp": hour_timestamp(Utc::now()),
        "properties": event.properties(Local::now()),
    });
    let Ok(client) = reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(concat!("goog/", env!("CARGO_PKG_VERSION")))
        .build()
    else {
        return;
    };
    let _ = client
        .post(format!("{}/i/v0/e/", project.host.trim_end_matches('/')))
        .json(&body)
        .send();
}
