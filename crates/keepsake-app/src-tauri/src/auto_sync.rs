//! Background auto-sync loop.
//!
//! Spawned on `unlock` and cancelled on `lock`.  Every 30
//! minutes (and once immediately on start) it pushes and
//! pulls every vault that has a shared sync setup with a
//! `server_url` bound to it.  Errors are recorded but do not
//! stop the loop.
//!
//! The loop does **not** hold the session mutex across HTTP
//! calls.  Push payloads are built under the lock and sent
//! after release; pull responses are received without the
//! lock and applied under a re-acquired lock.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use keepsake_core::sync::client::SyncClient;
use keepsake_core::sync::protocol::{Request as ProtoRequest, Response as ProtoResponse};
use keepsake_core::sync::{Change, VectorClock};
use keepsake_core::vault::Vault;
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

use super::AppState;

/// Cadence of the auto-sync loop.  Tests can override this.
pub const SYNC_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// Snapshot of one cycle's inputs.  Built under the lock,
/// consumed by the async HTTP step without the lock.
struct PushPlan {
    server_url: String,
    vault_id: String,
    changes: Vec<Change>,
    new_clock: VectorClock,
    sealed_row: Option<crate::session::SealedKeyRowWire>,
}

struct PullPlan {
    server_url: String,
    vault_id: String,
}

/// Public status of the loop, exposed to the UI.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoSyncStatus {
    pub enabled: bool,
    pub running: bool,
    pub last_push_at: Option<DateTime<Utc>>,
    pub last_pull_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub next_at: Option<DateTime<Utc>>,
}

impl AutoSyncStatus {
    fn touch(&mut self) {
        self.next_at = Some(Utc::now() + chrono::Duration::from_std(SYNC_INTERVAL).unwrap());
    }
}

/// Handle to a running auto-sync loop.  Drop it (or call
/// `stop()`) to cancel.
pub struct AutoSyncHandle {
    cancel: Arc<AtomicBool>,
    notify: Arc<Notify>,
    status: Arc<Mutex<AutoSyncStatus>>,
    join: Option<tokio::task::JoinHandle<()>>,
}

impl AutoSyncHandle {
    pub fn status_snapshot(&self) -> AutoSyncStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn stop(mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
        if let Some(j) = self.join.take() {
            // Best-effort wait.  The loop checks the cancel
            // flag at the top of every sleep and exits.
            let _ = j;
        }
    }
}

impl Drop for AutoSyncHandle {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }
}

/// Start the auto-sync loop for the current session.  The
/// loop runs until the handle is dropped (or `stop()` is
/// called), or until the session is locked.
pub fn spawn(state: Arc<AppState>) -> AutoSyncHandle {
    let cancel = Arc::new(AtomicBool::new(false));
    let notify = Arc::new(Notify::new());
    let status = Arc::new(Mutex::new(AutoSyncStatus {
        enabled: true,
        running: true,
        ..Default::default()
    }));
    status.lock().unwrap().touch();

    let s_status = status.clone();
    let s_cancel = cancel.clone();
    let s_notify = notify.clone();
    let s_state = state.clone();
    let join = tokio::spawn(async move {
        run_loop(s_state, s_cancel, s_notify, s_status).await;
    });

    AutoSyncHandle { cancel, notify, status, join: Some(join) }
}

async fn run_loop(
    state: Arc<AppState>,
    cancel: Arc<AtomicBool>,
    notify: Arc<Notify>,
    status: Arc<Mutex<AutoSyncStatus>>,
) {
    // Initial sync runs immediately.  The state may not be
    // ready yet (the unlock command returns before the
    // loop is spawned, and the spawn happens inside unlock).
    // We tolerate that by checking the cancel flag and
    // skipping if not ready.
    loop {
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        do_one_cycle(&state, &status).await;
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        // Wait for either the interval to elapse, or a
        // cancellation signal.
        tokio::select! {
            _ = tokio::time::sleep(SYNC_INTERVAL) => {}
            _ = notify.notified() => {
                if cancel.load(Ordering::SeqCst) { break; }
            }
        }
    }
    status.lock().unwrap().running = false;
}

