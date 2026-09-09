//! `holler body run` (issue #182): the live connection loop — authenticate,
//! exchange hellos, heartbeat via `session/presence`, survive drops with
//! jittered backoff, and honour `body/detach_request`.
//!
//! This is the body's **first talk**: no sessions exist yet (`sessions:[]` in
//! every presence), so there is nothing here about spawning harnesses or
//! dispatching prompts — those are later stories (session manager, Talk).
//! What this story makes true is "the hub can `token ping` this body and get
//! an RTT", i.e. a live, self-healing circuit exists.
//!
//! Decisions made resolving ambiguity in the issue (documented here, not just
//! in the PR, so the next reader of this file sees them too):
//! - The bidirectional hello exchange: the body's own `circuit/hello` request
//!   gets the hub's `Hello` document as its result; the hub's *own*
//!   `circuit/hello` request (sent right after) is the one this body answers
//!   with a bare `{}}`, matching the issue's literal wording for that half.
//! - SIGINT/SIGTERM is honoured inside the live session loop and during the
//!   backoff sleep; a signal that lands during the brief connect+authenticate
//!   +hello handshake is caught as soon as that (bounded, sub-second) phase
//!   finishes rather than interrupting it mid-flight — acceptable because
//!   that window holds no session state to tear down yet.
//!
//! Issue #230 folded the live loop's connection-scoped mutable state (the
//! sink/stream, identity, session manager, configs, timers, and the pending
//! outbound channel) into [`LiveConnection`], turning `live_loop`/`next_beat`/
//! `handle_frame`/`handle_text` into methods on it instead of functions
//! threading 7-15 loose positional parameters — a pure refactor, no behavior
//! change.

mod session_dispatch;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{Sink, SinkExt, Stream, StreamExt};
use holler_proto::log::{Component, Direction as LogDirection, Event, Severity};
use holler_proto::{Authenticate, Code, CorrelationId, Envelope, Hello, HelloRole, PingAck, Presence};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::config::SessionConfig;
use crate::identity::BodyIdentity;
use crate::registry::SessionRegistry;
use crate::session_manager::SessionManager;

/// The heartbeat/presence interval (issue #182 step 3): 15s, or
/// `HOLLER_HEARTBEAT_INTERVAL_MS` for tests that cannot wait 15s for real.
fn heartbeat_interval() -> Duration {
    std::env::var("HOLLER_HEARTBEAT_INTERVAL_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_secs(15))
}

/// The exit outcome `holler body run` applies (ADR 0003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunExit {
    /// A clean end (detach, or SIGINT/SIGTERM): the bin exits 0.
    Ok,
    /// This body has never joined: the bin exits 1.
    NotJoined,
    /// The hub rejected the credential (`-32002`): the bin exits 1, no retry.
    AuthFailed(String),
    /// A state-dir I/O failure: the bin exits 1.
    Io(String),
    /// Another `body run` already holds the instance lock: the bin exits 3.
    LockHeld(String),
}

