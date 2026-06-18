//! `sync` — push/pull changes with a vault-scoped server.
//!
//! Usage:
//! ```text
//! keepsake sync push   --server URL --vault ID
//! keepsake sync pull   --server URL --vault ID
//! ```

use keepsake_core::session::Session;
use keepsake_core::sync::client::SyncClient;

use super::with_unlocked_mut;

pub async fn run(
    path: &std::path::Path,
    server: Option<String>,
    vault: Option<String>,
    sub: String,
    session: &mut Option<Session>,
) -> anyhow::Result<()> {
    let server_url = match server {
        Some(s) => s,
        None => dialoguer::Input::new()
            .with_prompt("server URL")
            .interact_text()?,
    };
    let vault_id = match vault {
        Some(v) => v,
        None => dialoguer::Input::new()
            .with_prompt("vault id")
            .default("personal".into())
            .interact_text()?,
    };

    let client = SyncClient::new(server_url.clone(), vault_id.clone());

    with_unlocked_mut(session, |sess| -> keepsake_core::Result<()> {
        let key = keepsake_core::sync::client::derive_personal_vault_key(&sess.master)?;
        let rt = tokio::runtime::Handle::current();
        match sub.as_str() {
            "push" => {
                let n = rt.block_on(client.push(&key, sess))?;
                eprintln!("pushed {n} records to {server_url} (vault: {vault_id})");
            }
            "pull" => {
                let n = rt.block_on(client.pull(&key, sess))?;
                eprintln!("pulled {n} records from {server_url} (vault: {vault_id})");
            }
            other => {
                return Err(keepsake_core::Error::Invalid(format!(
                    "unknown sync subcommand: {other} (expected push or pull)"
                )));
            }
        }
        let _ = path;
        Ok(())
    })?;
    Ok(())
}