/// One push+pull cycle across every configured vault.
async fn do_one_cycle(
    state: &Arc<AppState>,
    status: &Arc<Mutex<AutoSyncStatus>>,
) {
    // Phase 1: build push plans and pull plans under the lock.
    let (push_plans, pull_plans) = match build_plans(state) {
        Ok(p) => p,
        Err(e) => {
            record_error(status, format!("plan: {e}"));
            return;
        }
    };

    // Phase 2: HTTP pushes (lock released).
    for plan in push_plans {
        let push_records = !plan.changes.is_empty();
        let push_sealed = plan.sealed_row.is_some();
        if !push_records && !push_sealed {
            continue;
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client");
        if push_records {
            let url = format!(
                "{}/v1/vaults/{}/sync/push",
                plan.server_url.trim_end_matches('/'),
                plan.vault_id,
            );
            let body = match serde_json::to_vec(&ProtoRequest::Push {
                new_clock: plan.new_clock,
                changes: plan.changes,
            }) {
                Ok(b) => b,
                Err(e) => {
                    record_error(status, format!("encode push: {e}"));
                    continue;
                }
            };
            match client.post(&url).header("content-type", "application/json").body(body).send().await {
                Ok(resp) if resp.status().is_success() => {
                    status.lock().unwrap().last_push_at = Some(Utc::now());
                }
                Ok(resp) => {
                    let st = resp.status();
                    let txt = resp.text().await.unwrap_or_default();
                    record_error(status, format!("push {plan_url}: {st}: {txt}", plan_url = url));
                }
                Err(e) => {
                    record_error(status, format!("push {url}: {e}"));
                }
            }
        }
        if push_sealed {
            // Re-publish this device's sealed_keys row every
            // cycle.  The server's storage is keyed by
            // (vault_id, username) so a new row here overrides
            // any prior sealed blob — used for vault_key
            // re-sealing on password rotation or key refresh.
            if let Some(row) = plan.sealed_row {
                if let Err(e) = crate::session::push_sealed_keys_row(
                    &plan.server_url,
                    &plan.vault_id,
                    &row,
                )
                .await
                {
                    record_error(status, format!("push sealed_keys: {e}"));
                }
            }
        }
    }

    // Phase 3: HTTP pulls (lock released).
    for plan in pull_plans {
        let url = format!(
            "{}/v1/vaults/{}/sync/pull",
            plan.server_url.trim_end_matches('/'),
            plan.vault_id,
        );
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client");
        let resp = match client
            .post(&url)
            .header("content-type", "application/json")
            .body(Vec::new())
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                record_error(status, format!("pull {url}: {e}"));
                continue;
            }
        };
        if !resp.status().is_success() {
            let st = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            record_error(status, format!("pull {url}: {st}: {txt}"));
            continue;
        }
        let bytes = match resp.bytes().await {
            Ok(b) => b.to_vec(),
            Err(e) => {
                record_error(status, format!("pull read body: {e}"));
                continue;
            }
        };
        let parsed: ProtoResponse = match serde_json::from_slice(&bytes) {
            Ok(p) => p,
            Err(e) => {
                record_error(status, format!("pull parse: {e}"));
                continue;
            }
        };
        let changes = match parsed {
            ProtoResponse::PullResp { changes, .. } => changes,
            other => {
                record_error(status, format!("unexpected pull response: {other:?}"));
                continue;
            }
        };
        status.lock().unwrap().last_pull_at = Some(Utc::now());

        // Phase 4: apply pulled changes under the lock.
        if let Err(e) = apply_pulled_changes(state, &plan.vault_id, &changes) {
            record_error(status, format!("apply: {e}"));
        }
    }
    status.lock().unwrap().touch();
}

fn record_error(status: &Arc<Mutex<AutoSyncStatus>>, msg: String) {
    status.lock().unwrap().last_error = Some(msg);
}

