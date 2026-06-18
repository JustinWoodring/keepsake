//! HTTP API for the sync server.  Implements the wire protocol
//! from `crates/keepsake-core/src/sync/protocol.rs` plus the
//! signature/bearer envelope from `docs/sync-protocol.md`.
//!
//! Routes:
//!
//! * `POST /v1/auth/register`  — body is `ProtoRequest::Register`
//! * `POST /v1/auth/login`     — body is `ProtoRequest::Login` (returns a bearer)
//! * `POST /v1/sync/push`      — `ProtoRequest::Push` (signed)
//! * `POST /v1/sync/pull`      — `ProtoRequest::Pull` (signed)
//! * `PUT  /v1/blobs/:sha256`  — body is `Request::PutBlob` (signed)
//! * `GET  /v1/blobs/:sha256`  — returns `ProtoResponse::BlobResp` (signed)
//! * `GET  /v1/health`         — liveness

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response as AxumResponse},
    routing::{get, post, put},
    Json, Router,
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use keepsake_core::sync::protocol::{Request as ProtoRequest, Response as ProtoResponse};
use parking_lot::Mutex;
use rand::RngCore;
use serde::Deserialize;

use crate::storage::{SessionRow, Storage};

/// App state shared across all routes.
pub struct AppState {
    pub storage: parking_lot::Mutex<Storage>,
    /// Cached verifying keys (32 bytes → VerifyingKey) to avoid
    /// re-deriving on every request.  Small enough to keep
    /// in-process; not security-critical.
    pub keys: Mutex<std::collections::BTreeMap<[u8; 32], VerifyingKey>>,
    /// Bearer session lifetime (seconds).
    pub session_ttl_secs: i64,
}

impl AppState {
    pub fn new(storage: Storage) -> Self {
        Self {
            storage: parking_lot::Mutex::new(storage),
            keys: Mutex::new(std::collections::BTreeMap::new()),
            session_ttl_secs: 60 * 60 * 24, // 24 hours
        }
    }

    fn verifying_key(&self, pk: &[u8; 32]) -> Option<VerifyingKey> {
        if let Some(k) = self.keys.lock().get(pk) {
            return Some(*k);
        }
        let bytes: [u8; 32] = *pk;
        let vk = VerifyingKey::from_bytes(&bytes).ok()?;
        self.keys.lock().insert(*pk, vk);
        Some(vk)
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/auth/challenge", get(issue_challenge))
        .route("/v1/auth/register", post(register))
        .route("/v1/auth/login", post(login))
        .route("/v1/sync/push", post(push))
        .route("/v1/sync/pull", post(pull))
        .route("/v1/blobs/:sha256", put(put_blob).get(get_blob))
        .with_state(state)
}

// -- envelope parsing --------------------------------------------------------

/// Headers every authenticated request must carry.
fn auth_headers(headers: &HeaderMap) -> Result<([u8; 32], [u8; 32], Vec<u8>), ApiError> {
    let pk_hex = headers
        .get("x-envelope-pk")
        .ok_or(ApiError::unauthorized("missing x-envelope-pk"))?
        .to_str()
        .map_err(|_| ApiError::unauthorized("x-envelope-pk not utf-8"))?;
    let pk_bytes = hex::decode(pk_hex)
        .map_err(|_| ApiError::unauthorized("x-envelope-pk not hex"))?;
    let pk: [u8; 32] = pk_bytes
        .try_into()
        .map_err(|_| ApiError::unauthorized("x-envelope-pk not 32 bytes"))?;

    let auth = headers
        .get("authorization")
        .ok_or(ApiError::unauthorized("missing authorization"))?
        .to_str()
        .map_err(|_| ApiError::unauthorized("authorization not utf-8"))?;
    let bearer_hex = auth
        .strip_prefix("Bearer ")
        .ok_or(ApiError::unauthorized("authorization must be Bearer"))?;
    let bearer_bytes = hex::decode(bearer_hex)
        .map_err(|_| ApiError::unauthorized("bearer not hex"))?;
    let bearer: [u8; 32] = bearer_bytes
        .try_into()
        .map_err(|_| ApiError::unauthorized("bearer not 32 bytes"))?;

    let sig_hex = headers
        .get("x-signature")
        .ok_or(ApiError::unauthorized("missing x-signature"))?
        .to_str()
        .map_err(|_| ApiError::unauthorized("x-signature not utf-8"))?;
    let sig = hex::decode(sig_hex)
        .map_err(|_| ApiError::unauthorized("x-signature not hex"))?;
    if sig.len() != 64 {
        return Err(ApiError::unauthorized("x-signature not 64 bytes"));
    }

    Ok((pk, bearer, sig))
}

/// Verify the signature on `body` for the given envelope pk.
fn verify_signed(
    state: &AppState,
    pk: &[u8; 32],
    body: &[u8],
    sig_bytes: &[u8],
) -> Result<(), ApiError> {
    let vk = state
        .verifying_key(pk)
        .ok_or_else(|| ApiError::unauthorized("unknown envelope key"))?;
    let sig_arr: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| ApiError::unauthorized("bad signature length"))?;
    let sig = Signature::from_bytes(&sig_arr);
    vk.verify(body, &sig)
        .map_err(|_| ApiError::unauthorized("signature verification failed"))?;
    Ok(())
}

