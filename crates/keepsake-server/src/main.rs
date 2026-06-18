//! `keepsake-server` — standalone axum server that stores opaque
//! ciphertext for sync.  Authenticates users via Ed25519
//! envelope signatures and short-lived bearer tokens.
//!
//! Configuration (env vars):
//!
//! * `KEEPSAKE_ADDR`     — listen address (default `127.0.0.1:8484`)
//! * `KEEPSAKE_DB`       — SQLite path (default `./keepsake-server.db`)
//! * `KEEPSAKE_TLS`      — set to `1` to require TLS at the
//!                          load-balancer layer (recommended in
//!                          production; we don't terminate TLS
//!                          here, nginx/caddy should).
//!
//! See `docs/sync-protocol.md` for the wire spec.

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
    tracing::info!("keepsake-server listening on http://{addr} (db: {})", db_path.display());

    if std::env::var("KEEPSAKE_TLS").is_ok() {
        tracing::warn!(
            "KEEPSAKE_TLS is set, but the server does not terminate TLS itself. \
             Put nginx/caddy in front for TLS termination."
        );
    }

    axum::serve(listener, app).await?;
    Ok(())
}
