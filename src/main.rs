mod auth;
mod cli;
mod commands;
mod drive;

use clap::Parser;

use auth::config::{load_config, resolve_account};
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    let result = load_config()
        .and_then(|config| resolve_account(&config, cli.account.as_deref()))
        .map_err(anyhow::Error::from)
        .and_then(|account| match cli.command {
            Command::Auth { command } => commands::auth::run(command, account),
            Command::Drive { command } => commands::drive::run(command, account),
        });

    if let Err(err) = result {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
