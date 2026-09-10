//! The hub's in-memory table of **live** (authenticated) circuits (issue
//! #182), extended (issue #185) with the small amount of per-body state
//! `query/*` needs: the harnesses a body advertised in its hello, which of
//! those the hub has since **confirmed** with a real `query/support` probe,
//! its last-reported session count, and a generic `query/*` forward so `hub
//! query TARGET …` can reach a specific live body from the control-socket
//! task rather than the connection task that owns its socket. Issue #190
//! (`say`) adds a second kind of per-body cache — the body's last-reported
//! `session/presence` sessions — plus a `Say` command alongside `Ping`/
//! `Query`.
//!
//! This is deliberately still a minimal map, not the full registry — issue
//! #184 (hub registry, supersede-on-reauth, connection hygiene) is the
//! story that replaces it with the real roster-backed struct. The fields
//! added here (`harnesses_advertised`/`harnesses_confirmed`/`session_count`/
//! label lookup, and #190's presence cache) are exactly the "minimal local
//! hub-state lookup" #185/#190 authorize building ahead of #184/#186
//! landing, documented for reconciliation once those merge: the real
//! registry/roster should absorb this state (or a superset of it) rather
//! than reinventing a second one.
//!
//! # Decisions I made (issue #190)
//!
//! - **Neither #185 (query) nor #186 (roster) had landed** when this story
//!   was *started* (checked via `gh pr list`/`gh issue view`: both issues
//!   open, no branch, no PR); #185 landed while this story was in flight, so
//!   this file now carries both its `LiveState`/`TargetLookup`/`find_target`
//!   machinery and this story's own presence cache side by side. `say`'s own
//!   spec needs two things a real roster would give it — "resolve a session
//!   name to a live connection" and "read the session's current state for
//!   the busy check" — so [`LiveHandle`] also carries the **last
//!   `session/presence` this body sent**, cached here as it arrives (see
//!   [`Registry::update_presence`], called from `circuit::handle_inbound`).
//!   [`Registry::resolve_session`] is the ADR 0003 bare/label name
//!   resolution `say` needs, built directly against this cache.
//!   **Reconcile when #186 lands**: that story's own roster row is the
//!   intended long-term home for both #185's `LiveState` and this story's
//!   presence cache — either should be replaced outright by the roster or
//!   folded into it, whichever #186's own shape turns out to want.
//! - **One in-flight `say` per session prompt_id, tracked in a map (not a
//!   single `Option`) in `circuit::session_loop`.** A `--queue`d
//!   `session/prompt` is forwarded to the body immediately (the body itself
//!   queues it, not the hub), so two (or more) `say`s are routinely
//!   concurrent on one connection — confirmed the hard way while building
//!   this story's own `say_queue_appends_and_runs_after_turn` test.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::{oneshot, Mutex};

use holler_proto::{Message, PingAck, SessionAd, WireError};

/// `component=registry` debug event: a circuit registered or removed
/// (issue #197 — nothing in this module logged at all before this; the hub's
/// live-body registry was completely silent).
fn log_registry(method: &'static str, client_id: &str, hostname: Option<&str>) {
    let mut fields = vec![("client_id", client_id.to_string())];
    if let Some(h) = hostname {
        fields.push(("hostname", h.to_string()));
    }
    holler_proto::log::emit(&holler_proto::log::Event {
        component: holler_proto::log::Component::Registry,
        severity: holler_proto::log::Severity::Debug,
        direction: holler_proto::log::Direction::Local,
        method,
        id: None,
        peer: None,
        fields,
        frame: None,
    });
}

