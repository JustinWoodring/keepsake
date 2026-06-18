//! End-to-end tests for the sync server.  Uses `axum::Router`
//! with a real `Storage` (in-memory SQLite) and a real
//! `reqwest` client.  Signs every authenticated request with
//! an ephemeral Ed25519 key.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header::CONTENT_TYPE, Request as HttpRequest, StatusCode};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey, SECRET_KEY_LENGTH};
use keepsake_core::sync::protocol::{Request as ProtoRequest, Response as ProtoResponse};
use keepsake_core::sync::Change;
use keepsake_server::api::{router, AppState};
use keepsake_server::storage::Storage;
use rand::rngs::OsRng;
use rand::RngCore;
use serde_json::json;
use tower::ServiceExt; // for `oneshot`

fn make_keypair() -> SigningKey {
    let mut bytes = [0u8; SECRET_KEY_LENGTH];
    OsRng.fill_bytes(&mut bytes);
    SigningKey::from_bytes(&bytes)
}

fn envelope_pk(sk: &SigningKey) -> [u8; 32] {
    sk.verifying_key().to_bytes()
}

fn make_state() -> Arc<AppState> {
    let storage = Storage::open_in_memory().unwrap();
    Arc::new(AppState::new(storage))
}

async fn body_bytes(resp: axum::response::Response) -> Vec<u8> {
    axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap()
        .to_vec()
}

/// Send a request signed with `sk`, with the given bearer.
/// If `sk` is None, no signature header is sent.
async fn send_signed(
    router: axum::Router,
    method: &str,
    path: &str,
    body: &[u8],
    sk: Option<&SigningKey>,
    bearer: Option<&[u8; 32]>,
) -> (StatusCode, Vec<u8>) {
    let mut builder = HttpRequest::builder()
        .method(method)
        .uri(path)
        .header(CONTENT_TYPE, "application/json");
    if let Some(sk) = sk {
        let sig = sk.sign(body);
        builder = builder
            .header("x-envelope-pk", hex::encode_upper(envelope_pk(sk)))
            .header("x-signature", hex::encode_upper(sig.to_bytes()));
    }
    if let Some(b) = bearer {
        builder = builder.header("authorization", format!("Bearer {}", hex::encode_upper(b)));
    }
    let req = builder.body(Body::from(body.to_vec())).unwrap();
    let resp = router.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = body_bytes(resp).await;
    (status, bytes)
}