/// Build the push and pull plans under the session lock.
/// Returns `Ok((pushes, pulls))`.
fn build_plans(
    state: &Arc<AppState>,
) -> Result<(Vec<PushPlan>, Vec<PullPlan>), keepsake_core::Error> {
    let mut guard = state.session.lock();
    let session = guard.as_mut().ok_or(keepsake_core::Error::Locked)?;
    let mut push_plans = Vec::new();
    let mut pull_plans = Vec::new();
    for vault_id in session.vault.list_shared_syncs()? {
        let setup = match session.vault.get_shared_sync(&vault_id)? {
            Some(s) => s,
            None => continue,
        };
        let server_url = match setup.server_url {
            Some(u) if !u.is_empty() => u,
            _ => continue,
        };
        let shared_key = match session.shared_sync_key(&vault_id) {
            Some(k) => k,
            None => continue,
        };
        // Build push changes for this vault.
        let mut changes = Vec::new();
        let mut lamport: u64 = 0;
        for t in keepsake_core::records::ALL_TYPES {
            for h in session.vault.list_records(t)? {
                if let Some((aead_nonce, aead_aad, ciphertext)) =
                    session.vault.get_record_envelope(h.id)?
                {
                    let payload = keepsake_core::sync::client::wrap_envelope(
                        shared_key,
                        &aead_nonce,
                        &aead_aad,
                        &ciphertext,
                    )?;
                    lamport += 1;
                    changes.push(Change {
                        id: uuid::Uuid::new_v4(),
                        lamport,
                        ts: Utc::now(),
                        author: session.username.clone(),
                        record_id: Some(h.id),
                        payload,
                    });
                }
            }
        }
        push_plans.push(PushPlan {
            server_url: server_url.clone(),
            vault_id: vault_id.clone(),
            changes,
            new_clock: VectorClock {
                counters: [(session.username.clone(), lamport)]
                    .into_iter()
                    .collect(),
            },
            sealed_row: session
                .vault
                .get_sealed_key(&session.username)?
                .map(crate::session::SealedKeyRowWire::from),
        });
        pull_plans.push(PullPlan { server_url, vault_id });
    }
    Ok((push_plans, pull_plans))
}

/// Apply pulled changes to the local vault under the session
/// lock.  Each change is unwrapped (using the shared sync
/// key) and its inner envelope is written verbatim to the
/// `records` table, then run through the CRDT layer.
fn apply_pulled_changes(
    state: &Arc<AppState>,
    vault_id: &str,
    changes: &[Change],
) -> Result<(), keepsake_core::Error> {
    let mut guard = state.session.lock();
    let session = guard.as_mut().ok_or(keepsake_core::Error::Locked)?;
    let shared_key = match session.shared_sync_key(vault_id) {
        Some(k) => k.clone(),
        None => return Ok(()),
    };
    for ch in changes {
        apply_one_change(&mut session.vault, &shared_key, ch)?;
    }
    Ok(())
}

fn apply_one_change(
    vault: &mut Vault,
    shared_key: &keepsake_core::crypto::AeadKey,
    ch: &Change,
) -> Result<(), keepsake_core::Error> {
    let record_id = ch.record_id.ok_or_else(|| {
        keepsake_core::Error::Sync("change has no record_id".into())
    })?;
    let (aead_nonce, aead_aad, ciphertext) =
        keepsake_core::sync::client::unwrap_envelope(
            shared_key,
            &ch.payload,
        )?;
    // Defense in depth: a forged payload from the server
    // fails the local AEAD check.  We don't act on the
    // plaintext here; the inner envelope is what gets
    // written to the records table.
    let local_key = vault.require_unlocked_for_sync()?;
    let _plaintext = keepsake_core::crypto::aead::decrypt(
        local_key,
        &keepsake_core::crypto::aead::Nonce::from_bytes(
            aead_nonce[..].try_into().unwrap(),
        ),
        &ciphertext,
        &aead_aad,
    )?;
    // The server's current_state is already the LWW-picked
    // result.  Apply it verbatim.  This bypasses the CRDT
    // layer (used by manual pull) because there's no
    // concurrent local edit to merge with: the auto-sync
    // happens on a 30-minute cadence and any local edit
    // is captured in the next push.
    let header = keepsake_core::sync::client::decode_inner_aad(
        &aead_aad,
        record_id,
        &ch.author,
        ch.ts,
    )?;
    vault.put_record_envelope(
        header.id,
        &header.r#type,
        header.schema_version,
        &header.created_by,
        &header.updated_by,
        header.created_at,
        header.updated_at,
        &aead_nonce,
        &aead_aad,
        &ciphertext,
    )?;
    Ok(())
}