/// `holler body run` — builds its own throwaway multi-thread runtime (the CLI
/// has none in scope) and drives the loop until a clean end.
///
/// `registry` (issue #187) is this body's configured, validated session list.
/// Issue #190 gives it real teeth: a [`SessionManager`] is started over it
/// (one task per session) for the live's whole lifetime, and its dynamic
/// `presence_doc()` — not `registry`'s own always-`idle` snapshot — is what
/// every `session/presence` this loop emits actually carries. Issue #185's
/// `query/*` answers need the same registry's *config* rows (harness ids,
/// `command[0]` resolution) — derived here once, from `registry` itself,
/// rather than threading a second parallel list down alongside it.
pub fn run(state_root: &Path, registry: SessionRegistry) -> RunExit {
    let identity = match crate::identity::load(state_root) {
        None => {
            eprintln!("error: not joined; run `holler body join` first");
            return RunExit::NotJoined;
        }
        Some(Err(e)) => {
            eprintln!("error: state dir: {e}");
            return RunExit::Io(e.to_string());
        }
        Some(Ok(i)) => i,
    };
    let lock = match crate::instance_lock::acquire(state_root) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: {}", e.message());
            return match e {
                crate::instance_lock::LockError::Held(pid) => RunExit::LockHeld(pid),
                crate::instance_lock::LockError::Io(m) => RunExit::Io(m),
            };
        }
    };
    let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => return RunExit::Io(format!("runtime: {e}")),
    };
    let configs: Vec<SessionConfig> = registry.iter().map(|(_, entry)| entry.config.clone()).collect();
    let exit = rt.block_on(async {
        // Started once, for this process's whole live lifetime: every
        // reconnect attempt below (`connect_and_serve`) shares the same
        // `SessionManager`, so an in-flight turn survives a dropped/retried
        // socket — only the WS connection is retried, never a session.
        let session_manager = Arc::new(SessionManager::start(&registry));
        let exit = run_loop(state_root, &identity, &session_manager, &configs).await;
        // Best-effort graceful shutdown: only possible when no in-flight
        // `session/prompt`/`session/cancel` task still holds its own `Arc`
        // clone. A live turn racing the process's own clean end is left to
        // the OS teardown — the SessionManager's tasks are plain tokio tasks
        // with no state that outlives the process either way.
        if let Ok(sm) = Arc::try_unwrap(session_manager) {
            sm.shutdown().await;
        }
        exit
    });
    drop(lock); // release + remove the lock file on every clean path out.
    exit
}

/// One reconnect attempt's outcome: what the caller (the reconnect loop)
/// should do next.
enum Attempt {
    /// `detach` fired, or a signal arrived: stop for good.
    Ended(RunExit),
    /// The hub rejected the credential: stop for good, no retry.
    AuthFailed(String),
    /// The socket dropped (connect failure, decode error, liveness timeout,
    /// hub close): back off and retry.
    Dropped(String),
}

/// The reconnect loop: connect, authenticate, hello, then live until the
/// circuit ends. A dropped circuit backs off (full jitter, 1s..30s) and
/// tries again, forever, until a clean end or an unretryable auth failure.
async fn run_loop(
    state_root: &Path,
    identity: &BodyIdentity,
    session_manager: &Arc<SessionManager>,
    configs: &[SessionConfig],
) -> RunExit {
    let mut sigint = match signal(SignalKind::interrupt()) {
        Ok(s) => s,
        Err(e) => return RunExit::Io(format!("install SIGINT handler: {e}")),
    };
    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(s) => s,
        Err(e) => return RunExit::Io(format!("install SIGTERM handler: {e}")),
    };

    let mut attempt: u32 = 0;
    loop {
        let outcome =
            connect_and_serve(state_root, identity, session_manager, configs, attempt, &mut sigint, &mut sigterm)
                .await;
        match outcome {
            Attempt::Ended(exit) => return exit,
            Attempt::AuthFailed(msg) => {
                eprintln!("error: authentication failed: {msg}");
                return RunExit::AuthFailed(msg);
            }
            Attempt::Dropped(reason) => {
                warn("conn_dropped", vec![("reason", reason)]);
                let _ = crate::connection_state::write(
                    state_root,
                    &crate::connection_state::ConnectionState {
                        state: crate::connection_state::ConnState::Reconnecting,
                        since: crate::connection_state::now_secs(),
                        last_frame_at: None,
                        attempt,
                    },
                );
                let delay = crate::backoff::delay_random(attempt);
                if wait_or_signal(delay, &mut sigint, &mut sigterm).await {
                    let _ = mark_disconnected(state_root);
                    return RunExit::Ok;
                }
                attempt = attempt.saturating_add(1);
            }
        }
    }
}

/// Resolve as soon as either signal fires. Folding both into one future keeps
/// callers' `tokio::select!` blocks to one arm instead of two, which is what
/// keeps their cognitive complexity under the workspace threshold.
async fn any_signal(sigint: &mut tokio::signal::unix::Signal, sigterm: &mut tokio::signal::unix::Signal) {
    tokio::select! {
        _ = sigint.recv() => {}
        _ = sigterm.recv() => {}
    }
}

