use clap::Parser;

use goog::{
    auth::account::resolve_account_store,
    auth::config::load_config,
    cli::{Cli, Command},
    commands,
};

fn main() {
    let update_check = goog::update::start();
    let exit_code = match Cli::try_parse() {
        Ok(cli) => match run(cli) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("error: {err:#}");
                1
            }
        },
        Err(error) => {
            let exit_code = error.exit_code();
            let _ = error.print();
            exit_code
        }
    };

    update_check.finish();

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    if let Command::Version { json } = cli.command {
        return goog::version::print(json);
    }

    let config = load_config()?;
    let output_json_by_default = config
        .settings
        .as_ref()
        .and_then(|settings| settings.output.as_deref())
        == Some("json");

    match cli.command {
        Command::Version { .. } => unreachable!("version exits before configuration is loaded"),
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
        Command::Slides { command } => {
            let store = resolve_account_store()?;
            commands::slides::run(
                command,
                &config,
                &store,
                cli.account.as_deref(),
                output_json_by_default,
                cli.quiet,
            )
        }
        Command::Calendar { command } => {
            let store = resolve_account_store()?;
            commands::calendar::run(
                command,
                &config,
                &store,
                cli.account.as_deref(),
                output_json_by_default,
            )
        }
    }
}
