use clap::Parser;

use goog::{
    auth::account::KeyringStore,
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
            let store = KeyringStore;
            let client = AuthClient::from_config(config, &store, resolved_account.as_deref())?;
            commands::drive::run(command, &client, output_json_by_default, cli.quiet)
        }
        Command::Docs { command } => {
            let store = KeyringStore;
            let client = AuthClient::from_config(config, &store, resolved_account.as_deref())?;
            commands::docs::run(command, &client)
        }
        Command::Mail { command } => {
            let store = KeyringStore;
            let client = AuthClient::from_config(config, &store, resolved_account.as_deref())?;
            commands::mail::run(command, &client)
        }
    }
}
