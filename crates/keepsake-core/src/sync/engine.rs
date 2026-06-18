//! Sync engine.  In v1 this is a thin coordinator that batches
//! local changes and ships them through a [`Transport`].  The
//! CRDT-merge logic is added in a follow-up commit; the public
//! API is stable so the CRDT implementation can drop in without
//! breaking callers.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Result;
use crate::transport::Transport;

use super::protocol::{Request, Response};
use super::VectorClock;

/// A single local change.  In v1 this is a thin wrapper around
/// the protocol type; in a follow-up commit the merge logic
/// moves here.
pub type Change = super::Change;

/// The sync engine.  Holds a vector clock and a queue of pending
/// changes.  Thread-safe via interior mutability.
pub struct Engine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    clock: Mutex<VectorClock>,
    pending: Mutex<Vec<Change>>,
    transport: Box<dyn Transport>,
}

impl Engine {
    /// Construct a new engine with the given transport.
    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self {
            inner: Arc::new(EngineInner {
                clock: Mutex::new(VectorClock::new()),
                pending: Mutex::new(Vec::new()),
                transport,
            }),
        }
    }

    /// Record a locally-produced change.  Bumps the local clock
    /// and queues the change for the next push.
    pub fn record(&self, change: Change) -> Result<()> {
        let mut clock = self.inner.clock.lock().expect("clock mutex poisoned");
        clock.bump(&change.author);
        self.inner.pending.lock().expect("pending mutex poisoned").push(change);
        Ok(())
    }

    /// Push all pending changes to the server.  Returns the
    /// number of changes pushed.
    pub async fn push(&self) -> Result<usize> {
        let (new_clock, batch) = {
            let mut pending = self.inner.pending.lock().expect("pending mutex poisoned");
            if pending.is_empty() {
                return Ok(0);
            }
            let clock = self.inner.clock.lock().expect("clock mutex poisoned").clone();
            let batch = std::mem::take(&mut *pending);
            (clock, batch)
        };
        let n = batch.len();
        let req = Request::Push {
            new_clock: new_clock.clone(),
            changes: batch,
        };
        match self.inner.transport.round_trip(&req).await? {
            Response::Ok { .. } => {
                *self.inner.clock.lock().expect("clock mutex poisoned") = new_clock;
                Ok(n)
            }
            other => Err(crate::Error::Sync(format!(
                "unexpected push response: {other:?}"
            ))),
        }
    }

    /// Pull all changes since the engine's last-known clock.
    pub async fn pull(&self) -> Result<Vec<Change>> {
        let since = self.inner.clock.lock().expect("clock mutex poisoned").clone();
        let req = Request::Pull { since };
        match self.inner.transport.round_trip(&req).await? {
            Response::PullResp { changes, server_clock } => {
                self.inner.clock.lock().expect("clock mutex poisoned").merge(&server_clock);
                Ok(changes)
            }
            other => Err(crate::Error::Sync(format!(
                "unexpected pull response: {other:?}"
            ))),
        }
    }

    /// Current vector clock.
    pub fn clock(&self) -> VectorClock {
        self.inner.clock.lock().expect("clock mutex poisoned").clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::Transport;

    struct FakeTransport {
        responses: std::sync::Mutex<Vec<Response>>,
    }

    #[async_trait]
    impl Transport for FakeTransport {
        async fn round_trip(&self, _req: &Request) -> Result<Response> {
            Ok(self.responses.lock().unwrap().remove(0))
        }
    }

    #[tokio::test]
    async fn push_sends_pending() {
        let transport: Box<dyn Transport> = Box::new(FakeTransport {
            responses: std::sync::Mutex::new(vec![Response::Ok { message: None }]),
        });
        let engine = Engine::new(transport);
        engine.record(Change {
            id: Uuid::new_v4(),
            lamport: 1,
            ts: chrono::Utc::now(),
            author: "justin".into(),
            record_id: None,
            payload: vec![1, 2, 3],
        }).unwrap();
        let n = engine.push().await.unwrap();
        assert_eq!(n, 1);
    }
}