/// `(hostname, that body's last-known sessions)`, keyed by `client_id` — the
/// shape [`Registry`]'s `offline` field holds (named here to keep the
/// field's own type simple enough for clippy's `type_complexity` gate).
type OfflineSnapshots = HashMap<String, (String, Vec<SessionAd>)>;

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
    /// Send a `session/prompt` request to the body (issue #190) and report
    /// how it resolved. `request_id` is the `h-…` id the connection task
    /// sends the request under and correlates `session/update`s by
    /// `prompt_id` against.
    Say {
        request_id: String,
        session: String,
        // Boxed: `Message` is the largest field by far (it carries a
        // `Vec<Part>`), and `clippy::large_enum_variant` compares this
        // variant against `Ping`'s single-`oneshot::Sender` payload.
        message: Box<Message>,
        queue: bool,
        /// `interrupt SESSION TEXT` (issue #191): mark this `session/prompt`
        /// as a redirect (`Prompt.replace`) rather than a plain `say` — sent
        /// only after the hub has itself observed the matching cancel's
        /// `{applied:true}`.
        replace: bool,
        reply: oneshot::Sender<SayReply>,
    },
    /// Issue #192's `control/test_drop` test hook: forcibly end this
    /// connection to simulate an abrupt network drop (never a clean WS
    /// close), so a test can observe the hub's own reconnect contract
    /// without waiting out the liveness timeout or killing the whole body
    /// process. The connection task fails every pending `say`/`cancel` with
    /// `connection_lost`, marks the token's roster rows `reconnecting`, acks
    /// on `reply`, then ends its own loop — the socket itself closes
    /// abruptly (no WS close frame) once the connection task's owned
    /// sink/stream go out of scope, exactly like a real dropped TCP
    /// connection would look to the body on the other end.
    Drop { reply: oneshot::Sender<()> },
    /// Issue #184's supersede-on-reauth: a fresh socket just re-authenticated
    /// for the same token this connection is bound to. The connection sends
    /// `circuit/superseded` to its body, closes the socket with WS code
    /// **1000**, and ends its own loop — the new socket then takes the
    /// token's slot. Never touches the roster (the new connection is about
    /// to (re)claim it).
    Supersede { reply: oneshot::Sender<()> },
    /// Issue #184's `hub token revoke`: the operator revoked this
    /// connection's token. The connection closes the socket with WS code
    /// **1008** (policy violation), marks the roster row `gone` immediately
    /// (not `reconnecting` — a revoked token can never reconnect), acks on
    /// `reply`, then ends its own loop.
    Revoke { reply: oneshot::Sender<()> },
}

/// One outstanding `session/cancel` (issue #191): kept separate from
/// [`LiveCommand`] so it can travel the connection's own **priority**
/// channel — see [`LiveHandle::cancel`]/[`crate::circuit`]'s doc for why a
/// cancel must never queue behind a `Say`/`Query`/`Ping` command already
/// sitting in the normal `cmd_rx`.
pub struct CancelCommand {
    pub request_id: String,
    pub session: String,
    pub reply: oneshot::Sender<CancelReply>,
}

/// How a [`CancelCommand`] resolved.
pub enum CancelReply {
    /// The body answered `{applied:true}`.
    Applied,
    /// The body answered with a JSON-RPC error.
    Refused(WireError),
    /// The socket dropped before a response arrived.
    ConnectionLost,
}

/// The per-body state a control-socket task or the confirmation pass reads or
/// updates. Kept separate from [`LiveHandle`]'s `Clone` fields so many
/// cloned handles (one per accessor) share one source of truth via the `Arc`.
#[derive(Default)]
struct LiveState {
    harnesses_advertised: Vec<String>,
    harnesses_confirmed: BTreeSet<String>,
    session_count: u32,
    /// The most recently measured circuit RTT (issue #191): updated by
    /// `hub token ping` and by `interrupt`'s own cancel round trip. `None`
    /// until this circuit has measured one — the ack-timeout floor
    /// (`interrupt::ack_timeout`) covers that case.
    last_rtt_ms: Option<u64>,
}

/// One streamed `session/update` the connection task observed while a `say`
/// was in flight — kept so the TalkLog can record it (issue #190 spec: "per
/// update `{ts, prompt_id, seq, parts}`") even though the control-socket
/// exchange itself only ever returns the final result.
#[derive(Debug, Clone)]
pub struct SeenUpdate {
    pub ts: String,
    pub seq: u64,
    pub parts: Vec<holler_proto::Part>,
}

