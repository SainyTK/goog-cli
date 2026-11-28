mod command_path;
mod event;
mod identity;
mod posthog;
mod settings;

use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

pub use command_path::command_path;
pub use event::Outcome;

use event::CommandEvent;

const SEND_WAIT: Duration = Duration::from_millis(1500);

static ACTING_ACCOUNT: Mutex<Option<String>> = Mutex::new(None);

pub fn record_acting_account(email: &str) {
    if let Ok(mut account) = ACTING_ACCOUNT.lock() {
        *account = Some(email.to_string());
    }
}

pub struct Telemetry {
    started_at: Instant,
    command: Vec<String>,
}

impl Telemetry {
    pub fn start(command: Vec<String>) -> Self {
        Self {
            started_at: Instant::now(),
            command,
        }
    }

    pub fn finish(self, outcome: Outcome, exit_code: i32) {
        let Some(project) = settings::enabled_project() else {
            return;
        };
        let mut identity = identity::load();
        if identity.show_notice_once(settings::NOTICE_VERSION) {
            eprintln!("\n{}", settings::NOTICE);
            return;
        }
        let acting_account = ACTING_ACCOUNT
            .lock()
            .ok()
            .and_then(|account| account.clone());
        let duration = self.started_at.elapsed();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let account_id = acting_account.and_then(|email| identity.account_id(&email));
            let event = CommandEvent {
                command: self.command,
                outcome,
                exit_code,
                duration,
                account_id,
            };
            posthog::capture(&project, &identity.device_id, &event);
            let _ = sender.send(());
        });
        let _ = receiver.recv_timeout(SEND_WAIT);
    }
}
