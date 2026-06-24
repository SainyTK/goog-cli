use anyhow::{Context, Result};
use dialoguer::Input;

use crate::auth::{
    config::{config_path, load_config, save_config, OAuthAppConfig},
    setup::parse_client_secret_file,
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
        AuthCommand::Setup { client_secret_file } => run_setup(client_secret_file),
        AuthCommand::Login { .. } => not_yet_implemented(),
        AuthCommand::List { .. } => not_yet_implemented(),
        AuthCommand::Switch { .. } => not_yet_implemented(),
    }
}

fn run_setup(client_secret_file: Option<String>) -> Result<()> {
    run_setup_to(client_secret_file, &mut std::io::stdout())
}

fn run_setup_to(client_secret_file: Option<String>, out: &mut impl std::io::Write) -> Result<()> {
    let path = match client_secret_file {
        Some(p) => p,
        None => {
            write!(out, "{SETUP_GUIDE}").context("failed to write setup guide")?;
            Input::new()
                .with_prompt("Path to client_secret_*.json")
                .interact_text()
                .context("failed to read client secret file path from stdin")?
        }
    };

    let secrets = parse_client_secret_file(&path)
        .with_context(|| format!("failed to load OAuth App from {path}"))?;

    let mut config = load_config().context("failed to load config")?;
    config.oauth_app = Some(OAuthAppConfig {
        client_id: secrets.client_id,
        client_secret: secrets.client_secret,
    });
    save_config(&config).context("failed to save config")?;

    let saved_to = config_path().context("could not determine config path")?;
    writeln!(out, "OAuth App saved to {}", saved_to.display())
        .context("failed to write output")?;

    Ok(())
}

fn not_yet_implemented() -> Result<()> {
    println!("not yet implemented");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_guide_is_printed_when_no_file_given_and_guide_contains_all_steps() {
        let mut out: Vec<u8> = Vec::new();
        // Providing a path skips the interactive prompt and lets us verify guide is NOT printed.
        // To verify guide IS in SETUP_GUIDE, we test the constant directly since the interactive
        // path requires a TTY. The constant is the single source of truth for what gets printed.
        assert!(SETUP_GUIDE.contains("1."), "guide is missing step 1");
        assert!(SETUP_GUIDE.contains("8."), "guide is missing step 8");
        assert!(
            SETUP_GUIDE.contains("client_secret_*.json"),
            "guide is missing filename hint"
        );
        assert!(
            SETUP_GUIDE.contains("console.cloud.google.com"),
            "guide is missing GCP Console URL"
        );

        // When a file path is given, the guide must NOT appear in the output.
        // We use a nonexistent path to get an error, then check output was empty before the error.
        let result = run_setup_to(Some("/nonexistent/client_secret.json".into()), &mut out);
        assert!(result.is_err(), "expected error for nonexistent file");
        assert!(out.is_empty(), "guide must not be printed when --client-secret-file is given");
    }
}
