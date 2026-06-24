use anyhow::Result;
use reqwest::Url;

use crate::auth::account::KeyringStore;
use crate::auth::client::AuthClient;
use crate::auth::config::load_config;
use crate::cli::DriveCommand;
use crate::drive::DRIVE_SCOPES;

const DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";

pub fn run(cmd: DriveCommand, account: Option<String>) -> Result<()> {
    match cmd {
        DriveCommand::List { limit, all, json: _ } => run_list(limit, all, account),
        DriveCommand::Download { .. } | DriveCommand::Upload { .. } => {
            println!("not yet implemented");
            Ok(())
        }
    }
}

fn run_list(limit: Option<u32>, all: bool, account: Option<String>) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let config = load_config()?;
        let store = KeyringStore;
        let client = AuthClient::from_config(config, &store, account.as_deref())?;
        let mut url = Url::parse(DRIVE_FILES_URL)?;
        url.query_pairs_mut()
            .append_pair("pageSize", &limit.unwrap_or(50).to_string());
        if !all {
            url.query_pairs_mut()
                .append_pair("fields", "files(id,name,mimeType,modifiedTime)");
        }

        let response = client
            .send_with_scopes(client.get(url), DRIVE_SCOPES)
            .await?
            .error_for_status()?;
        println!("{}", response.text().await?);
        Ok::<(), anyhow::Error>(())
    })
}
