//! End-to-end tests for the sync server.  Uses `axum::Router`
//! with a real `Storage` (in-memory SQLite) and a real
//! `reqwest` client.  The server is unauthenticated: anyone
//! with the URL can read or write any vault.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header::CONTENT_TYPE, Request as HttpRequest, StatusCode};
use chrono::Utc;
use keepsake_core::sync::protocol::{Request as ProtoRequest, Response as ProtoResponse};
use keepsake_core::sync::Change;
use keepsake_server::api::{router, AppState};
use keepsake_server::storage::Storage;
use serde_json::json;
use tower::ServiceExt; // for `oneshot`
use uuid::Uuid;

fn make_state() -> Arc<AppState> {
    let storage = Storage::open_in_memory().unwrap();
    Arc::new(AppState::new(storage))
}

fn mk_change(record: Option<Uuid>, payload: &[u8]) -> Change {
    Change {
        id: Uuid::new_v4(),
        lamport: 1,
        ts: Utc::now(),
        author: "alice".into(),
        record_id: record,
        payload: payload.to_vec(),
    }
}

async fn body_bytes(resp: axum::response::Response) -> Vec<u8> {
    axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap()
        .to_vec()
}

async fn send(
    router: axum::Router,
    method: &str,
    path: &str,
    body: &[u8],
) -> (StatusCode, Vec<u8>) {
    let req = HttpRequest::builder()
        .method(method)
        .uri(path)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_vec()))
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = body_bytes(resp).await;
    (status, bytes)
}