/// Sleep for `d`, or return `true` early if a signal arrives first.
async fn wait_or_signal(
    d: Duration,
    sigint: &mut tokio::signal::unix::Signal,
    sigterm: &mut tokio::signal::unix::Signal,
) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(d) => false,
        _ = any_signal(sigint, sigterm) => true,
    }
}

fn mark_disconnected(state_root: &Path) -> std::io::Result<()> {
    crate::connection_state::write(
        state_root,
        &crate::connection_state::ConnectionState {
            state: crate::connection_state::ConnState::Disconnected,
            since: crate::connection_state::now_secs(),
            last_frame_at: None,
            attempt: 0,
        },
    )
}

fn warn(method: &'static str, fields: Vec<(&'static str, String)>) {
    holler_proto::log::emit(&Event {
        component: Component::Session,
        severity: Severity::Warn,
        direction: LogDirection::Local,
        method,
        id: None,
        peer: None,
        fields,
        frame: None,
    });
}

fn info(method: &'static str, fields: Vec<(&'static str, String)>) {
    holler_proto::log::emit(&Event {
        component: Component::Session,
        severity: Severity::Info,
        direction: LogDirection::Local,
        method,
        id: None,
        peer: None,
        fields,
        frame: None,
    });
}

/// One connect → authenticate → hello → live-loop attempt.
async fn connect_and_serve(
    state_root: &Path,
    identity: &BodyIdentity,
    session_manager: &Arc<SessionManager>,
    configs: &[SessionConfig],
    attempt: u32,
    sigint: &mut tokio::signal::unix::Signal,
    sigterm: &mut tokio::signal::unix::Signal,
) -> Attempt {
    let _ = crate::connection_state::write(
        state_root,
        &crate::connection_state::ConnectionState {
            state: crate::connection_state::ConnState::Connecting,
            since: crate::connection_state::now_secs(),
            last_frame_at: None,
            attempt,
        },
    );

    let ws = match tokio_tungstenite::connect_async(&identity.server_url).await {
        Ok((ws, _)) => ws,
        Err(e) => return Attempt::Dropped(format!("connect: {e}")),
    };
    let (mut sink, mut stream) = ws.split();

    if let Err(reason) = authenticate(&mut sink, &mut stream, identity).await {
        return reason;
    }
    if hello_exchange(&mut sink, &mut stream, identity, configs).await.is_err() {
        return Attempt::Dropped("hello exchange failed".to_string());
    }

    info("conn_connected", vec![("server", identity.server_url.clone())]);
    let _ = crate::connection_state::write(
        state_root,
        &crate::connection_state::ConnectionState {
            state: crate::connection_state::ConnState::Connected,
            since: crate::connection_state::now_secs(),
            last_frame_at: Some(crate::connection_state::now_secs()),
            attempt: 0,
        },
    );

    let mut conn = LiveConnection::new(state_root, &mut sink, &mut stream, identity, session_manager, configs);
    conn.run(sigint, sigterm).await
}

/// Send `circuit/authenticate` and await the answer. `-32002` maps to
/// [`Attempt::AuthFailed`] (no retry, per the issue); every other failure
/// (connect-adjacent decode errors, a foreign error code, a closed socket, a
/// 10s timeout) maps to [`Attempt::Dropped`] (retry with backoff).
async fn authenticate<Snk, St>(sink: &mut Snk, stream: &mut St, identity: &BodyIdentity) -> Result<(), Attempt>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    let cid = CorrelationId::mint_body();
    let params = Authenticate {
        token_id: identity.token_id.clone(),
        credential: identity.credential.clone(),
        hostname: identity.hostname.clone(),
    };
    let params = serde_json::to_value(params).map_err(|e| Attempt::Dropped(format!("encode auth: {e}")))?;
    let req = Envelope::request(&cid, "circuit/authenticate", Some(params));
    send(sink, &req).await.map_err(|_| Attempt::Dropped("send auth: socket closed".to_string()))?;

    let env = timeout_next_envelope(stream)
        .await
        .ok_or_else(|| Attempt::Dropped("no answer to circuit/authenticate".to_string()))?;
    match env {
        Envelope::Response { id, .. } if id == cid.as_str() => Ok(()),
        Envelope::Error { error, .. } if error.code == Code::Unauthenticated.jsonrpc() => {
            Err(Attempt::AuthFailed(error.message))
        }
        Envelope::Error { error, .. } => Err(Attempt::Dropped(format!("authenticate refused: {}", error.message))),
        _ => Err(Attempt::Dropped("unexpected reply to circuit/authenticate".to_string())),
    }
}

