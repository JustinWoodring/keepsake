//! `status` — print whether the vault is unlocked and the
//! current username, as JSON.

use keepsake_core::session::Session;

pub async fn run(session: &Option<Session>) -> anyhow::Result<()> {
    let payload = match session {
        Some(s) => serde_json::json!({
            "unlocked": true,
            "username": s.username,
        }),
        None => serde_json::json!({
            "unlocked": false,
            "username": null,
        }),
    };
    println!("{}", serde_json::to_string(&payload)?);
    Ok(())
}
