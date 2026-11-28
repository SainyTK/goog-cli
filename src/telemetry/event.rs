use std::time::Duration;

use chrono::{DateTime, Datelike, Local, Timelike, Utc};
use serde_json::{json, Value};

pub const EVENT_NAME: &str = "command";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Failure,
    UsageError,
    Help,
}

impl Outcome {
    fn as_str(self) -> &'static str {
        match self {
            Outcome::Success => "success",
            Outcome::Failure => "failure",
            Outcome::UsageError => "usage_error",
            Outcome::Help => "help",
        }
    }
}

pub struct CommandEvent {
    pub command: Vec<String>,
    pub outcome: Outcome,
    pub exit_code: i32,
    pub duration: Duration,
    pub account_id: Option<String>,
}

impl CommandEvent {
    pub fn properties(&self, now: DateTime<Local>) -> Value {
        json!({
            "command": self.command.join(" "),
            "service": self.command.first(),
            "outcome": self.outcome.as_str(),
            "exit_code": self.exit_code,
            "duration_ms": u64::try_from(self.duration.as_millis()).unwrap_or(u64::MAX),
            "account_id": self.account_id,
            "version": env!("CARGO_PKG_VERSION"),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "local_hour": now.hour(),
            "local_weekday": now.weekday().num_days_from_monday(),
            "$process_person_profile": false,
            "$geoip_disable": true,
            "$ip": Value::Null,
        })
    }
}

pub fn hour_timestamp(now: DateTime<Utc>) -> String {
    now.format("%Y-%m-%dT%H:00:00Z").to_string()
}
