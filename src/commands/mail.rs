use std::io::Write;

use anyhow::{Context, Result};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::cli::MailCommand;
use crate::mail::{get_message, GetMessageOptions};

pub fn run<S: AccountStore>(cmd: MailCommand, client: &AuthClient<'_, S>) -> Result<()> {
    match cmd {
        MailCommand::Read { message_id } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_read_to(
                client,
                message_id,
                &mut std::io::stdout(),
                None,
            ))
        }
    }
}

pub(super) async fn run_read_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    message_id: String,
    out: &mut impl Write,
    messages_url: Option<&str>,
) -> Result<()> {
    let options = get_message_options(message_id, messages_url);

    let message = get_message(client, &options)
        .await
        .context("failed to fetch GoogleMail Message")?;
    write_json_line(out, &message, "failed to serialize GoogleMail Message")
}

fn get_message_options(message_id: String, messages_url: Option<&str>) -> GetMessageOptions {
    let mut options = GetMessageOptions::new(message_id);
    if let Some(messages_url) = messages_url {
        options = options.with_messages_url(messages_url);
    }
    options
}

fn write_json_line(out: &mut impl Write, value: &serde_json::Value, context: &str) -> Result<()> {
    serde_json::to_writer(&mut *out, value).context(context.to_string())?;
    writeln!(out).context("failed to write output")?;
    Ok(())
}
