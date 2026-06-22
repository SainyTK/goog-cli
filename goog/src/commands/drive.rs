use anyhow::Result;

use crate::cli::DriveCommand;

pub fn run(cmd: DriveCommand) -> Result<()> {
    let name = match &cmd {
        DriveCommand::List { .. } => "drive list",
        DriveCommand::Download { .. } => "drive download",
        DriveCommand::Upload { .. } => "drive upload",
    };
    println!("{name}: not yet implemented");
    Ok(())
}
