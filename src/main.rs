use clap::Parser;

use goog::{
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
        Command::Drive { command } => commands::drive::run(command, resolved_account),
    }
}