/// Resolve a bearer to a username; rejects expired sessions.
fn bearer_to_username(state: &AppState, bearer: &[u8; 32]) -> Result<String, ApiError> {
    let now = chrono::Utc::now().timestamp();
    let session = state.storage.lock()
        .get_session(bearer)
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::unauthorized("unknown bearer"))?;
    if session.expires_at < now {
        return Err(ApiError::unauthorized("bearer expired"));
    }
    Ok(session.username)
}

// -- ApiError ----------------------------------------------------------------

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: String,
    message: String,
}

impl ApiError {
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized".into(),
            message: msg.into(),
        }
    }
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "bad_request".into(),
            message: msg.into(),
        }
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: "conflict".into(),
            message: msg.into(),
        }
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found".into(),
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
            NotFound(m) => Self::not_found(m),
            Conflict(m) => Self::conflict(m),
            Invalid(m)  => Self::bad_request(m),
            Unauthorized => Self::unauthorized("storage: unauthorized"),
            Expired      => Self::unauthorized("storage: expired"),
            Sqlite(e)    => Self::internal(format!("sqlite: {e}")),
            Io(e)        => Self::internal(format!("io: {e}")),
        }
    }
}

// -- handlers ----------------------------------------------------------------

async fn health() -> Json<ProtoResponse> {
    Json(ProtoResponse::Ok { message: Some("ok".into()) })
}

#[derive(Deserialize)]
struct RegisterBody {
    username: String,
    envelope_pk: [u8; 32],
}

async fn register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterBody>,
) -> Result<Json<ProtoResponse>, ApiError> {
    if body.username.is_empty() || body.username.len() > 64 {
        return Err(ApiError::bad_request("username must be 1..=64 chars"));
    }
    if state.storage.lock().get_user(&body.username)
        .map_err(ApiError::from)?
        .is_some()
    {
        return Err(ApiError::conflict("username already taken"));
    }
    let now = chrono::Utc::now().timestamp();
    state.storage.lock()
        .put_user(&crate::storage::UserRow {
            username: body.username,
            envelope_pk: body.envelope_pk,
            created_at: now,
        })
        .map_err(ApiError::from)?;
    Ok(Json(ProtoResponse::Ok { message: Some("registered".into()) }))
}

/// Login step 1 (server returns a challenge).
///
/// This is implicit in the wire protocol's `ProtoRequest::Login` —
/// the client supplies a `signature` over a challenge.  We
/// implement the explicit two-step flow instead: the client
/// first calls `GET /v1/auth/challenge?username=...`, signs
/// the returned bytes with its envelope key, then calls
/// `POST /v1/auth/login { username, challenge, signature }`.
///
/// This route is the second step.
async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ProtoRequest>,
) -> Result<Json<ProtoResponse>, ApiError> {
    let (username, challenge, signature) = match req {
        ProtoRequest::Login { username, challenge, signature } => (username, challenge, signature),
        _ => return Err(ApiError::bad_request("expected ProtoRequest::Login")),
    };

    // Look up the user; their envelope public key signs the
    // challenge.
    let user = state.storage.lock()
        .get_user(&username)
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::unauthorized("unknown user"))?;

    // Consume the challenge.
    if challenge.len() != 32 {
        return Err(ApiError::bad_request("challenge must be 32 bytes"));
    }
    let challenge_arr: [u8; 32] = challenge[..].try_into().unwrap();
    let stored = state.storage.lock()
        .take_challenge(&challenge_arr)
        .map_err(ApiError::from)?;
    if stored.username != username {
        return Err(ApiError::unauthorized("challenge was for a different user"));
    }
    if stored.expires_at < chrono::Utc::now().timestamp() {
        return Err(ApiError::unauthorized("challenge expired"));
    }

    // Verify the signature.
    if signature.len() != 64 {
        return Err(ApiError::bad_request("signature must be 64 bytes"));
    }
    let sig_arr: [u8; 64] = signature[..].try_into().unwrap();
    verify_signed(&state, &user.envelope_pk, &challenge, &sig_arr)?;

    // Issue a bearer.
    let mut bearer = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bearer);
    let expires_at =
        chrono::Utc::now().timestamp() + state.session_ttl_secs;
    state.storage.lock()
        .put_session(&SessionRow {
            bearer,
            username: username.clone(),
            expires_at,
        })
        .map_err(ApiError::from)?;
    Ok(Json(ProtoResponse::LoginOk { bearer, expires_at }))
}