/// How a [`LiveCommand::Say`] resolved.
pub enum SayReply {
    /// The turn ran and ended; `updates` is every `session/update` observed,
    /// in arrival order.
    Result { message: Box<Message>, stop_reason: String, state: String, updates: Vec<SeenUpdate> },
    /// The body answered with a JSON-RPC error (`-32009 session_busy`,
    /// `-32003 unknown_session`, …) — carried verbatim so the caller can map
    /// it to the spec's exact CLI wording.
    Refused(holler_proto::WireError),
    /// The socket dropped before a response arrived (docs/issue #190:
    /// `-32008 connection_lost`, "ask again" — never reported as
    /// `no live hub reachable`).
    ConnectionLost,
}

/// A live, authenticated circuit's command channel — how another task (the
/// control socket) reaches this specific connection.
#[derive(Clone)]
pub struct LiveHandle {
    pub client_id: String,
    pub hostname: String,
    pub token_id: String,
    /// The transport peer address (`ip:port`) this connection was accepted
    /// from (issue #184: captured at accept, reported in every connection
    /// log line and in `hub status --json`'s `clients_detail[].peer`).
    pub peer: String,
    /// The moment this circuit was registered (issue #184's `connected_at`,
    /// epoch milliseconds).
    pub connected_at: u64,
    /// This registry entry's monotonic generation stamp (issue #184). Not
    /// exposed on the wire; used by [`Registry::remove_if_current`] so a
    /// superseded connection's own teardown never evicts the entry that
    /// replaced it (see that fn's doc for the race it closes).
    seq: u64,
    tx: tokio::sync::mpsc::UnboundedSender<LiveCommand>,
    /// The priority channel a `session/cancel` travels instead of `tx`
    /// (issue #191) — see [`CancelCommand`]'s own doc.
    cancel_tx: tokio::sync::mpsc::UnboundedSender<CancelCommand>,
    state: Arc<Mutex<LiveState>>,
    /// The last `session/presence` this body sent (issue #190's roster
    /// stand-in — see the module doc). `None` until its first presence.
    presence: Arc<Mutex<Option<Vec<SessionAd>>>>,
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