/// The bidirectional hello exchange (see the module doc's "Decisions made").
/// `configs` (issue #185) is this body's own session config — its harness
/// ids populate the hello's `harnesses` field, which is what the hub's
/// confirmation pass ([`crate::query`]'s hub-side counterpart, `holler_hub::
/// circuit::confirm_harnesses`) probes right after this exchange completes.
async fn hello_exchange<Snk, St>(
    sink: &mut Snk,
    stream: &mut St,
    identity: &BodyIdentity,
    configs: &[SessionConfig],
) -> Result<(), ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    let mut harnesses: Vec<String> = configs.iter().map(|c| c.harness.clone()).collect();
    harnesses.sort();
    harnesses.dedup();

    let cid = CorrelationId::mint_body();
    let hello = Hello {
        protocol: holler_proto::PROTOCOL_VERSION,
        protocol_min: holler_proto::PROTOCOL_MIN,
        protocol_max: holler_proto::PROTOCOL_MAX,
        role: HelloRole::Body,
        hostname: identity.hostname.clone(),
        token_id: Some(identity.token_id.clone()),
        client_id: Some(identity.client_id.clone()),
        features: Vec::new(),
        harnesses: Some(harnesses),
        harnesses_known: None,
        harnesses_confirmed: None,
        sessions: Some(Vec::new()),
    };
    let params = serde_json::to_value(hello).map_err(|_| ())?;
    let req = Envelope::request(&cid, "circuit/hello", Some(params));
    send(sink, &req).await.map_err(|_| ())?;

    match timeout_next_envelope(stream).await {
        Some(Envelope::Response { id, .. }) if id == cid.as_str() => {}
        _ => return Err(()),
    }

    // The hub's own hello: a request we must answer with `{}`.
    match timeout_next_envelope(stream).await {
        Some(Envelope::Request { id, method, .. }) if method == "circuit/hello" => {
            let cid = CorrelationId::parse(&id).map_err(|_| ())?;
            let ack = Envelope::response(&cid, Some(serde_json::json!({})));
            send(sink, &ack).await.map_err(|_| ())
        }
        _ => Err(()),
    }
}

/// Read the next envelope with a 10s timeout, skipping ping/pong/raw frames.
/// `None` on timeout, close, error, EOF, or a frame that fails to decode.
async fn timeout_next_envelope<St>(stream: &mut St) -> Option<Envelope>
where
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    tokio::time::timeout(Duration::from_secs(10), next_envelope(stream)).await.ok()?
}

async fn next_envelope<St>(stream: &mut St) -> Option<Envelope>
where
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    loop {
        match stream.next().await {
            Some(Ok(Message::Text(t))) => return holler_proto::decode(&t).ok(),
            Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => continue,
            _ => return None,
        }
    }
}

async fn send<Snk>(sink: &mut Snk, env: &Envelope) -> Result<(), ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let text = holler_proto::encode(env).map_err(|_| ())?;
    sink.send(Message::text(text)).await.map_err(|_| ())?;
    sink.flush().await.map_err(|_| ())
}

/// How long with no frame of any kind from the hub before the socket is
/// treated as dead (issue #182 step 4: "3 × interval").
fn liveness_timeout() -> Duration {
    heartbeat_interval() * 3
}

/// How often `body/detach_request` is polled (issue #182 step 6).
const DETACH_POLL: Duration = Duration::from_millis(500);

