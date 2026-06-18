//! `sync` — push/pull changes with the configured server.
//!
//! Usage:
//! ```text
//! keepsake sync push        # push local records to server
//! keepsake sync pull        # pull remote records and apply locally
//! keepsake sync register    # register this user with the server
//! ```

use keepsake_core::session::Session;
use keepsake_core::sync::client::SyncClient;

use super::with_unlocked_mut;

pub async fn run(
    path: &std::path::Path,
    server: Option<String>,
    sub: String,
    session: &mut Option<Session>,
) -> anyhow::Result<()> {
    let server_url = match server {
        Some(s) => s,
        None => dialoguer::Input::new()
            .with_prompt("server URL")
            .interact_text()?,
    };

    let client = SyncClient::new(server_url.clone());

    with_unlocked_mut(session, |sess| -> keepsake_core::Result<()> {
        let rt = tokio::runtime::Handle::current();
        match sub.as_str() {
            "register" => {
                rt.block_on(client.register(sess))?;
                eprintln!("registered {server_url}");
            }
            "push" => {
                let n = rt.block_on(client.push(sess))?;
                eprintln!("pushed {n} records to {server_url}");
            }
            "pull" => {
                let n = rt.block_on(client.pull(sess))?;
                eprintln!("pulled {n} changes from {server_url}");
            }
            other => {
                return Err(keepsake_core::Error::Invalid(format!(
                    "unknown sync subcommand: {other} (expected push, pull, or register)"
                )));
            }
        }
        let _ = path;
        Ok(())
    })?;
    Ok(())
}
