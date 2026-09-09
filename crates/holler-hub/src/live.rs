//! The hub's in-memory table of **live** (authenticated) circuits (issue
//! #182), extended (issue #185) with the small amount of per-body state
//! `query/*` needs: the harnesses a body advertised in its hello, which of
//! those the hub has since **confirmed** with a real `query/support` probe,
//! its last-reported session count, and a generic `query/*` forward so `hub
//! query TARGET …` can reach a specific live body from the control-socket
//! task rather than the connection task that owns its socket.
//!
//! This is deliberately still a minimal map, not the full registry — issue
//! #184 (hub registry, supersede-on-reauth, connection hygiene) is the
//! story that replaces it with the real roster-backed struct. The fields
//! added here (`harnesses_advertised`/`harnesses_confirmed`/`session_count`/
//! label lookup) are exactly the "minimal local hub-state lookup" the #185
//! issue authorizes building ahead of #184 landing, documented for
//! reconciliation once #184 merges: its registry should absorb this state
//! (or a superset of it) rather than #185 re-inventing a second one.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::{oneshot, Mutex};

use holler_proto::{PingAck, WireError};

/// One outstanding command a control-socket task can ask a live connection's
/// own task to perform. The connection task (which alone owns the socket)
/// executes it and answers on `reply`.
pub enum LiveCommand {
    /// Send a `circuit/ping` request to the body and report its answer (or
    /// that the body did not answer within the caller's own timeout budget —
    /// the connection task does not itself time out; the caller does, via
    /// `tokio::time::timeout` around the receive).
    Ping { reply: oneshot::Sender<PingAck> },
    /// Forward one `query/*` request to the body and report its answer
    /// (issue #185: `hub query TARGET CMD…`). `Ok(Err(..))` carries a wire
    /// error the body itself answered with (e.g. `-32006 unknown_feature`
    /// from `query/support`); the outer `Err` on the receiver's side (a
    /// dropped sender, or the caller's own timeout) means the body never
    /// answered at all.
    Query {
        method: String,
        params: Option<Value>,
        reply: oneshot::Sender<Result<Value, WireError>>,
    },
}

/// The per-body state a control-socket task or the confirmation pass reads or
/// updates. Kept separate from [`LiveHandle`]'s `Clone` fields so many
/// cloned handles (one per accessor) share one source of truth via the `Arc`.
#[derive(Default)]
struct LiveState {
    harnesses_advertised: Vec<String>,
    harnesses_confirmed: BTreeSet<String>,
    session_count: u32,
}

/// A live, authenticated circuit's command channel — how another task (the
/// control socket) reaches this specific connection.
#[derive(Clone)]
pub struct LiveHandle {
    pub client_id: String,
    pub hostname: String,
    pub token_id: String,
    tx: tokio::sync::mpsc::UnboundedSender<LiveCommand>,
    state: Arc<Mutex<LiveState>>,
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

    /// Forward one `query/*` request to this body and await its answer,
    /// bounded by `timeout` (issue #185: `hub query TARGET CMD…`). The outer
    /// `Option` is `None` when the body never answered in time (or the
    /// connection task is gone) — the caller reports that as `-32004
    /// not_connected`, same as `ping`. The inner `Result` is the body's own
    /// answer: `Ok` a result document, `Err` a wire error it raised (e.g.
    /// `query/support`'s `-32006 unknown_feature`), forwarded verbatim.
    pub async fn query(
        &self,
        method: &str,
        params: Option<Value>,
        timeout: std::time::Duration,
    ) -> Option<Result<Value, WireError>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self
            .tx
            .send(LiveCommand::Query { method: method.to_string(), params, reply: reply_tx })
            .is_err()
        {
            return None;
        }
        tokio::time::timeout(timeout, reply_rx).await.ok()?.ok()
    }
}

/// The hub-wide table of live circuits, keyed by `client_id`. Shared (an
/// `Arc<Mutex<..>>`) across the accept loop's spawned connection tasks and the
/// control-socket tasks that answer `hub token ping` / `control/status` /
/// `hub query`.
#[derive(Clone, Default)]
pub struct Registry {
    inner: Arc<Mutex<HashMap<String, LiveHandle>>>,
}

/// The result of resolving a `hub query TARGET …` target against the live
/// registry (issue #185, ADR 0003's `token id | client id | label` order).
pub enum TargetLookup {
    /// Exactly one live body matched.
    Found(LiveHandle),
    /// No live body matched `target` at all.
    NotConnected,
    /// More than one live body matched (a hostname/label collision — should
    /// not happen once #184's uniqueness is enforced, but the CLI's own exit
    /// 2 contract needs a distinct outcome here rather than silently picking
    /// one).
    Ambiguous,
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
            client_id: client_id.to_string(),
            hostname: hostname.to_string(),
            token_id: token_id.to_string(),
            tx,
            state: Arc::new(Mutex::new(LiveState::default())),
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

    /// Resolve a `hub query TARGET …` target against the live registry
    /// (issue #185): `target` may name a token id, a client id, or a
    /// hostname/label (a `label/session` form's label part). Exactly one
    /// match is [`TargetLookup::Found`]; zero is `NotConnected`; more than
    /// one is `Ambiguous`.
    pub async fn find_target(&self, target: &str) -> TargetLookup {
        let label = target.split('/').next().unwrap_or(target);
        let mut matches: Vec<LiveHandle> = self
            .inner
            .lock()
            .await
            .values()
            .filter(|h| h.token_id == target || h.client_id == target || h.hostname == label)
            .cloned()
            .collect();
        match matches.len() {
            0 => TargetLookup::NotConnected,
            1 => TargetLookup::Found(matches.remove(0)),
            _ => TargetLookup::Ambiguous,
        }
    }

