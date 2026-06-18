//! HTTP API for the sync server.  Pure encrypted blob store —
//! no per-user auth.  Every endpoint is scoped to a `vault_id`
//! path segment; the server has no idea who the caller is,
//! only that a request is well-formed and the URL points at a
//! real vault.
//!
//! Routes:
//!
//! * `GET  /v1/health`              — liveness, no auth, no body
//! * `POST /v1/vaults/:id/sync/push` — body is `Request::Push`
//! * `POST /v1/vaults/:id/sync/pull` — body is `Request::Pull`;
//!                                     returns either the current
//!                                     state (when `since` is
//!                                     empty) or a change feed
//! * `PUT  /v1/vaults/:id/blobs/:sha256` — body is ciphertext
//! * `GET  /v1/vaults/:id/blobs/:sha256` — returns ciphertext

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response as AxumResponse},
    routing::{get, post, put},
    Json, Router,
};
use keepsake_core::sync::protocol::{Request as ProtoRequest, Response as ProtoResponse};

use crate::storage::{StateRow, Storage};

/// App state shared across all routes.
pub struct AppState {
    pub storage: parking_lot::Mutex<Storage>,
}

impl AppState {
    pub fn new(storage: Storage) -> Self {
        Self {
            storage: parking_lot::Mutex::new(storage),
        }
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route(
            "/v1/vaults/:id/sync/push",
            post(push),
        )
        .route(
            "/v1/vaults/:id/sync/pull",
            post(pull),
        )
        .route(
            "/v1/vaults/:id/blobs/:sha256",
            put(put_blob).get(get_blob),
        )
        .with_state(state)
}

// -- error type ------------------------------------------------------------

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: String,
    message: String,
}

impl ApiError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "bad_request".into(),
            message: msg.into(),
        }
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal".into(),
            message: msg.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> AxumResponse {
        let body = ProtoResponse::Error {
            code: self.code,
            message: self.message,
        };
        (self.status, Json(body)).into_response()
    }
}

impl From<crate::storage::Error> for ApiError {
    fn from(e: crate::storage::Error) -> Self {
        use crate::storage::Error::*;
        match e {
            Invalid(m)  => Self::bad_request(m),
            Sqlite(e)   => Self::internal(format!("sqlite: {e}")),
            Io(e)       => Self::internal(format!("io: {e}")),
            other       => Self::internal(format!("{other:?}")),
        }
    }
}

// -- handlers --------------------------------------------------------------

async fn health() -> Json<ProtoResponse> {
    Json(ProtoResponse::Ok { message: Some("ok".into()) })
}

async fn push(
    State(state): State<Arc<AppState>>,
    Path(vault_id): Path<String>,
    body: Bytes,
) -> Result<Json<ProtoResponse>, ApiError> {
    validate_vault_id(&vault_id)?;
    let req: ProtoRequest = serde_json::from_slice(&body)
        .map_err(|e| ApiError::bad_request(format!("body parse: {e}")))?;
    let (n, max_lamport) = match req {
        ProtoRequest::Push { changes, .. } => {
            let n = changes.len();
            let max = changes.iter().map(|c| c.lamport).max().unwrap_or(0);
            state.storage.lock()
                .append_changes(&vault_id, &changes)?;
            (n, max)
        }
        _ => return Err(ApiError::bad_request("expected Request::Push")),
    };
    tracing::info!(vault = %vault_id, n, max_lamport, "push ok");
    Ok(Json(ProtoResponse::Ok { message: None }))
}