    /// Ask the live connection to run one `say` turn and await its outcome,
    /// bounded by `timeout` (the CLI's own `--timeout`, default 600s).
    /// `None` means the connection task is gone (the socket dropped) or did
    /// not answer within `timeout` at all — the caller maps that to the
    /// spec's `no reply from … within Ns` / `connection_lost` wording.
    pub async fn say(
        &self,
        request_id: String,
        session: String,
        message: Box<Message>,
        queue: bool,
        replace: bool,
        timeout: std::time::Duration,
    ) -> Option<SayReply> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let cmd = LiveCommand::Say { request_id, session, message, queue, replace, reply: reply_tx };
        if self.tx.send(cmd).is_err() {
            return None;
        }
        tokio::time::timeout(timeout, reply_rx).await.ok()?.ok()
    }

    /// Ask the live connection to send `session/cancel` for `session` over
    /// its **priority** channel and await the body's `{applied:true}` (or
    /// refusal), bounded by `timeout` (`interrupt::ack_timeout`'s RTT-scaled
    /// value). `None` means no answer arrived within `timeout` — the
    /// caller reports the spec's own "interrupt sent but not confirmed"
    /// wording, never a bare `not_connected` (the connection may well still
    /// be alive; the body is just slow to confirm).
    pub async fn cancel(
        &self,
        request_id: String,
        session: String,
        timeout: std::time::Duration,
    ) -> Option<CancelReply> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let cmd = CancelCommand { request_id, session, reply: reply_tx };
        if self.cancel_tx.send(cmd).is_err() {
            return None;
        }
        tokio::time::timeout(timeout, reply_rx).await.ok()?.ok()
    }

    /// The last measured circuit RTT (issue #191's ack-timeout scale), if
    /// any has been recorded yet for this body.
    pub async fn last_rtt(&self) -> Option<std::time::Duration> {
        self.state.lock().await.last_rtt_ms.map(std::time::Duration::from_millis)
    }

    /// Record a freshly measured RTT for this body (`hub token ping`'s own
    /// measurement, or `interrupt`'s own cancel round trip) — the input to
    /// the *next* `interrupt`'s ack-timeout scale.
    pub async fn record_rtt(&self, rtt: std::time::Duration) {
        self.state.lock().await.last_rtt_ms = Some(u64::try_from(rtt.as_millis()).unwrap_or(u64::MAX));
    }

    /// The last presence this body reported, if any yet.
    pub async fn presence(&self) -> Option<Vec<SessionAd>> {
        self.presence.lock().await.clone()
    }

    /// Issue #192's `control/test_drop` test hook: ask the live connection to
    /// forcibly end itself (see [`LiveCommand::Drop`]'s own doc), waiting up
    /// to 2s for its ack. `false` means the connection task was already gone
    /// (a send failure) or never acked within the wait — either way the
    /// caller should treat the body as no longer live.
    pub async fn force_drop(&self) -> bool {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send(LiveCommand::Drop { reply: reply_tx }).is_err() {
            return false;
        }
        tokio::time::timeout(std::time::Duration::from_secs(2), reply_rx).await.is_ok()
    }

    /// Issue #184's supersede-on-reauth: ask this (old) connection to notify
    /// its body of `circuit/superseded`, close with WS code 1000, and end its
    /// own loop. Waits up to 2s for the ack; `false` means the connection was
    /// already gone (a send failure) or never acked in time — either way the
    /// caller proceeds to register the new connection regardless (best
    /// effort: a superseded body that never got the notification still loses
    /// its slot the moment the new connection registers).
    pub async fn supersede(&self) -> bool {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send(LiveCommand::Supersede { reply: reply_tx }).is_err() {
            return false;
        }
        tokio::time::timeout(std::time::Duration::from_secs(2), reply_rx).await.is_ok()
    }

    /// Issue #184's `hub token revoke`: ask this connection to close with WS
    /// code 1008 and end its own loop, marking the roster row `gone` at once.
    /// Waits up to 2s for the ack; `false` means the connection was already
    /// gone or never acked in time (the caller reports the revoke as applied
    /// either way — the token store side already took effect).
    pub async fn revoke(&self) -> bool {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send(LiveCommand::Revoke { reply: reply_tx }).is_err() {
            return false;
        }
        tokio::time::timeout(std::time::Duration::from_secs(2), reply_rx).await.is_ok()
    }
}

