use anyhow::{Context, Result};
use dialoguer::Input;
use goog_auth::{
    config::{load_config, save_config, OAuthAppConfig},
    setup::parse_credentials_file,
};

use crate::cli::AuthCommand;

const SETUP_GUIDE: &str = "\
Setting up your OAuth App. Follow these steps in the Google Cloud Console:

  1. Open https://console.cloud.google.com and sign in.
  2. Create a new project or select an existing one.
  3. Go to \"APIs & Services\" > \"Library\" and enable the APIs you need
     (e.g. \"Google Drive API\").
  4. Go to \"APIs & Services\" > \"OAuth consent screen\".
     - Choose \"External\" user type and fill in the required fields.
     - Add the scopes your commands will use.
     - Add your Google account email as a test user.
  5. Go to \"APIs & Services\" > \"Credentials\".
  6. Click \"Create Credentials\" > \"OAuth client ID\".
  7. Choose \"Desktop app\" as the application type.
  8. Click \"Create\", then download the JSON file (client_secret_*.json).

Enter the path to the downloaded file below.
";

pub fn run(cmd: AuthCommand) -> Result<()> {
    match cmd {
        AuthCommand::Setup { credentials } => run_setup(credentials),
        AuthCommand::Login { .. } => not_yet_implemented("auth login"),
        AuthCommand::List { .. } => not_yet_implemented("auth list"),
        AuthCommand::Switch { .. } => not_yet_implemented("auth switch"),
    }
}

fn run_setup(credentials_flag: Option<String>) -> Result<()> {
    let path = match credentials_flag {
        Some(p) => p,
        None => {
            println!("{SETUP_GUIDE}");
            Input::new()
                .with_prompt("Path to client_secret_*.json")
                .interact_text()
                .context("failed to read credentials path from stdin")?
        }
    };

    let creds =
        parse_credentials_file(&path).with_context(|| format!("failed to load credentials from {path}"))?;

    let mut config = load_config().context("failed to load config")?;
    config.oauth_app = Some(OAuthAppConfig {
        client_id: creds.client_id,
        client_secret: creds.client_secret,
    });
    save_config(&config).context("failed to save config")?;

    let config_path = goog_auth::config::config_path().context("could not determine config path")?;
    println!(
        "OAuth App credentials saved to {}",
        config_path.display()
    );

    Ok(())
}

fn not_yet_implemented(cmd: &str) -> Result<()> {
    println!("{cmd}: not yet implemented");
    Ok(())
}