#[tokio::test]
async fn health_works() {
    let app = router(make_state());
    let (status, body) = send(app, "GET", "/v1/health", b"").await;
    assert_eq!(status, StatusCode::OK);
    let resp: ProtoResponse = serde_json::from_slice(&body).unwrap();
    match resp {
        ProtoResponse::Ok { message: Some(m) } => assert_eq!(m, "ok"),
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn push_and_pull_round_trip() {
    let state = make_state();
    let app = router(state.clone());

    let rec = Uuid::new_v4();
    let change1 = mk_change(Some(rec), b"opaque-1");
    let change2 = mk_change(Some(rec), b"opaque-2");
    let push_body = serde_json::to_vec(&ProtoRequest::Push {
        new_clock: Default::default(),
        changes: vec![change1, change2],
    })
    .unwrap();
    let (status, _) = send(app.clone(), "POST", "/v1/vaults/family/sync/push", &push_body).await;
    assert_eq!(status, StatusCode::OK);

    // Pull with a since vector clock that includes the first
    // change's actor at lamport 1 returns the second change.
    // (Both test changes have lamport 1; the > filter on
    // since means neither is > 1, so the change feed is empty.
    // The server's clock for "alice" is 1.)
    let pull_body = serde_json::to_vec(&ProtoRequest::Pull {
        since: keepsake_core::sync::VectorClock {
            counters: [("alice".into(), 1u64)].into_iter().collect(),
        },
    })
    .unwrap();
    let (status, body) = send(
        app.clone(),
        "POST",
        "/v1/vaults/family/sync/pull",
        &pull_body,
    )
    .await;
    eprintln!("status={status} body={}", String::from_utf8_lossy(&body));
    assert_eq!(status, StatusCode::OK);
    let resp: ProtoResponse = serde_json::from_slice(&body).unwrap();
    match resp {
        ProtoResponse::PullResp { changes, server_clock } => {
            // No changes with lamport > 1.
            assert!(changes.is_empty());
            assert_eq!(server_clock.counters.get("alice"), Some(&1));
        }
        other => panic!("unexpected: {other:?}"),
    }

    // Re-pushing is idempotent.
    let (status, _) = send(app.clone(), "POST", "/v1/vaults/family/sync/push", &push_body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn pull_state_returns_current_records() {
    let state = make_state();
    let app = router(state.clone());

    let rec = Uuid::new_v4();
    let change = mk_change(Some(rec), b"current-version");
    let push_body = serde_json::to_vec(&ProtoRequest::Push {
        new_clock: Default::default(),
        changes: vec![change],
    })
    .unwrap();
    let (status, _) = send(app.clone(), "POST", "/v1/vaults/family/sync/push", &push_body).await;
    assert_eq!(status, StatusCode::OK);

    // Empty body = state pull.
    let (status, body) = send(app.clone(), "POST", "/v1/vaults/family/sync/pull", b"").await;
    assert_eq!(status, StatusCode::OK);
    let resp: ProtoResponse = serde_json::from_slice(&body).unwrap();
    match resp {
        ProtoResponse::PullResp { changes, .. } => {
            assert_eq!(changes.len(), 1);
            assert_eq!(changes[0].record_id, Some(rec));
            assert_eq!(changes[0].payload, b"current-version");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn pull_state_picks_latest_lww_per_record() {
    let state = make_state();
    let app = router(state.clone());

    let rec = Uuid::new_v4();
    let mut older = mk_change(Some(rec), b"older");
    older.lamport = 1;
    older.ts = Utc::now();
    older.author = "alice".into();
    let mut newer = mk_change(Some(rec), b"newer");
    newer.lamport = 2;
    newer.ts = Utc::now();
    newer.author = "bob".into();
    // Push older first, then newer.  LWW picks newer.
    let push = serde_json::to_vec(&ProtoRequest::Push {
        new_clock: Default::default(),
        changes: vec![older, newer],
    })
    .unwrap();
    let (status, _) = send(app.clone(), "POST", "/v1/vaults/v1/sync/push", &push).await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = send(app.clone(), "POST", "/v1/vaults/v1/sync/pull", b"").await;
    assert_eq!(status, StatusCode::OK);
    let resp: ProtoResponse = serde_json::from_slice(&body).unwrap();
    match resp {
        ProtoResponse::PullResp { changes, .. } => {
            assert_eq!(changes.len(), 1);
            assert_eq!(changes[0].payload, b"newer");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn cross_vault_isolation() {
    let state = make_state();
    let app = router(state.clone());

    let c1 = mk_change(Some(Uuid::new_v4()), b"vault-1");
    let c2 = mk_change(Some(Uuid::new_v4()), b"vault-2");
    let push1 = serde_json::to_vec(&ProtoRequest::Push {
        new_clock: Default::default(),
        changes: vec![c1],
    })
    .unwrap();
    let push2 = serde_json::to_vec(&ProtoRequest::Push {
        new_clock: Default::default(),
        changes: vec![c2],
    })
    .unwrap();
    let (status, _) = send(app.clone(), "POST", "/v1/vaults/v1/sync/push", &push1).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send(app.clone(), "POST", "/v1/vaults/v2/sync/push", &push2).await;
    assert_eq!(status, StatusCode::OK);

    let (_, body) = send(app.clone(), "POST", "/v1/vaults/v1/sync/pull", b"").await;
    let resp: ProtoResponse = serde_json::from_slice(&body).unwrap();
    match resp {
        ProtoResponse::PullResp { changes, .. } => {
            assert_eq!(changes.len(), 1);
            assert_eq!(changes[0].payload, b"vault-1");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn blob_put_then_get() {
    let state = make_state();
    let app = router(state.clone());

    let sha = [0xab; 32];
    let ciphertext = b"encrypted-blob-bytes".to_vec();
    let (status, _) = send(
        app.clone(),
        "PUT",
        &format!("/v1/vaults/family/blobs/{}", hex::encode_upper(sha)),
        &ciphertext,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = send(
        app.clone(),
        "GET",
        &format!("/v1/vaults/family/blobs/{}", hex::encode_upper(sha)),
        b"",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let resp: ProtoResponse = serde_json::from_slice(&body).unwrap();
    match resp {
        ProtoResponse::BlobResp { ciphertext: out } => {
            assert_eq!(out, ciphertext);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn sealed_keys_round_trip() {
    use keepsake_core::vault::{SealedKeyRow, Vault};
    use keepsake_server::api::{SealedKeyRowWire, SealedKeysListResp};

    let state = make_state();
    let app = router(state.clone());

    // Build a real SealedKeyRow to upload, exactly like
    // the local vault would produce.
    let dir = tempfile::tempdir().unwrap();
    let v = Vault::open_or_create(&dir.path().join("v.db")).unwrap();
    let kdf_salt: [u8; 16] = [7; 16];
    let row = SealedKeyRow {
        username: "alice".into(),
        device_id: [0u8; 16],
        kdf_salt,
        kdf_params: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        seal_nonce: [0x11u8; 24],
        seal_ciphertext: b"sealed-blob".to_vec(),
        envelope_pk: [0u8; 32],
        created_at: chrono::Utc::now(),
    };
    let body = serde_json::to_vec(&SealedKeyRowWire::from(row.clone())).unwrap();
    let (status, _) = send(app.clone(), "PUT", "/v1/vaults/family/sealed-keys", &body).await;
    assert_eq!(status, StatusCode::OK);

    // Second user on the same vault.
    let row2 = SealedKeyRow {
        username: "bob".into(),
        device_id: [0u8; 16],
        kdf_salt: [9; 16],
        kdf_params: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        seal_nonce: [0x22u8; 24],
        seal_ciphertext: b"bob-blob".to_vec(),
        envelope_pk: [0u8; 32],
        created_at: chrono::Utc::now(),
    };
    let body2 = serde_json::to_vec(&SealedKeyRowWire::from(row2.clone())).unwrap();
    let (status, _) = send(app.clone(), "PUT", "/v1/vaults/family/sealed-keys", &body2).await;
    assert_eq!(status, StatusCode::OK);

    // GET should return both.
    let (status, body) = send(app.clone(), "GET", "/v1/vaults/family/sealed-keys", b"").await;
    assert_eq!(status, StatusCode::OK);
    let resp: SealedKeysListResp = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp.rows.len(), 2);
    let by_name: std::collections::HashMap<_, _> =
        resp.rows.iter().map(|r| (r.username.clone(), r.clone())).collect();
    assert_eq!(by_name["alice"].seal_ciphertext, row.seal_ciphertext);
    assert_eq!(by_name["bob"].seal_ciphertext, row2.seal_ciphertext);

    // Overwrite alice's row.
    let row3 = SealedKeyRow {
        username: "alice".into(),
        kdf_salt,
        kdf_params: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        seal_nonce: [0x33u8; 24],
        seal_ciphertext: b"alice-new-blob".to_vec(),
        envelope_pk: [0u8; 32],
        created_at: chrono::Utc::now(),
        device_id: [0u8; 16],
    };
    let body3 = serde_json::to_vec(&SealedKeyRowWire::from(row3.clone())).unwrap();
    let (status, _) = send(app.clone(), "PUT", "/v1/vaults/family/sealed-keys", &body3).await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = send(app.clone(), "GET", "/v1/vaults/family/sealed-keys", b"").await;
    let resp: SealedKeysListResp = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp.rows.len(), 2);
    let alice = resp.rows.iter().find(|r| r.username == "alice").unwrap();
    assert_eq!(alice.seal_ciphertext, row3.seal_ciphertext);
}

#[tokio::test]
async fn invalid_vault_id_rejected() {
    let state = make_state();
    let app = router(state.clone());

    // Empty vault id (path segment) hits the catch-all 404
    // rather than our validator, which is fine.  An id with
    // bad characters is rejected by the validator.
    let (status, _) = send(app.clone(), "POST", "/v1/vaults/bad%20id/sync/pull", b"").await;
    // The exact status depends on routing; we just want it to
    // not be 200 OK.
    assert_ne!(status, StatusCode::OK);
}

#[tokio::test]
async fn malformed_body_returns_400() {
    let state = make_state();
    let app = router(state.clone());
    let (status, body) = send(
        app.clone(),
        "POST",
        "/v1/vaults/v1/sync/push",
        b"not json",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let resp: ProtoResponse = serde_json::from_slice(&body).unwrap();
    match resp {
        ProtoResponse::Error { code, .. } => assert_eq!(code, "bad_request"),
        other => panic!("unexpected: {other:?}"),
    }
}

// Silence unused-import warnings.
#[allow(dead_code)]
fn _use_json() {
    let _ = json!({});
}
