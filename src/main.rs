use clap::Parser;

use goog::{
    cli::{Cli, Command},
    commands,
};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Auth { command } => commands::auth::run(command),
        Command::Drive { command } => commands::drive::run(command),
    };

    if let Err(err) = result {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
