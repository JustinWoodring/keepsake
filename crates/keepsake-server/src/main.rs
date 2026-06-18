//! `keepsake-server` — pure encrypted blob store.  No auth,
//! no per-user state.  Vaults are addressable by URL path
//! segment; knowing the URL + vault id is the only access
//! control.
//!
//! Configuration (env vars):
//!
//! * `KEEPSAKE_ADDR`     — listen address (default `127.0.0.1:8484`)
//! * `KEEPSAKE_DB`       — SQLite path (default `./keepsake-server.db`)
//!
//! TLS is not handled by this binary.  Put nginx/caddy in
//! front for TLS termination.  See `docs/deploy-sync-server.md`.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use keepsake_server::api::{router, AppState};
use keepsake_server::storage::Storage;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("keepsake_server=info,tower_http=info")))
        .init();

    let addr: SocketAddr = std::env::var("KEEPSAKE_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8484".to_string())
        .parse()?;
    let db_path: PathBuf = std::env::var("KEEPSAKE_DB")
        .unwrap_or_else(|_| "./keepsake-server.db".to_string())
        .into();

    let storage = Storage::open(&db_path)?;
    let state = Arc::new(AppState::new(storage));

    let app = router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(
        "keepsake-server listening on http://{addr} (db: {})",
        db_path.display()
    );
    axum::serve(listener, app).await?;
    Ok(())
}
