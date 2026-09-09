//! The hub's in-memory table of **live** (authenticated) circuits (issue
//! #182). Keyed by `client_id`, it is what makes `hub token ping` possible —
//! the control socket needs a way to reach a specific body's open WebSocket
//! from a different task entirely.
//!
//! This is deliberately a minimal map, not a registry: the full peer/session
//! bookkeeping (hostnames, harnesses, roster rows) is the registry story's
//! (#184) struct, which this is explicitly scoped to be replaced by (the
//! issue's own words: "a minimal map is fine here and is replaced there").
//! Nothing here reaches into a body's sessions or roster row — it only knows
//! how to find a live socket and ask it to answer one `circuit/ping`.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{oneshot, Mutex};

use holler_proto::PingAck;

/// One outstanding command a control-socket task can ask a live connection's
/// own task to perform. The connection task (which alone owns the socket)
/// executes it and answers on `reply`.
pub enum LiveCommand {
    /// Send a `circuit/ping` request to the body and report its answer (or
    /// that the body did not answer within the caller's own timeout budget —
    /// the connection task does not itself time out; the caller does, via
    /// `tokio::time::timeout` around the receive).
    Ping { reply: oneshot::Sender<PingAck> },
}

/// A live, authenticated circuit's command channel — how another task (the
/// control socket) reaches this specific connection.
#[derive(Clone)]
pub struct LiveHandle {
    pub hostname: String,
    pub token_id: String,
    tx: tokio::sync::mpsc::UnboundedSender<LiveCommand>,
}

impl LiveHandle {
    /// Ask the live connection to ping its body and await the answer, bounded
    /// by `timeout`. `None` covers every way this can fail to produce an
    /// answer in time: the connection task is gone, or the body never
    /// replied — the caller (the CLI's `hub token ping`) reports both as
    /// `not_connected`, matching the spec's fail-closed `-32004`.
    pub async fn ping(&self, timeout: std::time::Duration) -> Option<PingAck> {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send(LiveCommand::Ping { reply: reply_tx }).is_err() {
            return None;
        }
        tokio::time::timeout(timeout, reply_rx).await.ok()?.ok()
    }
}

/// The hub-wide table of live circuits, keyed by `client_id`. Shared (an
/// `Arc<Mutex<..>>`) across the accept loop's spawned connection tasks and the
/// control-socket tasks that answer `hub token ping` / `control/status`.
#[derive(Clone, Default)]
pub struct Registry {
    inner: Arc<Mutex<HashMap<String, LiveHandle>>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a newly-authenticated circuit, returning the receiver its
    /// connection task drains for [`LiveCommand`]s. Replaces (and thereby
    /// supersedes) a prior live handle for the same `client_id` — a second
    /// `body run` reconnecting under the same identity naturally displaces
    /// the stale one rather than leaving two entries.
    pub async fn insert(
        &self,
        client_id: &str,
        hostname: &str,
        token_id: &str,
    ) -> tokio::sync::mpsc::UnboundedReceiver<LiveCommand> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let handle = LiveHandle {
            hostname: hostname.to_string(),
            token_id: token_id.to_string(),
            tx,
        };
        self.inner.lock().await.insert(client_id.to_string(), handle);
        rx
    }

    /// Drop a circuit's live entry (the connection ended). A no-op if it was
    /// already replaced or removed (never an error — teardown is best-effort).
    pub async fn remove(&self, client_id: &str) {
        self.inner.lock().await.remove(client_id);
    }

    /// The number of currently-live circuits (`hub status`'s `clients`).
    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    /// Whether the registry currently holds no live circuits.
    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }

    /// Find the live handle for the body currently bound to `token_id` (`hub
    /// token ping ID` looks a body up by token, not by client id).
    pub async fn find_by_token(&self, token_id: &str) -> Option<LiveHandle> {
        self.inner
            .lock()
            .await
            .values()
            .find(|h| h.token_id == token_id)
            .cloned()
    }
}
