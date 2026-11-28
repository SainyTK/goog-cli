use clap::error::ErrorKind;
use clap::{CommandFactory, FromArgMatches};

use goog::{
    auth::account::resolve_account_store,
    auth::config::load_config,
    cli::{Cli, Command},
    commands,
    telemetry::{command_path, Outcome, Telemetry},
};

/// Matches the 8 MB main-thread stack Linux and macOS provide by default, and
/// the `RUST_MIN_STACK` value in `.cargo/config.toml`.
const CLI_STACK_SIZE: usize = 8 * 1024 * 1024;
const PANIC_EXIT_CODE: i32 = 101;

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

    let exit_code = cli.join().unwrap_or(PANIC_EXIT_CODE);

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

fn run_cli() -> i32 {
    let update_check = goog::update::start();
    let root = Cli::command();
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let telemetry = Telemetry::start(command_path(
        &root,
        args.iter().map(|arg| arg.to_string_lossy()),
    ));
    let (outcome, exit_code) = match root
        .try_get_matches_from(args)
        .and_then(|matches| Cli::from_arg_matches(&matches))
    {
        Ok(cli) => match run(cli) {
            Ok(()) => (Outcome::Success, 0),
            Err(err) => {
                eprintln!("error: {err:#}");
                (Outcome::Failure, 1)
            }
        },
        Err(error) => {
            let _ = error.print();
            (parse_outcome(error.kind()), error.exit_code())
        }
    };

    telemetry.finish(outcome, exit_code);
    update_check.finish();

    exit_code
}

fn parse_outcome(kind: ErrorKind) -> Outcome {
    match kind {
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayVersion
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => Outcome::Help,
        _ => Outcome::UsageError,
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