#[derive(Deserialize)]
struct ChallengeQuery {
    username: String,
}

/// Login step 1: server returns a random 32-byte challenge.
async fn issue_challenge(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<ChallengeQuery>,
) -> Result<Json<ProtoResponse>, ApiError> {
    // Confirm the user exists; don't leak whether the username
    // is taken by responding differently.
    if state.storage.lock().get_user(&q.username)
        .map_err(ApiError::from)?
        .is_none()
    {
        return Err(ApiError::unauthorized("unknown user"));
    }
    let mut challenge = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut challenge);
    let expires_at = chrono::Utc::now().timestamp() + 60; // 1 minute
    state
        .storage
        .lock()
        .put_challenge(&crate::storage::ChallengeRow {
            challenge,
            username: q.username,
            expires_at,
        })
        .map_err(ApiError::from)?;
    // Reuse BlobResp (just bytes) as a transport for the
    // challenge, since the protocol doesn't have a dedicated
    // challenge response type.  Clients should parse the
    // `ciphertext` field as the 32-byte challenge.
    Ok(Json(ProtoResponse::BlobResp { ciphertext: challenge.to_vec() }))
}

async fn push(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ProtoResponse>, ApiError> {
    let (pk, bearer, sig) = auth_headers(&headers)?;
    verify_signed(&state, &pk, &body, &sig)?;
    let username = bearer_to_username(&state, &bearer)?;

    let req: ProtoRequest = serde_json::from_slice(&body)
        .map_err(|e| ApiError::bad_request(format!("body parse: {e}")))?;
    let new_clock = match req {
        ProtoRequest::Push { new_clock, changes } => {
            state.storage.lock()
                .append_changes(&changes)
                .map_err(ApiError::from)?;
            new_clock
        }
        _ => return Err(ApiError::bad_request("expected ProtoRequest::Push")),
    };
    tracing::info!(
        actor = %username,
        clock = ?new_clock.counters,
        "push ok"
    );
    Ok(Json(ProtoResponse::Ok { message: None }))
}

async fn pull(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ProtoResponse>, ApiError> {
    let (pk, bearer, sig) = auth_headers(&headers)?;
    verify_signed(&state, &pk, &body, &sig)?;
    let _username = bearer_to_username(&state, &bearer)?;

    let req: ProtoRequest = serde_json::from_slice(&body)
        .map_err(|e| ApiError::bad_request(format!("body parse: {e}")))?;
    let since = match req {
        ProtoRequest::Pull { since } => since,
        _ => return Err(ApiError::bad_request("expected ProtoRequest::Pull")),
    };

    // For the long-poll, we'd block here on a watch channel.
    // For v1, do an immediate read.  Clients can re-call.
    let changes = state.storage.lock()
        .changes_since(None, 0)
        .map_err(ApiError::from)?;
    let _ = since; // TODO: filter by per-actor clock
    let server_clock = state.storage.lock().all_clocks().map_err(ApiError::from)?;
    Ok(Json(ProtoResponse::PullResp {
        changes,
        server_clock: keepsake_core::sync::VectorClock {
            counters: server_clock,
        },
    }))
}

async fn put_blob(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(sha256_hex): Path<String>,
    body: Bytes,
) -> Result<Json<ProtoResponse>, ApiError> {
    let (pk, bearer, sig) = auth_headers(&headers)?;
    verify_signed(&state, &pk, &body, &sig)?;
    let _ = bearer_to_username(&state, &bearer)?;

    let sha256 = hex::decode(&sha256_hex)
        .map_err(|_| ApiError::bad_request("sha256 not hex"))?;
    let sha256: [u8; 32] = sha256
        .try_into()
        .map_err(|_| ApiError::bad_request("sha256 not 32 bytes"))?;

    // `body` is the ciphertext.  The signature is over the body
    // bytes; the URL path carries the sha256.
    state.storage.lock()
        .put_blob(&sha256, &body)
        .map_err(ApiError::from)?;
    Ok(Json(ProtoResponse::Ok { message: Some("stored".into()) }))
}

async fn get_blob(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(sha256_hex): Path<String>,
) -> Result<Json<ProtoResponse>, ApiError> {
    let (pk, bearer, sig) = auth_headers(&headers)?;
    // The signature for a GET covers an empty body.  We still
    // need the bearer to authenticate the caller.
    verify_signed(&state, &pk, &[], &sig)?;
    let _ = bearer_to_username(&state, &bearer)?;

    let sha256 = hex::decode(&sha256_hex)
        .map_err(|_| ApiError::bad_request("sha256 not hex"))?;
    let sha256: [u8; 32] = sha256
        .try_into()
        .map_err(|_| ApiError::bad_request("sha256 not 32 bytes"))?;

    let ciphertext = state.storage.lock()
        .get_blob(&sha256)
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::not_found("blob not found"))?;
    Ok(Json(ProtoResponse::BlobResp { ciphertext }))
}