/// The hub-wide table of live circuits, keyed by `client_id`. Shared (an
/// `Arc<Mutex<..>>`) across the accept loop's spawned connection tasks and the
/// control-socket tasks that answer `hub token ping` / `control/status` /
/// `hub query` / `control/say`.
#[derive(Clone, Default)]
pub struct Registry {
    inner: Arc<Mutex<HashMap<String, LiveHandle>>>,
    /// Monotonic counter stamping each registered entry's `seq` (issue #184)
    /// — see [`Registry::remove_if_current`]'s doc for why this exists.
    next_seq: Arc<std::sync::atomic::AtomicU64>,
    /// The last-known `(hostname, sessions)` of a body that has since
    /// disconnected, keyed by `client_id` — issue #190's own gap-filler so
    /// `say` can report `not_connected` ("this session exists, its body just
    /// isn't live right now") rather than `unknown_session` ("no such
    /// session was ever seen") for a session whose body dropped. A real
    /// roster (#186) would carry this as a persistent row regardless of live
    /// connection state; this is the minimal stand-in — cleared once a
    /// *fresh* connection re-registers under the same `client_id` (a
    /// reconnect naturally supersedes the stale offline snapshot with a live
    /// one again).
    offline: Arc<Mutex<OfflineSnapshots>>,
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
    /// connection task drains for [`LiveCommand`]s, (issue #191) the second,
    /// high-priority receiver it drains for [`CancelCommand`]s, and (issue
    /// #184) this entry's own generation `seq` — the connection task must
    /// hold onto `seq` and pass it to [`Registry::remove_if_current`] at
    /// teardown instead of the old unconditional [`Registry::remove`].
    /// Replaces (and thereby supersedes) a prior live handle for the same
    /// `client_id` — a second `body run` reconnecting under the same
    /// identity naturally displaces the stale one rather than leaving two
    /// entries.
    pub async fn insert(
        &self,
        client_id: &str,
        hostname: &str,
        token_id: &str,
        peer: &str,
    ) -> (
        tokio::sync::mpsc::UnboundedReceiver<LiveCommand>,
        tokio::sync::mpsc::UnboundedReceiver<CancelCommand>,
        u64,
    ) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (cancel_tx, cancel_rx) = tokio::sync::mpsc::unbounded_channel();
        let seq = self.next_seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let handle = LiveHandle {
            client_id: client_id.to_string(),
            hostname: hostname.to_string(),
            token_id: token_id.to_string(),
            peer: peer.to_string(),
            connected_at: now_millis_u64(),
            seq,
            tx,
            cancel_tx,
            state: Arc::new(Mutex::new(LiveState::default())),
            presence: Arc::new(Mutex::new(None)),
        };
        self.inner.lock().await.insert(client_id.to_string(), handle);
        // A fresh connection supersedes any stale offline snapshot for this
        // exact `client_id` — see the `offline` field's own doc.
        self.offline.lock().await.remove(client_id);
        log_registry("register", client_id, Some(hostname));
        (rx, cancel_rx, seq)
    }

    /// Drop a circuit's live entry (the connection ended) **unconditionally**
    /// — kept for callers that know they hold the sole reference to this
    /// slot (the test suite's own direct `Registry` unit tests below). Every
    /// production connection path uses [`Registry::remove_if_current`]
    /// instead, so a superseded connection's own (possibly-delayed) teardown
    /// can never evict the entry that replaced it.
    pub async fn remove(&self, client_id: &str) {
        let removed = self.inner.lock().await.remove(client_id);
        self.snapshot_offline(client_id, removed).await;
    }

    /// Remove `client_id`'s live entry **only if it is still the entry
    /// stamped `seq`** (issue #184). A connection's own teardown always
    /// calls this with the `seq` [`Registry::insert`] handed it, rather than
    /// the unconditional [`Registry::remove`]: supersede-on-reauth registers
    /// the *new* connection under the same `client_id` (a body's client id is
    /// stable across reconnects — it is minted once, at redeem) before the
    /// *old* connection's task has necessarily finished unwinding. If the old
    /// connection's teardown ran an unconditional `remove(client_id)` after
    /// the new one had already registered, it would silently evict the new,
    /// live entry out from under it. Comparing `seq` closes that race
    /// regardless of which task's teardown happens to run first: an entry
    /// whose `seq` no longer matches has already been superseded, so this is
    /// a no-op.
    pub async fn remove_if_current(&self, client_id: &str, seq: u64) {
        let mut inner = self.inner.lock().await;
        let is_current = inner.get(client_id).is_some_and(|h| h.seq == seq);
        let removed = if is_current { inner.remove(client_id) } else { None };
        drop(inner);
        self.snapshot_offline(client_id, removed).await;
    }

    /// Shared teardown tail for [`Registry::remove`]/[`Registry::remove_if_current`]:
    /// log the removal and snapshot the handle's last-known presence into
    /// `offline` (issue #190's `not_connected`-vs-`unknown_session`
    /// gap-filler — see that field's own doc). A no-op if there was nothing
    /// to remove (already superseded, or a race with a concurrent teardown).
    async fn snapshot_offline(&self, client_id: &str, removed: Option<LiveHandle>) {
        if let Some(handle) = removed {
            log_registry("remove", client_id, Some(&handle.hostname));
            if let Some(sessions) = handle.presence.lock().await.clone() {
                self.offline.lock().await.insert(client_id.to_string(), (handle.hostname.clone(), sessions));
            }
        }
    }

    /// One row per live circuit for `hub status --json`'s `clients_detail`
    /// (issue #184): `{token_id, client_id, hostname, peer, connected_at}`.
    /// (`clients` itself stays the plain live count other tests already
    /// depend on — this is an additive field, not a replacement.)
    pub async fn clients_detail(&self) -> Vec<serde_json::Value> {
        self.inner
            .lock()
            .await
            .values()
            .map(|h| {
                serde_json::json!({
                    "token_id": h.token_id,
                    "client_id": h.client_id,
                    "hostname": h.hostname,
                    "peer": h.peer,
                    "connected_at": h.connected_at,
                })
            })
            .collect()
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

    /// The token id currently bound to `client_id` (issue #186's roster keys
    /// its rows by token, but the live loop only knows the `client_id` the
    /// socket carries, so the presence handler resolves the token through this
    /// helper). `None` once the client has disconnected (the roster keeps its
    /// rows and lets the TTL age them out, so an absent live entry is not an
    /// error — just no token to attribute the presence to).
    pub async fn token_id_for_client(&self, client_id: &str) -> Option<String> {
        self.inner
            .lock()
            .await
            .get(client_id)
            .map(|h| h.token_id.clone())
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

    /// Overwrite one cached [`SessionAd`] row (matched by name) on the
    /// connection identified by `handle`'s `token_id` — `say`'s best-effort
    /// `last_turn` update on its own cache (see the `talk` module doc's
    /// "Decisions I made"). A no-op if the connection or the row is gone by
    /// the time this runs (e.g. it reconnected mid-turn); the next real
    /// `session/presence` overwrites the whole row anyway.
    pub async fn replace_session(&self, handle: &LiveHandle, updated: SessionAd) {
        let inner = self.inner.lock().await;
        let Some(live) = inner.values().find(|h| h.token_id == handle.token_id) else { return };
        let mut presence = live.presence.lock().await;
        if let Some(sessions) = presence.as_mut() {
            if let Some(row) = sessions.iter_mut().find(|s| s.name == updated.name) {
                *row = updated;
            }
        }
    }

    /// Cache the latest `session/presence` a body sent (issue #190's roster
    /// stand-in — see the module doc). Called from `circuit::handle_inbound`
    /// on every `session/presence` notification, keyed by that connection's
    /// `client_id`. A no-op if the client is not (or no longer) live.
    pub async fn update_presence(&self, client_id: &str, sessions: Vec<SessionAd>) {
        if let Some(handle) = self.inner.lock().await.get(client_id) {
            *handle.presence.lock().await = Some(sessions);
        }
    }

    /// Resolve a `say`/`interrupt` target name against every live body's
    /// cached presence (ADR 0003's bare/label rules, the minimal slice this
    /// story needs): a `<label>/<session>` name matches only that body's
    /// session; a bare name matches by session name across every live body.
    pub async fn resolve_session(&self, name: &holler_proto::RoutableName) -> ResolveOutcome {
        let inner = self.inner.lock().await;
        let mut matches: Vec<(LiveHandle, SessionAd)> = Vec::new();
        for handle in inner.values() {
            if name.has_label() && handle.hostname != name.label() {
                continue;
            }
            let Some(sessions) = handle.presence.lock().await.clone() else { continue };
            for ad in sessions {
                if ad.name == name.session() {
                    matches.push((handle.clone(), ad));
                }
            }
        }
        drop(inner);
        if matches.is_empty() {
            return if self.matches_offline(name).await {
                ResolveOutcome::NotConnected
            } else {
                ResolveOutcome::Unknown
            };
        }
        if matches.len() == 1 {
            // Guarded by the length check just above, so this never panics —
            // `Vec::remove` at a valid index, not an `.expect()`/`.unwrap()`
            // escape hatch on a fallible value.
            let (handle, ad) = matches.remove(0);
            return ResolveOutcome::Found(handle, Box::new(ad));
        }
        let candidates = matches.into_iter().map(|(h, ad)| format!("{}/{}", h.hostname, ad.name)).collect();
        ResolveOutcome::Ambiguous(candidates)
    }

    /// `true` iff a now-disconnected body's last-known presence had a
    /// session matching `name` (the `not_connected` case — see the
    /// `offline` field's own doc).
    async fn matches_offline(&self, name: &holler_proto::RoutableName) -> bool {
        self.offline.lock().await.values().any(|(hostname, sessions)| {
            if name.has_label() && hostname != name.label() {
                return false;
            }
            sessions.iter().any(|ad| ad.name == name.session())
        })
    }
}

/// The current unix epoch in milliseconds (issue #184's `connected_at`).
fn now_millis_u64() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// The outcome of [`Registry::resolve_session`].
pub enum ResolveOutcome {
    /// Exactly one live session matched.
    Found(LiveHandle, Box<SessionAd>),
    /// No live session matched, live or offline.
    Unknown,
    /// A session by this name was last seen on a body that is not currently
    /// connected (issue #190's own stand-in — see the `offline` field doc).
    NotConnected,
    /// More than one live session matched a bare name — the candidates are
    /// `<label>/<session>` strings for the spec's "lists candidates" wording.
    Ambiguous(Vec<String>),
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #185
mod tests {
    use super::*;

    #[tokio::test]
    async fn find_target_matches_by_token_id_client_id_or_label() {
        let registry = Registry::new();
        let (mut rx, mut cancel_rx, _seq) = registry.insert("cli_1", "kiwi", "tok_1", "127.0.0.1:1").await;

        assert!(matches!(registry.find_target("tok_1").await, TargetLookup::Found(_)));
        assert!(matches!(registry.find_target("cli_1").await, TargetLookup::Found(_)));
        assert!(matches!(registry.find_target("kiwi").await, TargetLookup::Found(_)));
        // A `label/session` form resolves by its label segment.
        assert!(matches!(registry.find_target("kiwi/alpha").await, TargetLookup::Found(_)));
        assert!(matches!(registry.find_target("nobody").await, TargetLookup::NotConnected));

        rx.close();
        cancel_rx.close();
    }

    /// Two live bodies sharing the same hostname (a collision issue #184's
    /// uniqueness enforcement is what actually prevents in practice — this
    /// registry alone does not) resolve as [`TargetLookup::Ambiguous`], not a
    /// silently-picked one.
    #[tokio::test]
    async fn ambiguous_target_is_reported_as_ambiguous() {
        let registry = Registry::new();
        let (mut rx1, mut cancel_rx1, _seq) = registry.insert("cli_1", "kiwi", "tok_1", "127.0.0.1:1").await;
        let (mut rx2, mut cancel_rx2, _seq) = registry.insert("cli_2", "kiwi", "tok_2", "127.0.0.1:1").await;

        assert!(matches!(registry.find_target("kiwi").await, TargetLookup::Ambiguous));
        // Each body's own token/client id still resolves unambiguously.
        assert!(matches!(registry.find_target("tok_1").await, TargetLookup::Found(_)));

        rx1.close();
        rx2.close();
        cancel_rx1.close();
        cancel_rx2.close();
    }

    #[tokio::test]
    async fn confirm_harness_and_advertised_are_independent() {
        let registry = Registry::new();
        let (mut rx, mut cancel_rx, _seq) = registry.insert("cli_1", "kiwi", "tok_1", "127.0.0.1:1").await;
        registry.set_harnesses_advertised("cli_1", vec!["opencode".to_string(), "claude".to_string()]).await;
        assert_eq!(registry.harnesses_known().await, vec!["claude".to_string(), "opencode".to_string()]);
        assert!(registry.harnesses_confirmed().await.is_empty(), "advertising alone confirms nothing");

        registry.confirm_harness("cli_1", "opencode").await;
        let confirmed = registry.harnesses_confirmed().await;
        assert_eq!(confirmed.len(), 1);
        assert_eq!(confirmed[0].id, "opencode");
        assert_eq!(confirmed[0].bodies, vec!["kiwi".to_string()]);

        rx.close();
        cancel_rx.close();
    }

    #[tokio::test]
    async fn session_count_sums_across_bodies() {
        let registry = Registry::new();
        let (mut rx1, mut cancel_rx1, _seq) = registry.insert("cli_1", "kiwi", "tok_1", "127.0.0.1:1").await;
        let (mut rx2, mut cancel_rx2, _seq) = registry.insert("cli_2", "mango", "tok_2", "127.0.0.1:1").await;
        registry.set_session_count("cli_1", 2).await;
        registry.set_session_count("cli_2", 3).await;
        assert_eq!(registry.total_sessions().await, 5);
        rx1.close();
        rx2.close();
        cancel_rx1.close();
        cancel_rx2.close();
    }
}
