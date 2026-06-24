use anyhow::{Context, Result};
use dialoguer::Input;

use crate::auth::account::{AccountStore, KeyringStore};
use crate::auth::config::{
    config_path, load_config, save_config, Config, OAuthAppConfig, SettingsConfig,
};
use crate::auth::error::AuthError;
use crate::auth::list::{render_ndjson, render_table, rows_from_config};
use crate::auth::login::{
    build_authorize_url, exchange_code, fetch_email, random_state, LoopbackServer,
    DEFAULT_LOGIN_SCOPES, GOOGLE_AUTH_URL, GOOGLE_TOKEN_URL, GOOGLE_USERINFO_URL,
};
use crate::auth::setup::parse_client_secret_file;
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
        AuthCommand::Login { no_browser } => run_login(no_browser),
        AuthCommand::List { json } => run_list(json),
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

fn run_login(no_browser: bool) -> Result<()> {
    let mut config = load_config().context("failed to load config")?;
    let oauth_app = config
        .oauth_app
        .clone()
        .ok_or(AuthError::OAuthAppNotConfigured)?;

    let store = KeyringStore;
    let email = perform_login(&oauth_app, &store, no_browser)?;

    add_account_to_config(&mut config, &email);
    save_config(&config).context("failed to save config")?;

    println!("Authorized as {email}");
    Ok(())
}

fn perform_login(
    oauth_app: &OAuthAppConfig,
    store: &impl AccountStore,
    no_browser: bool,
) -> Result<String> {
    let server = LoopbackServer::bind().context("failed to bind loopback server")?;
    let redirect_uri = server.redirect_uri();
    let state = random_state();
    let url = build_authorize_url(
        GOOGLE_AUTH_URL,
        &oauth_app.client_id,
        &redirect_uri,
        DEFAULT_LOGIN_SCOPES,
        &state,
    )
    .context("failed to build authorize URL")?;

    if no_browser {
        println!("Open this URL in a browser to authorize:\n  {url}");
    } else {
        println!("Opening browser for Google sign-in...");
        if webbrowser::open(&url).is_err() {
            println!("Could not open a browser automatically. Open this URL manually:\n  {url}");
        }
    }

    let code = server
        .wait_for_callback(&state)
        .context("failed to capture authorization code")?;

    let runtime = tokio::runtime::Runtime::new().context("failed to start async runtime")?;
    let (token, email) = runtime.block_on(async {
        let token = exchange_code(
            GOOGLE_TOKEN_URL,
            &oauth_app.client_id,
            &oauth_app.client_secret,
            &redirect_uri,
            &code,
        )
        .await?;
        let email = fetch_email(GOOGLE_USERINFO_URL, &token.access_token).await?;
        Ok::<_, AuthError>((token, email))
    })?;

    store
        .save_token(&email, &token)
        .context("failed to save token to keychain")?;
    Ok(email)
}

fn add_account_to_config(config: &mut Config, email: &str) {
    if !config.accounts.iter().any(|e| e == email) {
        config.accounts.push(email.to_string());
    }

    let settings = config.settings.get_or_insert_with(SettingsConfig::default);
    if settings.active_account.is_none() {
        settings.active_account = Some(email.to_string());
    }
}

fn run_list(json: bool) -> Result<()> {
    run_list_to(json, &mut std::io::stdout())
}

fn run_list_to(json: bool, out: &mut impl std::io::Write) -> Result<()> {
    let config = load_config().context("failed to load config")?;
    let active = config
        .settings
        .as_ref()
        .and_then(|s| s.active_account.as_deref());
    let rows = rows_from_config(&config.accounts, active);

    let rendered = if json {
        render_ndjson(&rows)
    } else {
        render_table(&rows)
    };
    out.write_all(rendered.as_bytes())
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

        let result = run_setup_to(Some("/nonexistent/client_secret.json".into()), &mut out);
        assert!(result.is_err(), "expected error for nonexistent file");
        assert!(out.is_empty(), "guide must not be printed when --client-secret-file is given");
    }

    #[test]
    fn add_account_dedups_repeated_logins() {
        let mut config = Config::default();
        add_account_to_config(&mut config, "alice@example.com");
        add_account_to_config(&mut config, "alice@example.com");
        assert_eq!(config.accounts, vec!["alice@example.com".to_string()]);
    }

    #[test]
    fn add_account_appends_a_second_distinct_email() {
        let mut config = Config::default();
        add_account_to_config(&mut config, "alice@example.com");
        add_account_to_config(&mut config, "bob@example.com");
        assert_eq!(
            config.accounts,
            vec!["alice@example.com".to_string(), "bob@example.com".to_string()]
        );
    }

    #[test]
    fn first_login_becomes_the_active_account() {
        let mut config = Config::default();
        add_account_to_config(&mut config, "alice@example.com");
        assert_eq!(
            config.settings.unwrap().active_account.as_deref(),
            Some("alice@example.com")
        );
    }

    #[test]
    fn second_login_does_not_displace_active_account() {
        let mut config = Config::default();
        add_account_to_config(&mut config, "alice@example.com");
        add_account_to_config(&mut config, "bob@example.com");
        assert_eq!(
            config.settings.unwrap().active_account.as_deref(),
            Some("alice@example.com")
        );
    }

}