    /// Record the harness ids a body advertised in its `circuit/hello`
    /// (issue #185's `harnesses_known`). A no-op if `client_id` has already
    /// been removed (a race between hello and a drop — the registry entry
    /// wins whichever landed first; harmless either way).
    pub async fn set_harnesses_advertised(&self, client_id: &str, harnesses: Vec<String>) {
        if let Some(h) = self.inner.lock().await.get(client_id) {
            h.state.lock().await.harnesses_advertised = harnesses;
        }
    }

    /// Record that a live body's `query/support {harness}` probe answered
    /// `ok:true` (issue #185's confirmation pass) — the harness moves from
    /// merely "known" (some body advertised it) to "confirmed" (a live body
    /// actually proved it).
    pub async fn confirm_harness(&self, client_id: &str, harness: &str) {
        if let Some(h) = self.inner.lock().await.get(client_id) {
            h.state.lock().await.harnesses_confirmed.insert(harness.to_string());
        }
    }

    /// Record a body's most recently reported session count (from its
    /// `session/presence`) — summed across bodies for `hub status`'s
    /// `sessions` field.
    pub async fn set_session_count(&self, client_id: &str, n: u32) {
        if let Some(h) = self.inner.lock().await.get(client_id) {
            h.state.lock().await.session_count = n;
        }
    }

    /// The union of every live body's advertised harness ids, sorted
    /// (`hub status`'s `harnesses_known`).
    pub async fn harnesses_known(&self) -> Vec<String> {
        let mut set = BTreeSet::new();
        for h in self.inner.lock().await.values() {
            set.extend(h.state.lock().await.harnesses_advertised.iter().cloned());
        }
        set.into_iter().collect()
    }

    /// Every confirmed harness id, each with the (sorted) list of hostnames
    /// that confirmed it (`hub status`/`caps`'s `harnesses_confirmed`).
    pub async fn harnesses_confirmed(&self) -> Vec<holler_proto::ConfirmedHarness> {
        let mut by_id: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for h in self.inner.lock().await.values() {
            let state = h.state.lock().await;
            for id in &state.harnesses_confirmed {
                by_id.entry(id.clone()).or_default().insert(h.hostname.clone());
            }
        }
        by_id
            .into_iter()
            .map(|(id, bodies)| holler_proto::ConfirmedHarness { id, bodies: bodies.into_iter().collect() })
            .collect()
    }

    /// The sum of every live body's last-reported session count (`hub
    /// status`'s `sessions`).
    pub async fn total_sessions(&self) -> u32 {
        let mut total = 0u32;
        for h in self.inner.lock().await.values() {
            total = total.saturating_add(h.state.lock().await.session_count);
        }
        total
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #185
mod tests {
    use super::*;

    #[tokio::test]
    async fn find_target_matches_by_token_id_client_id_or_label() {
        let registry = Registry::new();
        let mut rx = registry.insert("cli_1", "kiwi", "tok_1").await;

        assert!(matches!(registry.find_target("tok_1").await, TargetLookup::Found(_)));
        assert!(matches!(registry.find_target("cli_1").await, TargetLookup::Found(_)));
        assert!(matches!(registry.find_target("kiwi").await, TargetLookup::Found(_)));
        // A `label/session` form resolves by its label segment.
        assert!(matches!(registry.find_target("kiwi/alpha").await, TargetLookup::Found(_)));
        assert!(matches!(registry.find_target("nobody").await, TargetLookup::NotConnected));

        rx.close();
    }

    /// Two live bodies sharing the same hostname (a collision issue #184's
    /// uniqueness enforcement is what actually prevents in practice — this
    /// registry alone does not) resolve as [`TargetLookup::Ambiguous`], not a
    /// silently-picked one.
    #[tokio::test]
    async fn ambiguous_target_is_reported_as_ambiguous() {
        let registry = Registry::new();
        let mut rx1 = registry.insert("cli_1", "kiwi", "tok_1").await;
        let mut rx2 = registry.insert("cli_2", "kiwi", "tok_2").await;

        assert!(matches!(registry.find_target("kiwi").await, TargetLookup::Ambiguous));
        // Each body's own token/client id still resolves unambiguously.
        assert!(matches!(registry.find_target("tok_1").await, TargetLookup::Found(_)));

        rx1.close();
        rx2.close();
    }

    #[tokio::test]
    async fn confirm_harness_and_advertised_are_independent() {
        let registry = Registry::new();
        let mut rx = registry.insert("cli_1", "kiwi", "tok_1").await;
        registry.set_harnesses_advertised("cli_1", vec!["opencode".to_string(), "claude".to_string()]).await;
        assert_eq!(registry.harnesses_known().await, vec!["claude".to_string(), "opencode".to_string()]);
        assert!(registry.harnesses_confirmed().await.is_empty(), "advertising alone confirms nothing");

        registry.confirm_harness("cli_1", "opencode").await;
        let confirmed = registry.harnesses_confirmed().await;
        assert_eq!(confirmed.len(), 1);
        assert_eq!(confirmed[0].id, "opencode");
        assert_eq!(confirmed[0].bodies, vec!["kiwi".to_string()]);

        rx.close();
    }

    #[tokio::test]
    async fn session_count_sums_across_bodies() {
        let registry = Registry::new();
        let mut rx1 = registry.insert("cli_1", "kiwi", "tok_1").await;
        let mut rx2 = registry.insert("cli_2", "mango", "tok_2").await;
        registry.set_session_count("cli_1", 2).await;
        registry.set_session_count("cli_2", 3).await;
        assert_eq!(registry.total_sessions().await, 5);
        rx1.close();
        rx2.close();
    }
}