/// One `select!` tick's outcome (see [`LiveConnection::run`]'s own comment):
/// `Presence`/`MaybeEnded` both carry "keep going, or end the circuit with
/// this `Attempt`" and share a match arm below — the two names stay distinct
/// only so each call site reads clearly for what it is.
enum Beat {
    Continue,
    Presence(Option<Attempt>),
    MaybeEnded(Option<Attempt>),
    Signal,
    LivenessExpired,
    Frame(FrameOutcome),
}

enum FrameOutcome {
    Continue,
    Superseded,
    Dropped(String),
}

/// Issue #230: the live loop's connection-scoped mutable state, bundled into
/// one owned struct instead of threaded as 7-15 loose positional parameters
/// across `live_loop`/`next_beat`/`handle_frame`/`handle_text` (which is what
/// forced the clippy "too many arguments" `allow` escapes those functions
/// used to carry). `sink`/`stream` are the socket halves; `identity`/
/// `session_manager`/`configs`/`state_root` are the body's own read-mostly
/// context; `heartbeat`/`detach_poll`/`last_frame_at` are the loop's timers;
/// `outbound_tx`/`outbound_rx` is the channel every `session/prompt`/
/// `session/cancel` dispatch task's own frames cross to reach `sink` (which
/// has exactly one owner: this loop).
struct LiveConnection<'a, Snk, St> {
    sink: &'a mut Snk,
    stream: &'a mut St,
    identity: &'a BodyIdentity,
    session_manager: &'a Arc<SessionManager>,
    configs: &'a [SessionConfig],
    state_root: &'a Path,
    detach_path: PathBuf,
    heartbeat: tokio::time::Interval,
    detach_poll: tokio::time::Interval,
    last_frame_at: tokio::time::Instant,
    outbound_tx: mpsc::UnboundedSender<Message>,
    outbound_rx: mpsc::UnboundedReceiver<Message>,
    presence_changed: tokio::sync::broadcast::Receiver<holler_proto::SessionName>,
}

