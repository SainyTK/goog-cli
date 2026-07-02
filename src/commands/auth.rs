use std::io::Write;

use anyhow::{Context, Result};
use dialoguer::{Input, Password};

use crate::auth::account::{AccountStore, FileAccountStore, KeyringStore, TokenSaveOutcome};
use crate::auth::config::{
    config_path, load_config, resolve_account_selector, save_config, switch_active_account, Config,
    OAuthAppConfig, OAuthAppType, SettingsConfig,
};
use crate::auth::error::AuthError;
use crate::auth::list::{render_ndjson, render_table, rows_from_config};
use crate::auth::login::{
    build_authorize_url, exchange_code, fetch_email, poll_device_token, random_state,
    render_device_authorization_prompt, request_device_authorization, LoopbackServer,
    DEFAULT_DEVICE_LOGIN_SCOPES, DEFAULT_LOGIN_SCOPES, GOOGLE_AUTH_URL, GOOGLE_DEVICE_CODE_URL,
    GOOGLE_TOKEN_URL, GOOGLE_USERINFO_URL,
};
use crate::auth::setup::{parse_client_secret_file, OAuthAppSecrets};
use crate::auth::state::{
    load_runtime_state, load_runtime_state_from_path, resource_key, save_runtime_state,
    save_runtime_state_to_path, RuntimeState,
};
use crate::cli::{AuthCommand, AuthMappingsCommand};

const DEVICE_OAUTH_CLIENT_TYPE: &str = "TVs and Limited Input devices";
const DEVICE_OAUTH_SETUP_COMMAND: &str =
    "`goog auth setup --client-secret-file <path> --app-type device`";

pub(super) const SETUP_GUIDE: &str = "\
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
     - If you need `goog auth login --no-browser`, choose
       \"TVs and Limited Input devices\" instead and pass
       `--app-type device` to setup.
  8. Click \"Create\", then copy the client ID and client secret.

Enter those values below.
";

pub fn run(cmd: AuthCommand, resolved_account: Option<String>) -> Result<()> {
    match cmd {
        AuthCommand::Setup {
            client_secret_file,
            app_type,
        } => run_setup(client_secret_file, app_type),
        AuthCommand::Login { no_browser } => run_login(no_browser),
        AuthCommand::List { json } => run_list(json, resolved_account),
        AuthCommand::Switch { email } => run_switch(email),
        AuthCommand::Export { email, out } => run_export(email.as_deref(), &out),
        AuthCommand::Mappings { command } => run_mappings(command),
    }
}

fn run_setup(client_secret_file: Option<String>, app_type: Option<OAuthAppType>) -> Result<()> {
    run_setup_to(client_secret_file, app_type, &mut std::io::stdout())
}

pub(super) fn run_setup_to(
    client_secret_file: Option<String>,
    app_type: Option<OAuthAppType>,
    out: &mut impl std::io::Write,
) -> Result<()> {
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
        app_type: app_type.unwrap_or(secrets.app_type),
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

pub(super) fn build_oauth_app_secrets(
    client_id: String,
    client_secret: String,
) -> Result<OAuthAppSecrets> {
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
        app_type: OAuthAppType::Desktop,
    })
}

fn run_login(no_browser: bool) -> Result<()> {
    let mut config = load_config().context("failed to load config")?;
    let oauth_app = config
        .oauth_app
        .clone()
        .ok_or(AuthError::OAuthAppNotConfigured)?;

    let store = KeyringStore;
    let login = perform_login(&oauth_app, &store, no_browser)?;

    add_account_to_config(&mut config, &login.email);
    save_config(&config).context("failed to save config")?;

    write_login_completion_to(
        &login.email,
        &login.token_save,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    )?;
    Ok(())
}

#[derive(Debug)]
pub(super) struct LoginOutcome {
    email: String,
    token_save: TokenSaveOutcome,
}

fn perform_login(
    oauth_app: &OAuthAppConfig,
    store: &impl AccountStore,
    no_browser: bool,
) -> Result<LoginOutcome> {
    if no_browser {
        return perform_device_login(oauth_app, store);
    }

    perform_loopback_login(oauth_app, store)
}

