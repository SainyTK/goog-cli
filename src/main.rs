use clap::Parser;

use goog::{
    auth::account::resolve_account_store,
    auth::config::load_config,
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
    let output_json_by_default = config
        .settings
        .as_ref()
        .and_then(|settings| settings.output.as_deref())
        == Some("json");

    match cli.command {
        Command::Auth { command } => commands::auth::run(command),
        Command::Drive { command } => {
            let store = resolve_account_store()?;
            commands::drive::run(
                command,
                &config,
                &store,
                cli.account.as_deref(),
                output_json_by_default,
                cli.quiet,
            )
        }
        Command::Docs { command } => {
            let store = resolve_account_store()?;
            commands::docs::run(
                command,
                &config,
                &store,
                cli.account.as_deref(),
                output_json_by_default,
                cli.quiet,
            )
        }
        Command::Mail { command } => {
            let store = resolve_account_store()?;
            commands::mail::run(command, &config, &store, cli.account.as_deref(), cli.quiet)
        }
        Command::Sheets { command } => {
            let store = resolve_account_store()?;
            commands::sheets::run(
                command,
                &config,
                &store,
                cli.account.as_deref(),
                output_json_by_default,
                cli.quiet,
            )
        }
    }
}
