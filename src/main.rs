use clap::Parser;

use goog::{
    auth::account::resolve_account_store,
    auth::config::load_config,
    cli::{Cli, Command},
    commands,
};

/// Matches the 8 MB main-thread stack Linux and macOS provide by default, and
/// the `RUST_MIN_STACK` value in `.cargo/config.toml`.
const CLI_STACK_SIZE: usize = 8 * 1024 * 1024;

fn main() {
    // Windows reserves 1 MB for the main thread, which the command tree in
    // `cli.rs` overflows before clap finishes building it. `RUST_MIN_STACK`
    // cannot fix that because it only applies to spawned threads, so run the
    // CLI on one with an explicit stack size.
    let cli = std::thread::Builder::new()
        .name("goog".to_string())
        .stack_size(CLI_STACK_SIZE)
        .spawn(run_cli)
        .expect("failed to start the goog CLI thread");

    let exit_code = match cli.join() {
        Ok(exit_code) => exit_code,
        // The panic hook already reported the failure; keep the exit status
        // Rust uses for a panicking process.
        Err(_) => 101,
    };

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

fn run_cli() -> i32 {
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

    exit_code
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
