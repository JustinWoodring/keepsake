//! In-process dev sync server.  Boots an axum instance bound to
//! 127.0.0.1:8484 backed by a temp directory.  Used for local
//! sync testing without a remote deployment.
//!
//! This is intentionally minimal in v1: it just confirms a
//! request round-trips.  Real sync semantics are added with the
//! standalone server crate.

use std::net::SocketAddr;
use std::path::Path;

use axum::{routing::post, Json, Router};

use keepsake_core::sync::protocol::{Request, Response};

pub fn run(vault_path: &Path) -> anyhow::Result<()> {
    let _ = vault_path;
    let app = Router::new().route("/v1/sync", post(echo));
    let addr: SocketAddr = "127.0.0.1:8484".parse()?;
    eprintln!("dev server listening on http://{addr}");
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    rt.block_on(async move {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok::<(), anyhow::Error>(())
    })
}

async fn echo(Json(req): Json<Request>) -> Json<Response> {
    Json(Response::Ok {
        message: Some(format!("echo: {req:?}")),
    })
}