impl<'a, Snk, St> LiveConnection<'a, Snk, St>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    fn new(
        state_root: &'a Path,
        sink: &'a mut Snk,
        stream: &'a mut St,
        identity: &'a BodyIdentity,
        session_manager: &'a Arc<SessionManager>,
        configs: &'a [SessionConfig],
    ) -> Self {
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel::<Message>();
        // Issue #190: push `session/presence` immediately on any session's
        // state change (not just at the next heartbeat) — this is what lets
        // the hub's own `say` busy/stalled check (issue #190's roster
        // stand-in, see `holler-hub`'s `live` module) read a live-enough
        // snapshot instead of one up to a full heartbeat interval stale.
        // `SessionManager` has always published this signal (issue #189);
        // this story is the first consumer.
        let presence_changed = session_manager.subscribe_presence_changes();
        Self {
            sink,
            stream,
            identity,
            session_manager,
            configs,
            state_root,
            detach_path: detach_request_path(state_root),
            heartbeat: tokio::time::interval(heartbeat_interval()),
            detach_poll: tokio::time::interval(DETACH_POLL),
            last_frame_at: tokio::time::Instant::now(),
            outbound_tx,
            outbound_rx,
            presence_changed,
        }
    }

    /// The live session: heartbeat, answer pings, watch for detach/liveness/
    /// signals, until the circuit ends one way or another. Split into
    /// [`Self::next_beat`] (race every event source) and [`Self::apply_beat`]
    /// (interpret the one that fired) — see each method's own doc for why.
    async fn run(
        &mut self,
        sigint: &mut tokio::signal::unix::Signal,
        sigterm: &mut tokio::signal::unix::Signal,
    ) -> Attempt {
        self.heartbeat.tick().await; // the first tick fires immediately; consume it.

        if send_presence(self.sink, self.identity, self.session_manager).await.is_err() {
            return Attempt::Dropped("send initial presence: socket closed".to_string());
        }

        loop {
            let beat = self.next_beat(sigint, sigterm).await;
            if let Some(attempt) = self.apply_beat(beat).await {
                return attempt;
            }
        }
    }

    /// Race every live-loop event source and produce the one [`Beat`] that
    /// fired. Split out of [`Self::run`] entirely (not just each arm's own
    /// branching, as the other methods do) — a bare `select!` this wide is
    /// itself the dominant share of the caller's cognitive-complexity score,
    /// independent of how much branching logic each arm still carries (see
    /// [`Self::apply_beat`] for the interpretation step this hands off to).
    async fn next_beat(
        &mut self,
        sigint: &mut tokio::signal::unix::Signal,
        sigterm: &mut tokio::signal::unix::Signal,
    ) -> Beat {
        let last_frame_at = self.last_frame_at;
        tokio::select! {
            _ = self.heartbeat.tick() => Beat::Presence(send_presence_or_drop(self.sink, self.identity, self.session_manager).await),
            changed = self.presence_changed.recv() => presence_changed_beat(&changed, self.sink, self.identity, self.session_manager).await,
            _ = self.detach_poll.tick() => Beat::MaybeEnded(check_detach(self.sink, &self.detach_path, self.state_root).await),
            _ = any_signal(sigint, sigterm) => Beat::Signal,
            _ = tokio::time::sleep_until(last_frame_at + liveness_timeout()) => Beat::LivenessExpired,
            outbound = self.outbound_rx.recv() => Beat::MaybeEnded(relay_outbound(self.sink, outbound).await),
            frame = self.stream.next() => Beat::Frame(self.handle_frame(frame).await),
        }
    }

    /// Interpret one [`Beat`]: `Some` means the circuit ends with that
    /// `Attempt`, `None` means keep looping (updating `last_frame_at` first,
    /// for `Beat::Frame(FrameOutcome::Continue)`). Split out of [`Self::run`]
    /// (see its own comment) — the flat interpretation `match` was itself a
    /// meaningful share of that function's cognitive-complexity score.
    async fn apply_beat(&mut self, beat: Beat) -> Option<Attempt> {
        match beat {
            Beat::Continue => None,
            Beat::Presence(None) | Beat::MaybeEnded(None) => None,
            Beat::Presence(Some(dropped)) | Beat::MaybeEnded(Some(dropped)) => Some(dropped),
            Beat::Signal => {
                close(self.sink).await;
                let _ = mark_disconnected(self.state_root);
                Some(Attempt::Ended(RunExit::Ok))
            }
            Beat::LivenessExpired => {
                Some(Attempt::Dropped("no frame from the hub within the liveness window".to_string()))
            }
            Beat::Frame(FrameOutcome::Continue) => {
                self.last_frame_at = tokio::time::Instant::now();
                None
            }
            Beat::Frame(FrameOutcome::Superseded) => Some(Attempt::Ended(RunExit::Ok)),
            Beat::Frame(FrameOutcome::Dropped(reason)) => Some(Attempt::Dropped(reason)),
        }
    }

    /// Handle one inbound WS message in the live loop.
    async fn handle_frame(&mut self, frame: Option<Result<Message, WsError>>) -> FrameOutcome {
        match frame {
            Some(Ok(Message::Text(t))) => self.handle_text(&t).await,
            Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => {
                FrameOutcome::Continue
            }
            Some(Ok(Message::Binary(_))) => FrameOutcome::Continue,
            Some(Ok(Message::Close(_))) => FrameOutcome::Dropped("hub closed the socket".to_string()),
            Some(Err(e)) => FrameOutcome::Dropped(format!("socket error: {e}")),
            None => FrameOutcome::Dropped("socket EOF".to_string()),
        }
    }

    /// Dispatch one decoded text frame: `circuit/ping`/`circuit/superseded`
    /// (issue #182); issue #185's four `query/*` requests, answered from
    /// local state per [`crate::query`] (the same document a local `body
    /// status`/`caps`/`support`/`query` CLI leaf builds, now also reachable
    /// when the hub forwards a `hub query TARGET …` over this live socket);
    /// and — issue #190 — `session/prompt`/`session/cancel`, each spawned as
    /// its own task against `session_manager` so a long-running turn on one
    /// session never blocks this loop's heartbeat, a ping reply, or another
    /// session's own turn. Every other method a later story owns is still
    /// answered `-32601`.
    async fn handle_text(&mut self, text: &str) -> FrameOutcome {
        let env = match holler_proto::decode(text) {
            Ok(e) => e,
            Err(_) => return FrameOutcome::Continue, // a malformed frame is logged upstream; not fatal to the circuit.
        };
        match env {
            Envelope::Request { id, method, .. } if method == "circuit/ping" => {
                let ack = PingAck { hostname: self.identity.hostname.clone(), ts: now_millis() };
                let Ok(cid) = CorrelationId::parse(&id) else { return FrameOutcome::Continue };
                let resp = Envelope::response(&cid, Some(serde_json::to_value(ack).unwrap_or_default()));
                match send(self.sink, &resp).await {
                    Ok(()) => FrameOutcome::Continue,
                    Err(()) => FrameOutcome::Dropped("send ping ack: socket closed".to_string()),
                }
            }
            Envelope::Request { id, method, params } if method.starts_with("query/") => {
                handle_query(self.sink, &id, &method, params, self.identity, self.state_root, self.configs).await
            }
            Envelope::Notification { method, .. } if method == "circuit/superseded" => {
                warn("conn_superseded", vec![]);
                FrameOutcome::Superseded
            }
            Envelope::Request { id, method, params }
                if matches!(method.as_str(), "session/prompt" | "session/cancel" | "session/answer") =>
            {
                session_dispatch::dispatch_session_request(
                    self.sink,
                    self.session_manager,
                    &self.outbound_tx,
                    id,
                    method,
                    params,
                )
                .await
            }
            Envelope::Request { id, .. } => {
                let Ok(cid) = CorrelationId::parse(&id) else { return FrameOutcome::Continue };
                let err = holler_proto::WireError::new(Code::MethodNotFound, "not implemented on this body yet", None);
                let _ = send(self.sink, &Envelope::error_frame(&cid, &err)).await;
                FrameOutcome::Continue
            }
            Envelope::Notification { .. } | Envelope::Response { .. } | Envelope::Error { .. } => FrameOutcome::Continue,
        }
    }
}

