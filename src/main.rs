use clap::Parser;

use goog::{
    auth::account::resolve_account_store,
    auth::client::AuthClient,
    auth::config::{load_config, resolve_account},
    cli::{Cli, Command},
    commands,
};

fn main() {
    if let Err(err) = run(Cli::parse()) {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let config = load_config()?;
    let resolved_account = resolve_account(&config, cli.account.as_deref())?;

    match cli.command {
        Command::Auth { command } => commands::auth::run(command, resolved_account),
        Command::Drive { command } => {
            let output_json_by_default = config
                .settings
                .as_ref()
                .and_then(|settings| settings.output.as_deref())
                == Some("json");
            let store = resolve_account_store();
            let client = AuthClient::from_config(config, &store, resolved_account.as_deref())?;
            commands::drive::run(command, &client, output_json_by_default, cli.quiet)
        }
        Command::Docs { command } => {
            let store = resolve_account_store();
            commands::docs::run(command, &config, &store, cli.account.as_deref())
        }
        Command::Mail { command } => {
            let store = resolve_account_store();
            let client = AuthClient::from_config(config, &store, resolved_account.as_deref())?;
            commands::mail::run(command, &client, cli.quiet)
        }
        Command::Sheets { command } => {
            let store = resolve_account_store();
            let client = AuthClient::from_config(config, &store, resolved_account.as_deref())?;
            commands::sheets::run(command, &client)
        }
    }
}