async fn pull(
    State(state): State<Arc<AppState>>,
    Path(vault_id): Path<String>,
    body: Bytes,
) -> Result<Json<ProtoResponse>, ApiError> {
    validate_vault_id(&vault_id)?;
    // Empty body = "fresh client, give me state"; otherwise
    // a `Request::Pull` with a `since` vector clock.
    let since_vector_clock: Option<keepsake_core::sync::VectorClock> = if body.is_empty() {
        None
    } else {
        let req: ProtoRequest = serde_json::from_slice(&body)
            .map_err(|e| ApiError::bad_request(format!("body parse: {e}")))?;
        match req {
            ProtoRequest::Pull { since } => Some(since),
            _ => return Err(ApiError::bad_request("expected Request::Pull")),
        }
    };
    let storage = state.storage.lock();
    match since_vector_clock {
        None => {
            // Fresh client: give it the current state.
            let state_rows: Vec<StateRow> = storage.current_state(&vault_id)?;
            // Map into a `Response::PullResp`-shaped payload.  We
            // piggyback on the existing wire type: the changes
            // list contains one synthetic `Change` per current
            // record, with `record_id` set and the ciphertext
            // payload.  The client unwraps these on the other
            // side.
            let synthetic = state_rows.into_iter().map(|r| {
                // Decode record_id from raw bytes to UUID.
                let mut record_id_arr = [0u8; 16];
                if r.record_id.len() != 16 {
                    return Err(ApiError::internal("bad record_id bytes"));
                }
                record_id_arr.copy_from_slice(&r.record_id);
                let record_id = uuid::Uuid::from_bytes(record_id_arr);
                let mut change_id_arr = [0u8; 16];
                if r.change_id.len() != 16 {
                    return Err(ApiError::internal("bad change_id bytes"));
                }
                change_id_arr.copy_from_slice(&r.change_id);
                let id = uuid::Uuid::from_bytes(change_id_arr);
                let ts = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(r.ts_millis)
                    .ok_or_else(|| ApiError::internal("bad ts_millis"))?;
                Ok(keepsake_core::sync::Change {
                    id,
                    lamport: r.lamport,
                    ts,
                    author: r.actor,
                    record_id: Some(record_id),
                    payload: r.payload,
                })
            }).collect::<Result<Vec<_>, ApiError>>()?;
            let clocks = storage.all_clocks(&vault_id)?;
            tracing::info!(vault = %vault_id, n = synthetic.len(), "pull state");
            Ok(Json(ProtoResponse::PullResp {
                changes: synthetic,
                server_clock: keepsake_core::sync::VectorClock { counters: clocks },
            }))
        }
        Some(since) => {
            // A change is "new" for the client if its lamport
            // exceeds any of the client's per-actor clocks (or
            // if the client has no clock for that actor).  This
            // is conservative: a change might be re-delivered
            // if a clock isn't strictly monotonic.
            let since_max: u64 = since.counters.values().copied().max().unwrap_or(0);
            let changes = storage.changes_since(&vault_id, since_max)?;
            let clocks = storage.all_clocks(&vault_id)?;
            tracing::info!(vault = %vault_id, n = changes.len(), since_max, "pull changes");
            Ok(Json(ProtoResponse::PullResp {
                changes,
                server_clock: keepsake_core::sync::VectorClock { counters: clocks },
            }))
        }
    }
}

async fn put_blob(
    State(state): State<Arc<AppState>>,
    Path((vault_id, sha256_hex)): Path<(String, String)>,
    body: Bytes,
) -> Result<Json<ProtoResponse>, ApiError> {
    validate_vault_id(&vault_id)?;
    let sha256 = parse_sha256(&sha256_hex)?;
    state.storage.lock()
        .put_blob(&vault_id, &sha256, &body)?;
    Ok(Json(ProtoResponse::Ok { message: Some("stored".into()) }))
}

async fn get_blob(
    State(state): State<Arc<AppState>>,
    Path((vault_id, sha256_hex)): Path<(String, String)>,
) -> Result<Json<ProtoResponse>, ApiError> {
    validate_vault_id(&vault_id)?;
    let sha256 = parse_sha256(&sha256_hex)?;
    let storage = state.storage.lock();
    let ciphertext = storage.get_blob(&vault_id, &sha256)?
        .ok_or_else(|| ApiError::bad_request("blob not found"))?;
    Ok(Json(ProtoResponse::BlobResp { ciphertext }))
}

// -- helpers ---------------------------------------------------------------

/// Vault ids are url path segments.  Constrain to a sensible
/// charset so we don't have to worry about path traversal or
/// control characters in SQL.
fn validate_vault_id(id: &str) -> Result<(), ApiError> {
    if id.is_empty() || id.len() > 64 {
        return Err(ApiError::bad_request("vault_id must be 1..=64 chars"));
    }
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(ApiError::bad_request(
            "vault_id must match [A-Za-z0-9_-]+",
        ));
    }
    Ok(())
}

fn parse_sha256(hex_str: &str) -> Result<[u8; 32], ApiError> {
    let bytes = hex::decode(hex_str)
        .map_err(|_| ApiError::bad_request("sha256 not hex"))?;
    bytes.try_into()
        .map_err(|_| ApiError::bad_request("sha256 not 32 bytes"))
}