/// `send_presence`, mapped to the `Attempt::Dropped` the caller should
/// return on failure (or `None` to keep looping). Split out of
/// [`LiveConnection::run`] (used at both its heartbeat and presence-changed
/// call sites) to keep that `select!`'s cognitive complexity under the
/// workspace threshold.
async fn send_presence_or_drop<Snk>(
    sink: &mut Snk,
    identity: &BodyIdentity,
    session_manager: &SessionManager,
) -> Option<Attempt>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    if send_presence(sink, identity, session_manager).await.is_err() {
        Some(Attempt::Dropped("send presence: socket closed".to_string()))
    } else {
        None
    }
}

/// Whether a `presence_changed` broadcast result is worth an immediate
/// resend — `true` for an actual change, and (deliberately) also for a
/// `Lagged` overrun, since `presence_doc()` always reflects the *current*
/// state regardless of how many discrete changes a slow subscriber missed.
/// `Closed` cannot happen while `session_manager` (held by this same call
/// chain, in `connection::run`) outlives [`LiveConnection`].
fn presence_worth_resending<T>(changed: &Result<T, tokio::sync::broadcast::error::RecvError>) -> bool {
    changed.is_ok() || matches!(changed, Err(tokio::sync::broadcast::error::RecvError::Lagged(_)))
}

/// The `presence_changed` arm's own [`Beat`] — split out of the `select!` in
/// [`LiveConnection::next_beat`] for the same reason as
/// [`send_presence_or_drop`].
async fn presence_changed_beat<Snk, T>(
    changed: &Result<T, tokio::sync::broadcast::error::RecvError>,
    sink: &mut Snk,
    identity: &BodyIdentity,
    session_manager: &SessionManager,
) -> Beat
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    if presence_worth_resending(changed) {
        Beat::Presence(send_presence_or_drop(sink, identity, session_manager).await)
    } else {
        Beat::Continue
    }
}

