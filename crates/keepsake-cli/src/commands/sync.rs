//! `sync` — set up, reveal, rotate, push, pull with a
//! vault-scoped shared server.
//!
//! Usage:
//! ```text
//! keepsake sync setup   --vault ID [--passphrase P]
//! keepsake sync reveal  --vault ID
//! keepsake sync rotate  --vault ID [--passphrase P]
//! keepsake sync delete  --vault ID
//! keepsake sync list
//! keepsake sync push    --server URL --vault ID
//! keepsake sync pull    --server URL --vault ID
//! ```

use keepsake_core::session::Session;
use keepsake_core::sync::client::SyncClient;

use super::with_unlocked_mut;

pub async fn run(
    path: &std::path::Path,
    server: Option<String>,
    vault: Option<String>,
    passphrase: Option<String>,
    sub: String,
    session: &mut Option<Session>,
) -> anyhow::Result<()> {
    let _ = path;
    match sub.as_str() {
        "setup" | "rotate" => {
            let vault_id = require_vault(vault.clone())?;
            let pass = match passphrase {
                Some(p) => p,
                None => dialoguer::Password::new()
                    .with_prompt(format!("{sub} passphrase for '{vault_id}'"))
                    .with_confirmation(format!("confirm passphrase"), "passphrases do not match")
                    .interact()?,
            };
            with_unlocked_mut(session, |sess| -> keepsake_core::Result<()> {
                sess.vault.set_shared_sync(&vault_id, &pass, None)?;
                sess.refresh_shared_sync_keys()?;
                Ok(())
            })?;
            eprintln!("{sub} complete for '{vault_id}'");
            Ok(())
        }
        "reveal" => {
            let vault_id = require_vault(vault.clone())?;
            with_unlocked_mut(session, |sess| -> keepsake_core::Result<()> {
                let setup = sess.vault.get_shared_sync(&vault_id)?
                    .ok_or_else(|| keepsake_core::Error::NotFound(format!(
                        "shared sync '{vault_id}'"
                    )))?;
                println!("vault_id:  {}", setup.vault_id);
                println!("passphrase: {}", setup.passphrase);
                if let Some(r) = setup.rotated_at {
                    println!("rotated_at: {}", r.to_rfc3339());
                }
                Ok(())
            })?;
            Ok(())
        }
        "delete" => {
            let vault_id = require_vault(vault.clone())?;
            with_unlocked_mut(session, |sess| -> keepsake_core::Result<()> {
                sess.vault.delete_shared_sync(&vault_id)?;
                sess.refresh_shared_sync_keys()?;
                Ok(())
            })?;
            eprintln!("deleted shared sync '{vault_id}'");
            Ok(())
        }
        "list" => {
            with_unlocked_mut(session, |sess| -> keepsake_core::Result<()> {
                for v in sess.vault.list_shared_syncs()? {
                    println!("{v}");
                }
                Ok(())
            })?;
            Ok(())
        }
        "push" | "pull" => {
            let server_url = match server {
                Some(s) => s,
                None => dialoguer::Input::new()
                    .with_prompt("server URL")
                    .interact_text()?,
            };
            let vault_id = require_vault(vault.clone())?;
            let client = SyncClient::new(server_url.clone(), vault_id.clone());
            with_unlocked_mut(session, |sess| -> keepsake_core::Result<()> {
                let rt = tokio::runtime::Handle::current();
                match sub.as_str() {
                    "push" => {
                        let n = rt.block_on(client.push(sess))?;
                        eprintln!("pushed {n} records to {server_url} (vault: {vault_id})");
                    }
                    "pull" => {
                        let n = rt.block_on(client.pull(sess))?;
                        eprintln!("pulled {n} records from {server_url} (vault: {vault_id})");
                    }
                    _ => unreachable!(),
                }
                Ok(())
            })?;
            Ok(())
        }
        other => Err(anyhow::anyhow!(
            "unknown sync subcommand: {other} (expected setup, reveal, rotate, delete, list, push, or pull)"
        )),
    }
}

fn require_vault(vault: Option<String>) -> anyhow::Result<String> {
    match vault {
        Some(v) if !v.is_empty() => Ok(v),
        _ => Err(anyhow::anyhow!("--vault is required for this subcommand")),
    }
}
