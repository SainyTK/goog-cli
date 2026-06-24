use std::io::Write;

use anyhow::{Context, Result};
use dialoguer::{Input, Password};

use crate::auth::account::{AccountStore, KeyringStore};
use crate::auth::config::{
    config_path, load_config, save_config, switch_active_account, Config, OAuthAppConfig,
    SettingsConfig,
};
use crate::auth::error::AuthError;
use crate::auth::list::{render_ndjson, render_table, rows_from_config};
use crate::auth::login::{
    build_authorize_url, exchange_code, fetch_email, poll_device_token, random_state,
    render_device_authorization_prompt, request_device_authorization, LoopbackServer,
    DEFAULT_LOGIN_SCOPES, GOOGLE_AUTH_URL, GOOGLE_DEVICE_CODE_URL, GOOGLE_TOKEN_URL,
    GOOGLE_USERINFO_URL,
};
use crate::auth::setup::{parse_client_secret_file, OAuthAppSecrets};
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
  8. Click \"Create\", then copy the client ID and client secret.

Enter those values below.
";

pub fn run(cmd: AuthCommand, resolved_account: Option<String>) -> Result<()> {
    match cmd {
        AuthCommand::Setup { client_secret_file } => run_setup(client_secret_file),
        AuthCommand::Login { no_browser } => run_login(no_browser),
        AuthCommand::List { json } => run_list(json, resolved_account),
        AuthCommand::Switch { email } => run_switch(email),
    }
}

fn run_setup(client_secret_file: Option<String>) -> Result<()> {
    run_setup_to(client_secret_file, &mut std::io::stdout())
}

fn run_setup_to(client_secret_file: Option<String>, out: &mut impl std::io::Write) -> Result<()> {
    let secrets = match client_secret_file {
        Some(path) => parse_client_secret_file(&path)
            .with_context(|| format!("failed to load OAuth App from {path}"))?,
        None => {
            write!(out, "{SETUP_GUIDE}").context("failed to write setup guide")?;
            prompt_for_oauth_app()?
        }
    };

    let mut config = load_config().context("failed to load config")?;
    config.oauth_app = Some(OAuthAppConfig {
        client_id: secrets.client_id,
        client_secret: secrets.client_secret,
    });
    save_config(&config).context("failed to save config")?;

    let saved_to = config_path().context("could not determine config path")?;
    writeln!(out, "OAuth App saved to {}", saved_to.display()).context("failed to write output")?;

    Ok(())
}

fn prompt_for_oauth_app() -> Result<OAuthAppSecrets> {
    let client_id = Input::new()
        .with_prompt("OAuth client ID")
        .interact_text()
        .context("failed to read OAuth client ID from stdin")?;

    let client_secret = Password::new()
        .with_prompt("OAuth client secret")
        .interact()
        .context("failed to read OAuth client secret from stdin")?;

    build_oauth_app_secrets(client_id, client_secret)
}

fn build_oauth_app_secrets(client_id: String, client_secret: String) -> Result<OAuthAppSecrets> {
    let client_id = client_id.trim().to_string();
    let client_secret = client_secret.trim().to_string();

    if client_id.is_empty() {
        return Err(AuthError::OAuthAppMissingField {
            field: "client_id".into(),
        }
        .into());
    }

    if client_secret.is_empty() {
        return Err(AuthError::OAuthAppMissingField {
            field: "client_secret".into(),
        }
        .into());
    }

    Ok(OAuthAppSecrets {
        client_id,
        client_secret,
    })
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
    if no_browser {
        return perform_device_login(oauth_app, store);
    }

    perform_loopback_login(oauth_app, store)
}

fn perform_loopback_login(oauth_app: &OAuthAppConfig, store: &impl AccountStore) -> Result<String> {
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

    println!("Opening browser for Google sign-in...");
    if webbrowser::open(&url).is_err() {
        println!("Could not open a browser automatically. Open this URL manually:\n  {url}");
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

fn perform_device_login(oauth_app: &OAuthAppConfig, store: &impl AccountStore) -> Result<String> {
    let runtime = tokio::runtime::Runtime::new().context("failed to start async runtime")?;
    let (token, email) = runtime.block_on(async {
        let authorization = request_device_authorization(
            GOOGLE_DEVICE_CODE_URL,
            &oauth_app.client_id,
            DEFAULT_LOGIN_SCOPES,
        )
        .await?;

        print!("{}", render_device_authorization_prompt(&authorization));
        std::io::stdout().flush().map_err(|e| {
            AuthError::OAuthFlow(format!("failed to flush device authorization prompt: {e}"))
        })?;

        let token = poll_device_token(
            GOOGLE_TOKEN_URL,
            &oauth_app.client_id,
            &oauth_app.client_secret,
            &authorization.device_code,
            authorization.interval,
            authorization.expires_in,
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

fn run_list(json: bool, active_account: Option<String>) -> Result<()> {
    run_list_to(json, active_account.as_deref(), &mut std::io::stdout())
}

fn run_list_to(
    json: bool,
    active_account: Option<&str>,
    out: &mut impl std::io::Write,
) -> Result<()> {
    let config = load_config().context("failed to load config")?;
    let active = active_account.or_else(|| config.active_account());
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

fn run_switch(email: String) -> Result<()> {
    run_switch_to(&email, &mut std::io::stdout())
}

fn run_switch_to(email: &str, out: &mut impl std::io::Write) -> Result<()> {
    let mut config = load_config().context("failed to load config")?;
    let active_account = switch_active_account(&mut config, email)?;
    save_config(&config).context("failed to save config")?;
    writeln!(out, "Active Account switched to {active_account}")
        .context("failed to write output")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_guide_describes_direct_client_id_and_secret_entry() {
        let mut out: Vec<u8> = Vec::new();
        assert!(SETUP_GUIDE.contains("1."), "guide is missing step 1");
        assert!(SETUP_GUIDE.contains("8."), "guide is missing step 8");
        assert!(
            SETUP_GUIDE.contains("client ID and client secret"),
            "guide is missing direct entry hint"
        );
        assert!(
            SETUP_GUIDE.contains("Desktop app"),
            "guide is missing Desktop app hint"
        );
        assert!(
            SETUP_GUIDE.contains("console.cloud.google.com"),
            "guide is missing GCP Console URL"
        );

        let result = run_setup_to(Some("/nonexistent/client_secret.json".into()), &mut out);
        assert!(result.is_err(), "expected error for nonexistent file");
        assert!(
            out.is_empty(),
            "guide must not be printed when --client-secret-file is given"
        );
    }

    #[test]
    fn build_oauth_app_secrets_trims_values() {
        let secrets = build_oauth_app_secrets("  id123  ".into(), "  sec456  ".into()).unwrap();

        assert_eq!(secrets.client_id, "id123");
        assert_eq!(secrets.client_secret, "sec456");
    }

    #[test]
    fn build_oauth_app_secrets_rejects_blank_client_id() {
        let err = build_oauth_app_secrets("  ".into(), "sec456".into()).unwrap_err();

        assert!(
            matches!(&err.downcast_ref::<AuthError>(), Some(AuthError::OAuthAppMissingField { field }) if field == "client_id")
        );
    }

    #[test]
    fn build_oauth_app_secrets_rejects_blank_client_secret() {
        let err = build_oauth_app_secrets("id123".into(), "  ".into()).unwrap_err();

        assert!(
            matches!(&err.downcast_ref::<AuthError>(), Some(AuthError::OAuthAppMissingField { field }) if field == "client_secret")
        );
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
            vec![
                "alice@example.com".to_string(),
                "bob@example.com".to_string()
            ]
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