/// Check (and, if requested, act on) `detach_request`. Split out of
/// [`LiveConnection::next_beat`] for the same reason as
/// [`send_presence_or_drop`].
async fn check_detach<Snk>(sink: &mut Snk, detach_path: &Path, state_root: &Path) -> Option<Attempt>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    if !detach_path.exists() {
        return None;
    }
    close(sink).await;
    let _ = mark_disconnected(state_root);
    let _ = std::fs::remove_file(detach_path);
    Some(Attempt::Ended(RunExit::Ok))
}

/// Forward one frame a `session/prompt`/`session/cancel` dispatch task
/// produced onto the wire. `None` (no message — the outbound channel is
/// empty right now, not closed) keeps the loop going; split out of
/// [`LiveConnection::next_beat`] for the same reason as
/// [`send_presence_or_drop`].
async fn relay_outbound<Snk>(sink: &mut Snk, outbound: Option<Message>) -> Option<Attempt>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    // `None` means every sender (this loop's own `outbound_tx`, cloned into
    // each dispatch task) is gone — unreachable while this loop still holds
    // its own clone in scope, but matched defensively rather than
    // `.expect()`-ing a channel invariant.
    let msg = outbound?;
    if sink.send(msg).await.is_err() || sink.flush().await.is_err() {
        Some(Attempt::Dropped("send outbound frame: socket closed".to_string()))
    } else {
        None
    }
}

fn detach_request_path(state_root: &Path) -> PathBuf {
    state_root.join("body").join("detach_request")
}

async fn close<Snk>(sink: &mut Snk)
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let _ = sink.send(Message::Close(None)).await;
    let _ = sink.flush().await;
}

async fn send_presence<Snk>(sink: &mut Snk, identity: &BodyIdentity, session_manager: &SessionManager) -> Result<(), ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    // Issue #190: the *live* per-session state (idle/working/input-required,
    // turn timing, `last_turn`) — not the always-`idle` config snapshot issue
    // #182 shipped before any session ever ran a turn.
    let presence: Presence = session_manager.presence_doc(identity.hostname.clone());
    let params = serde_json::to_value(presence).map_err(|_| ())?;
    send(sink, &Envelope::notification("session/presence", Some(params))).await
}

/// Answer one `query/*` request (issue #185) from local state — see
/// [`crate::query`] for the document builders. `query/support` with an
/// unknown id answers `-32006`; every other case answers `Ok`.
async fn handle_query<Snk>(
    sink: &mut Snk,
    id: &str,
    method: &str,
    params: Option<serde_json::Value>,
    identity: &BodyIdentity,
    state_root: &Path,
    configs: &[SessionConfig],
) -> FrameOutcome
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let Ok(cid) = CorrelationId::parse(id) else {
        return FrameOutcome::Continue;
    };
    let result: Result<serde_json::Value, holler_proto::WireError> = match method {
        "query/status" => Ok(serde_json::to_value(crate::query::local_status(state_root, Some(identity), configs))
            .unwrap_or_default()),
        "query/caps" => Ok(serde_json::to_value(crate::query::local_caps(state_root, Some(identity), configs))
            .unwrap_or_default()),
        "query/support" => {
            let feature = params
                .as_ref()
                .and_then(|p| p.get("feature"))
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            crate::query::local_support(feature, configs).map(|s| serde_json::to_value(s).unwrap_or_default())
        }
        "query/protocol" => holler_proto::ProtocolParams::parse_version(params.as_ref())
            .map(|version| serde_json::to_value(crate::query::local_protocol(version)).unwrap_or_default()),
        _ => Err(holler_proto::WireError::new(Code::MethodNotFound, "unknown query method", None)),
    };
    let send_result = match result {
        Ok(value) => send(sink, &Envelope::response(&cid, Some(value))).await,
        Err(e) => send(sink, &Envelope::error_frame(&cid, &e)).await,
    };
    match send_result {
        Ok(()) => FrameOutcome::Continue,
        Err(()) => FrameOutcome::Dropped("send query answer: socket closed".to_string()),
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