#[tokio::test]
async fn health_works() {
    let app = router(make_state());
    let (status, body) = send_signed(app, "GET", "/v1/health", b"", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let resp: ProtoResponse = serde_json::from_slice(&body).unwrap();
    match resp {
        ProtoResponse::Ok { message: Some(m) } => assert_eq!(m, "ok"),
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn register_then_login_flow() {
    let state = make_state();
    let app = router(state.clone());

    let sk = make_keypair();
    let pk = envelope_pk(&sk);

    // 1. Register.
    let body = serde_json::to_vec(&ProtoRequest::Register {
        username: "alice".into(),
        envelope_pk: pk,
    })
    .unwrap();
    let (status, _) = send_signed(app.clone(), "POST", "/v1/auth/register", &body, None, None).await;
    assert_eq!(status, StatusCode::OK);

    // 2. Re-registering is a conflict.
    let (status, _) = send_signed(app.clone(), "POST", "/v1/auth/register", &body, None, None).await;
    assert_eq!(status, StatusCode::CONFLICT);

    // 3. Get a challenge.
    let (status, body) =
        send_signed(app.clone(), "GET", "/v1/auth/challenge?username=alice", b"", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let resp: ProtoResponse = serde_json::from_slice(&body).unwrap();
    let challenge: [u8; 32] = match resp {
        ProtoResponse::BlobResp { ciphertext } => {
            assert_eq!(ciphertext.len(), 32);
            ciphertext.try_into().unwrap()
        }
        other => panic!("expected BlobResp, got {other:?}"),
    };

    // 4. Sign the challenge and POST login.
    let sig = sk.sign(&challenge);
    let body = serde_json::to_vec(&ProtoRequest::Login {
        username: "alice".into(),
        challenge: challenge.to_vec(),
        signature: sig.to_bytes().to_vec(),
    })
    .unwrap();
    let (status, body) = send_signed(app.clone(), "POST", "/v1/auth/login", &body, None, None).await;
    assert_eq!(status, StatusCode::OK);
    let resp: ProtoResponse = serde_json::from_slice(&body).unwrap();
    let bearer: [u8; 32] = match resp {
        ProtoResponse::LoginOk { bearer, .. } => bearer,
        other => panic!("expected LoginOk, got {other:?}"),
    };

    // 5. Login with a forged signature is rejected.  Re-issue
    // a challenge because the previous one was consumed.
    let (_, body) = send_signed(app.clone(), "GET", "/v1/auth/challenge?username=alice", b"", None, None).await;
    let challenge2: [u8; 32] = match serde_json::from_slice::<ProtoResponse>(&body).unwrap() {
        ProtoResponse::BlobResp { ciphertext } => ciphertext.try_into().unwrap(),
        _ => panic!(),
    };
    let bad_sk = make_keypair();
    let bad_sig = bad_sk.sign(&challenge2);
    let body = serde_json::to_vec(&ProtoRequest::Login {
        username: "alice".into(),
        challenge: challenge2.to_vec(),
        signature: bad_sig.to_bytes().to_vec(),
    })
    .unwrap();
    let (status, _) = send_signed(app.clone(), "POST", "/v1/auth/login", &body, None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // 6. The session is valid: a signed request with the
    // bearer succeeds.
    let body = serde_json::to_vec(&json!({"kind": "pull", "since": {"counters": {}}})).unwrap();
    let (status, _) = send_signed(
        app.clone(),
        "POST",
        "/v1/sync/pull",
        &body,
        Some(&sk),
        Some(&bearer),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn push_and_pull_round_trip() {
    let state = make_state();
    let app = router(state.clone());
    let sk = make_keypair();
    let pk = envelope_pk(&sk);

    // Register + login.
    let body = serde_json::to_vec(&ProtoRequest::Register { username: "alice".into(), envelope_pk: pk }).unwrap();
    send_signed(app.clone(), "POST", "/v1/auth/register", &body, None, None).await;
    let (_, body) = send_signed(app.clone(), "GET", "/v1/auth/challenge?username=alice", b"", None, None).await;
    let challenge: [u8; 32] = match serde_json::from_slice::<ProtoResponse>(&body).unwrap() {
        ProtoResponse::BlobResp { ciphertext } => ciphertext.try_into().unwrap(),
        _ => panic!(),
    };
    let sig = sk.sign(&challenge);
    let body = serde_json::to_vec(&ProtoRequest::Login {
        username: "alice".into(),
        challenge: challenge.to_vec(),
        signature: sig.to_bytes().to_vec(),
    }).unwrap();
    let (_, body) = send_signed(app.clone(), "POST", "/v1/auth/login", &body, None, None).await;
    let bearer: [u8; 32] = match serde_json::from_slice::<ProtoResponse>(&body).unwrap() {
        ProtoResponse::LoginOk { bearer, .. } => bearer,
        _ => panic!(),
    };

    // Push two changes.
    let change1 = Change {
        id: uuid::Uuid::new_v4(),
        lamport: 1,
        ts: chrono::Utc::now(),
        author: "alice".into(),
        record_id: None,
        payload: b"opaque-1".to_vec(),
    };
    let change2 = Change {
        id: uuid::Uuid::new_v4(),
        lamport: 2,
        ts: chrono::Utc::now(),
        author: "alice".into(),
        record_id: None,
        payload: b"opaque-2".to_vec(),
    };
    let push_body = serde_json::to_vec(&ProtoRequest::Push {
        new_clock: keepsake_core::sync::VectorClock {
            counters: [("alice".into(), 2u64)].into_iter().collect(),
        },
        changes: vec![change1.clone(), change2.clone()],
    })
    .unwrap();
    let (status, _) = send_signed(
        app.clone(),
        "POST",
        "/v1/sync/push",
        &push_body,
        Some(&sk),
        Some(&bearer),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Pull returns both changes.
    let pull_body = serde_json::to_vec(&ProtoRequest::Pull {
        since: keepsake_core::sync::VectorClock::default(),
    })
    .unwrap();
    let (status, body) = send_signed(
        app.clone(),
        "POST",
        "/v1/sync/pull",
        &pull_body,
        Some(&sk),
        Some(&bearer),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let resp: ProtoResponse = serde_json::from_slice(&body).unwrap();
    match resp {
        ProtoResponse::PullResp { changes, server_clock } => {
            assert_eq!(changes.len(), 2);
            assert_eq!(server_clock.counters.get("alice"), Some(&2));
        }
        other => panic!("unexpected: {other:?}"),
    }

    // Push the same changes again (idempotency): should not
    // double them.
    let (status, _) = send_signed(
        app.clone(),
        "POST",
        "/v1/sync/push",
        &push_body,
        Some(&sk),
        Some(&bearer),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = send_signed(
        app.clone(),
        "POST",
        "/v1/sync/pull",
        &pull_body,
        Some(&sk),
        Some(&bearer),
    )
    .await;
    match serde_json::from_slice::<ProtoResponse>(&body).unwrap() {
        ProtoResponse::PullResp { changes, .. } => assert_eq!(changes.len(), 2),
        _ => panic!(),
    }
}

#[tokio::test]
async fn push_rejects_bad_signature() {
    let state = make_state();
    let app = router(state.clone());
    let sk = make_keypair();
    let pk = envelope_pk(&sk);
    let bad_sk = make_keypair();

    let body = serde_json::to_vec(&ProtoRequest::Register { username: "alice".into(), envelope_pk: pk }).unwrap();
    send_signed(app.clone(), "POST", "/v1/auth/register", &body, None, None).await;
    let (_, body) = send_signed(app.clone(), "GET", "/v1/auth/challenge?username=alice", b"", None, None).await;
    let challenge: [u8; 32] = match serde_json::from_slice::<ProtoResponse>(&body).unwrap() {
        ProtoResponse::BlobResp { ciphertext } => ciphertext.try_into().unwrap(),
        _ => panic!(),
    };
    let sig = sk.sign(&challenge);
    let body = serde_json::to_vec(&ProtoRequest::Login {
        username: "alice".into(),
        challenge: challenge.to_vec(),
        signature: sig.to_bytes().to_vec(),
    }).unwrap();
    let (_, body) = send_signed(app.clone(), "POST", "/v1/auth/login", &body, None, None).await;
    let bearer: [u8; 32] = match serde_json::from_slice::<ProtoResponse>(&body).unwrap() {
        ProtoResponse::LoginOk { bearer, .. } => bearer,
        _ => panic!(),
    };

    // Push signed with the *wrong* key: sign with bad_sk but
    // declare the envelope as `sk` so the server's verify
    // step fails.
    let push_body = serde_json::to_vec(&ProtoRequest::Push {
        new_clock: Default::default(),
        changes: vec![],
    }).unwrap();
    let bad_sig = bad_sk.sign(&push_body);
    let mut builder = HttpRequest::builder()
        .method("POST")
        .uri("/v1/sync/push")
        .header(CONTENT_TYPE, "application/json")
        .header("x-envelope-pk", hex::encode_upper(envelope_pk(&sk))) // real pk
        .header("x-signature", hex::encode_upper(bad_sig.to_bytes()))   // bad sig
        .header("authorization", format!("Bearer {}", hex::encode_upper(bearer)));
    let req = builder.body(Body::from(push_body.clone())).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn push_rejects_missing_bearer() {
    let state = make_state();
    let app = router(state.clone());
    let sk = make_keypair();
    let pk = envelope_pk(&sk);
    let body = serde_json::to_vec(&ProtoRequest::Register { username: "alice".into(), envelope_pk: pk }).unwrap();
    send_signed(app.clone(), "POST", "/v1/auth/register", &body, None, None).await;
    let body = serde_json::to_vec(&ProtoRequest::Push { new_clock: Default::default(), changes: vec![] }).unwrap();
    // No bearer, valid signature.
    let (status, _) = send_signed(app.clone(), "POST", "/v1/sync/push", &body, Some(&sk), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn blob_put_then_get() {
    let state = make_state();
    let app = router(state.clone());
    let sk = make_keypair();
    let pk = envelope_pk(&sk);

    let body = serde_json::to_vec(&ProtoRequest::Register { username: "alice".into(), envelope_pk: pk }).unwrap();
    send_signed(app.clone(), "POST", "/v1/auth/register", &body, None, None).await;
    let (_, body) = send_signed(app.clone(), "GET", "/v1/auth/challenge?username=alice", b"", None, None).await;
    let challenge: [u8; 32] = match serde_json::from_slice::<ProtoResponse>(&body).unwrap() {
        ProtoResponse::BlobResp { ciphertext } => ciphertext.try_into().unwrap(),
        _ => panic!(),
    };
    let sig = sk.sign(&challenge);
    let body = serde_json::to_vec(&ProtoRequest::Login {
        username: "alice".into(),
        challenge: challenge.to_vec(),
        signature: sig.to_bytes().to_vec(),
    }).unwrap();
    let (_, body) = send_signed(app.clone(), "POST", "/v1/auth/login", &body, None, None).await;
    let bearer: [u8; 32] = match serde_json::from_slice::<ProtoResponse>(&body).unwrap() {
        ProtoResponse::LoginOk { bearer, .. } => bearer,
        _ => panic!(),
    };

    // Put a blob.
    let sha256 = [0xab; 32];
    let ciphertext = b"encrypted-blob-bytes".to_vec();
    let (status, _) = send_signed(
        app.clone(),
        "PUT",
        &format!("/v1/blobs/{}", hex::encode_upper(sha256)),
        &ciphertext,
        Some(&sk),
        Some(&bearer),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Get the blob.
    let (status, body) = send_signed(
        app.clone(),
        "GET",
        &format!("/v1/blobs/{}", hex::encode_upper(sha256)),
        b"",
        Some(&sk),
        Some(&bearer),
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

#[test]
fn storage_round_trip() {
    use keepsake_server::storage::{SessionRow, UserRow};

    let storage = Storage::open_in_memory().unwrap();
    let now = chrono::Utc::now().timestamp();
    storage.put_user(&UserRow {
        username: "alice".into(),
        envelope_pk: [1u8; 32],
        created_at: now,
    }).unwrap();
    let user = storage.get_user("alice").unwrap().unwrap();
    assert_eq!(user.envelope_pk, [1u8; 32]);

    storage.put_session(&SessionRow {
        bearer: [2u8; 32],
        username: "alice".into(),
        expires_at: now + 60,
    }).unwrap();
    let session = storage.get_session(&[2u8; 32]).unwrap().unwrap();
    assert_eq!(session.username, "alice");

    // Expired session lookup succeeds (storage doesn't
    // auto-prune); only the API layer enforces expiry.
    storage.put_session(&SessionRow {
        bearer: [3u8; 32],
        username: "bob".into(),
        expires_at: now - 1,
    }).unwrap();
    let n = storage.delete_expired_sessions(now).unwrap();
    assert_eq!(n, 1);
    assert!(storage.get_session(&[3u8; 32]).unwrap().is_none());

    // Blob round-trip.
    let sha = [0xab; 32];
    storage.put_blob(&sha, b"hello").unwrap();
    assert_eq!(storage.get_blob(&sha).unwrap(), Some(b"hello".to_vec()));
}

// Silence unused-import warnings if a test is removed.
#[allow(dead_code)]
fn _silence(_: &SigningKey, _: &VerifyingKey) {}