fn perform_loopback_login(
    oauth_app: &OAuthAppConfig,
    store: &impl AccountStore,
) -> Result<LoginOutcome> {
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

    let token_save = store
        .save_token_for_login(&email, &token)
        .context("failed to save token to keychain")?;
    Ok(LoginOutcome { email, token_save })
}

pub(super) fn perform_device_login(
    oauth_app: &OAuthAppConfig,
    store: &impl AccountStore,
) -> Result<LoginOutcome> {
    require_device_oauth_app(oauth_app)?;

    let runtime = tokio::runtime::Runtime::new().context("failed to start async runtime")?;
    let (token, email) = runtime.block_on(async {
        let authorization = request_device_authorization(
            GOOGLE_DEVICE_CODE_URL,
            &oauth_app.client_id,
            DEFAULT_DEVICE_LOGIN_SCOPES,
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

    let token_save = store
        .save_token_for_login(&email, &token)
        .context("failed to save token to keychain")?;
    Ok(LoginOutcome { email, token_save })
}

pub(super) fn write_login_completion_to(
    email: &str,
    token_save: &TokenSaveOutcome,
    out: &mut impl std::io::Write,
    err: &mut impl std::io::Write,
) -> Result<()> {
    if !token_save.prompt_free_access_is_guaranteed() {
        writeln!(
            err,
            "Warning: saved Token, but goog could not guarantee prompt-free Keychain access. \
You may still see Keychain Access Prompts from macOS when goog reads this Token; these are \
separate from Google browser consent prompts for new Scopes. To repair local Keychain access, \
rerun `goog auth login` for this Account."
        )
        .context("failed to write keychain warning")?;
    }

    writeln!(out, "Authorized as {email}").context("failed to write output")?;
    Ok(())
}

fn require_device_oauth_app(oauth_app: &OAuthAppConfig) -> Result<()> {
    if oauth_app.app_type == OAuthAppType::Device {
        return Ok(());
    }

    Err(AuthError::OAuthFlow(format!(
        "device login requires an OAuth client of type \"{DEVICE_OAUTH_CLIENT_TYPE}\". Run {DEVICE_OAUTH_SETUP_COMMAND} with that client."
    ))
    .into())
}

pub(super) fn add_account_to_config(config: &mut Config, email: &str) {
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

fn run_export(email: Option<&str>, out: &str) -> Result<()> {
    run_export_to(email, out, &mut std::io::stdout())
}

fn run_export_to(email: Option<&str>, out_path: &str, out: &mut impl std::io::Write) -> Result<()> {
    let config = load_config().context("failed to load config")?;

    let emails = match email {
        Some(selector) => vec![resolve_account_selector(&config, selector)?],
        None => config.accounts.clone(),
    };

    if emails.is_empty() {
        anyhow::bail!("no authorized accounts to export -- run `goog auth login` first");
    }

    let keychain = KeyringStore;
    let mut tokens = std::collections::HashMap::new();
    for account_email in &emails {
        let token = keychain
            .load_token(account_email)
            .context("failed to read token from keychain")?
            .ok_or_else(|| AuthError::TokenNotFound {
                email: account_email.clone(),
            })?;
        tokens.insert(account_email.clone(), token);
    }

    let file_store = FileAccountStore::new(std::path::PathBuf::from(out_path));
    file_store
        .replace_all(&tokens)
        .with_context(|| format!("failed to write token file to {out_path}"))?;

    writeln!(
        out,
        "Exported {} account(s) to {out_path}: {}",
        emails.len(),
        emails.join(", ")
    )
    .context("failed to write output")?;
    writeln!(
        out,
        "This file grants full access to those accounts within their authorized scopes. \
         Keep it out of git, mount it read-only wherever it's used, and delete it when done."
    )
    .context("failed to write output")?;
    Ok(())
}

fn run_mappings(command: AuthMappingsCommand) -> Result<()> {
    match command {
        AuthMappingsCommand::List { json } => {
            run_mappings_list_to(json, &mut std::io::stdout(), None)
        }
        AuthMappingsCommand::Clear {
            surface,
            resource_id,
        } => run_mappings_clear_to(
            surface.as_deref(),
            resource_id.as_deref(),
            &mut std::io::stdout(),
            None,
        ),
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub(super) struct ResourceAccountMappingRow {
    pub surface: String,
    pub resource_id: String,
    pub account: String,
    pub resource_key: String,
}

pub(super) fn run_mappings_list_to(
    json: bool,
    out: &mut impl std::io::Write,
    state_path: Option<&std::path::Path>,
) -> Result<()> {
    let state = load_mappings_runtime_state(state_path)?;
    let rows = resource_account_mapping_rows(&state);
    let rendered = if json {
        render_mapping_ndjson(&rows)
    } else {
        render_mapping_table(&rows)
    };

    out.write_all(rendered.as_bytes())
        .context("failed to write output")?;
    Ok(())
}

pub(super) fn run_mappings_clear_to(
    surface: Option<&str>,
    resource_id: Option<&str>,
    out: &mut impl std::io::Write,
    state_path: Option<&std::path::Path>,
) -> Result<()> {
    let mut state = load_mappings_runtime_state(state_path)?;
    let before = state.resource_account_mappings.len();

    match (surface, resource_id) {
        (None, None) => state.resource_account_mappings.clear(),
        (Some(surface), Some(resource_id)) => {
            state
                .resource_account_mappings
                .remove(&resource_key(surface, resource_id));
        }
        _ => {
            anyhow::bail!(
                "pass both --surface and --resource-id to clear one Resource Account Mapping, or omit both to clear all mappings"
            );
        }
    }

    let cleared = before.saturating_sub(state.resource_account_mappings.len());
    save_mappings_runtime_state(&state, state_path)?;
    writeln!(out, "Cleared {cleared} Resource Account Mapping(s).")
        .context("failed to write output")?;
    Ok(())
}

fn resource_account_mapping_rows(state: &RuntimeState) -> Vec<ResourceAccountMappingRow> {
    let mut rows: Vec<_> = state
        .resource_account_mappings
        .iter()
        .map(|(key, account)| {
            let (surface, resource_id) = split_resource_key(key);
            ResourceAccountMappingRow {
                surface: surface.to_string(),
                resource_id: resource_id.to_string(),
                account: account.clone(),
                resource_key: key.clone(),
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        a.surface
            .cmp(&b.surface)
            .then(a.resource_id.cmp(&b.resource_id))
            .then(a.account.cmp(&b.account))
    });
    rows
}

fn split_resource_key(key: &str) -> (&str, &str) {
    key.split_once(':').unwrap_or((key, ""))
}

fn render_mapping_table(rows: &[ResourceAccountMappingRow]) -> String {
    if rows.is_empty() {
        return "No Resource Account Mappings remembered.\n".to_string();
    }

    let surface_width = rows
        .iter()
        .map(|row| row.surface.len())
        .max()
        .unwrap_or(0)
        .max("SURFACE".len());
    let resource_width = rows
        .iter()
        .map(|row| row.resource_id.len())
        .max()
        .unwrap_or(0)
        .max("RESOURCE ID".len());
    let account_width = rows
        .iter()
        .map(|row| row.account.len())
        .max()
        .unwrap_or(0)
        .max("ACCOUNT".len());

    let mut out = String::new();
    out.push_str(&format!(
        "{:<surface_width$}  {:<resource_width$}  {:<account_width$}\n",
        "SURFACE", "RESOURCE ID", "ACCOUNT",
    ));
    for row in rows {
        out.push_str(&format!(
            "{:<surface_width$}  {:<resource_width$}  {:<account_width$}\n",
            row.surface, row.resource_id, row.account,
        ));
    }
    out
}

fn render_mapping_ndjson(rows: &[ResourceAccountMappingRow]) -> String {
    let mut out = String::new();
    for row in rows {
        out.push_str(
            &serde_json::to_string(row).expect("ResourceAccountMappingRow is always serializable"),
        );
        out.push('\n');
    }
    out
}

fn load_mappings_runtime_state(state_path: Option<&std::path::Path>) -> Result<RuntimeState> {
    match state_path {
        Some(path) => load_runtime_state_from_path(path),
        None => load_runtime_state(),
    }
    .context("failed to load runtime state")
}

fn save_mappings_runtime_state(
    state: &RuntimeState,
    state_path: Option<&std::path::Path>,
) -> Result<()> {
    match state_path {
        Some(path) => save_runtime_state_to_path(state, path),
        None => save_runtime_state(state),
    }
    .context("failed to save runtime state")
}
