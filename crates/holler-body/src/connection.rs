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

mod handshake;
mod session_dispatch;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{Sink, SinkExt, Stream, StreamExt};
use holler_proto::log::{Component, Direction as LogDirection, Event, Severity};
use holler_proto::{Code, CorrelationId, Envelope, PingAck, Presence};
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
    /// The hub rejected the authentication proof, or this connection's hub
    /// `circuit/hello` carried a public key that does not match the one
    /// pinned at `body join` (issue #322: a hard failure, never a prompt,
    /// never trust-on-first-use): stop for good, no retry.
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
        // `attempt` is `&mut`: reset to 0 on a live connect (issue #299 — grows only across consecutive failures).
        let outcome =
            connect_and_serve(state_root, identity, session_manager, configs, &mut attempt, &mut sigint, &mut sigterm)
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
                        since: holler_proto::now_secs(),
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
            since: holler_proto::now_secs(),
            last_frame_at: None,
            attempt: 0,
        },
    )
}

/// `component=wire` debug event for one inbound/outbound JSON-RPC frame
/// (issue #197 / the holler-server#207 regression this closes): `id` is the
/// frame's own correlation id (a request/response), `frame` is the full
/// redacted body — populated only at `noisy` via
/// [`holler_proto::log::frame_at_noisy`], so `quiet` still shows the frame
/// *shape* (method/direction/id) with no body.
/// `Event.method` is `&'static str` throughout this codebase's logging (a
/// handful of hardcoded call sites, never a runtime string) — an inbound
/// frame's method name is a `String` off the wire, so it is mapped onto a
/// matching static label here (falling back to `"other"` for anything not
/// worth a dedicated arm; the real method name is still visible in the
/// frame body itself at `noisy`).
fn static_wire_method(method: &str) -> &'static str {
    match method {
        "session/prompt" => "session/prompt",
        "session/cancel" => "session/cancel",
        "session/answer" => "session/answer",
        "session/update" => "session/update",
        "session/presence" => "session/presence",
        "circuit/ping" => "circuit/ping",
        "circuit/superseded" => "circuit/superseded",
        _ if method.starts_with("query/") => "query",
        _ => "other",
    }
}

fn log_wire_frame(direction: LogDirection, method: &'static str, id: Option<&str>, raw: &str) {
    // `Event.id` is `Option<&'static str>` (a static-lifetime slot for a
    // handful of hardcoded short labels elsewhere in this codebase) — a
    // frame's own runtime correlation id is owned/dynamic, so it goes into
    // `fields` instead, exactly like every other per-frame id in this
    // codebase's wire logging (see `circuit.rs`'s own `log()` convention).
    let fields = id.map(|i| vec![("id", i.to_string())]).unwrap_or_default();
    holler_proto::log::emit(&Event {
        component: Component::Wire,
        severity: Severity::Debug,
        direction,
        method,
        id: None,
        peer: None,
        fields,
        frame: holler_proto::log::frame_at_noisy(raw),
    });
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

/// One connect → authenticate → hello → live-loop attempt. `attempt` is
/// `&mut` (issue #299): reset to 0 on reaching `Connected` — see below.
async fn connect_and_serve(
    state_root: &Path,
    identity: &BodyIdentity,
    session_manager: &Arc<SessionManager>,
    configs: &[SessionConfig],
    attempt: &mut u32,
    sigint: &mut tokio::signal::unix::Signal,
    sigterm: &mut tokio::signal::unix::Signal,
) -> Attempt {
    let _ = crate::connection_state::write(
        state_root,
        &crate::connection_state::ConnectionState {
            state: crate::connection_state::ConnState::Connecting,
            since: holler_proto::now_secs(),
            last_frame_at: None,
            attempt: *attempt,
        },
    );

    let ws = match tokio_tungstenite::connect_async(&identity.server_url).await {
        Ok((ws, _)) => ws,
        Err(e) => return Attempt::Dropped(format!("connect: {e}")),
    };
    let (mut sink, mut stream) = ws.split();

    if let Err(reason) = handshake::authenticate(&mut sink, &mut stream, identity).await {
        return reason;
    }
    if let Err(reason) = handshake::hello_exchange(&mut sink, &mut stream, identity, configs).await {
        return reason;
    }

    // Live: the next drop, whenever it comes, must back off from 0 (#299).
    *attempt = 0;

    info("conn_connected", vec![("server", identity.server_url.clone())]);
    let _ = crate::connection_state::write(
        state_root,
        &crate::connection_state::ConnectionState {
            state: crate::connection_state::ConnState::Connected,
            since: holler_proto::now_secs(),
            last_frame_at: Some(holler_proto::now_secs()),
            attempt: 0,
        },
    );

    let mut conn = LiveConnection::new(state_root, &mut sink, &mut stream, identity, session_manager, configs);
    conn.run(sigint, sigterm).await
}

/// Read the next envelope with a 10s timeout, skipping ping/pong/raw frames.
/// `None` on timeout, close, error, EOF, or a frame that fails to decode.
async fn timeout_next_envelope<St>(stream: &mut St) -> Option<Envelope>
where
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    tokio::time::timeout(Duration::from_secs(10), next_envelope(stream)).await.ok()?
}

/// Issue #207: this is duplicated verbatim in `holler-hub`'s
/// `circuit::next_envelope` (same generic signature, same body). It is
/// deliberately *not* hoisted into `holler_proto` alongside [`now_secs`]/
/// [`now_millis`] (`holler-proto`'s clock helpers): `holler_proto::lib`'s own
/// doc comment states "this crate has no network or async dependency", and
/// this function's bound (`Stream<Item = Result<Message, WsError>>`) is
/// `tokio_tungstenite`-specific — hoisting it would mean adding
/// `tokio-tungstenite`/`futures-util` as dependencies of a crate whose
/// documented charter is exactly *not* to carry transport dependencies. That
/// is a bigger architectural change than this issue's "pure refactor, no
/// behavior change" scope, so the ~8-line loop stays local to each of the two
/// crates that own a transport.
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

pub(crate) async fn send<Snk>(sink: &mut Snk, env: &Envelope) -> Result<(), ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let text = holler_proto::encode(env).map_err(|_| ())?;
    log_wire_frame(
        LogDirection::Out,
        static_wire_method(env.method().unwrap_or("response_or_error")),
        env.id(),
        &text,
    );
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

pub(crate) enum FrameOutcome {
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
/// `outbound_tx`/`outbound_rx` is the channel every `session/prompt`
/// dispatch task's own frames cross to reach `sink` (which has exactly one
/// owner: this loop); `priority_tx`/`priority_rx` (issue #191) is the
/// second, high-priority channel `session/cancel`'s own response frame uses
/// instead, so it is never stuck behind a large `session/update` flush
/// already queued on `outbound_rx`.
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
    /// A second, high-priority outbound channel used **only** for
    /// `session/cancel`'s own response frame (issue #191's priority path):
    /// a `session/cancel` dispatch task's `applied:true` (or refusal) writes
    /// here instead of `outbound_tx`, and [`Self::next_beat`] drains this
    /// channel with `biased` precedence ahead of `outbound_rx` — so the
    /// cancel's own ack is never stuck behind a large `session/update` flush
    /// or another prompt's own outbound frames already queued on the normal
    /// channel.
    priority_tx: mpsc::UnboundedSender<Message>,
    priority_rx: mpsc::UnboundedReceiver<Message>,
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
        let (priority_tx, priority_rx) = mpsc::unbounded_channel::<Message>();
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
            priority_tx,
            priority_rx,
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
        // `biased` (issue #191): `priority_rx` is listed ahead of
        // `outbound_rx` so a queued `session/cancel` ack always wins the tie
        // when both channels have a frame ready — the whole point of the
        // priority path. Every other branch keeps its pre-#191 behaviour
        // (each is checked in turn when ready; none of them contends with
        // `priority_rx` under any test this story or #182/#190 wrote).
        tokio::select! {
            biased;
            _ = self.heartbeat.tick() => Beat::Presence(send_presence_or_drop(self.sink, self.identity, self.session_manager).await),
            changed = self.presence_changed.recv() => presence_changed_beat(&changed, self.sink, self.identity, self.session_manager).await,
            _ = self.detach_poll.tick() => Beat::MaybeEnded(check_detach(self.sink, &self.detach_path, self.state_root).await),
            _ = any_signal(sigint, sigterm) => Beat::Signal,
            _ = tokio::time::sleep_until(last_frame_at + liveness_timeout()) => Beat::LivenessExpired,
            // Folded into one leaf (`next_outbound`) rather than two separate
            // arms here, purely to keep this `select!`'s own arm count (and
            // therefore its cognitive-complexity score) at its pre-#191
            // level — the priority-vs-normal race itself still happens,
            // inside that helper's own `biased` select.
            outbound = next_outbound(&mut self.priority_rx, &mut self.outbound_rx) => Beat::MaybeEnded(relay_outbound(self.sink, outbound).await),
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
        // Every inbound frame, logged once here regardless of which arm below
        // ends up handling it (issue #197 / the holler-server#207 regression:
        // `session/prompt` never showed up at `noisy` because nothing on this
        // path ever logged an inbound frame at all).
        log_wire_frame(
            LogDirection::In,
            static_wire_method(env.method().unwrap_or("response_or_error")),
            env.id(),
            text,
        );
        match env {
            Envelope::Request { id, method, .. } if method == "circuit/ping" => {
                let ack = PingAck { hostname: self.identity.hostname.clone(), ts: holler_proto::now_millis() };
                let Ok(cid) = CorrelationId::parse(&id) else { return FrameOutcome::Continue };
                let resp = Envelope::response(&cid, Some(serde_json::to_value(ack).unwrap_or_default()));
                match send(self.sink, &resp).await {
                    Ok(()) => FrameOutcome::Continue,
                    Err(()) => FrameOutcome::Dropped("send ping ack: socket closed".to_string()),
                }
            }
            Envelope::Request { id, method, params } if method.starts_with("query/") => {
                crate::dispatch::handle_query(self.sink, &id, &method, params, self.identity, self.state_root, self.configs).await
            }
            Envelope::Notification { method, .. } if method == "circuit/superseded" => {
                warn("conn_superseded", vec![]);
                FrameOutcome::Superseded
            }
            Envelope::Request { id, method, params }
                if matches!(method.as_str(), "session/prompt" | "session/cancel" | "session/answer") =>
            {
                // Issue #191's priority path: `session/cancel`'s own
                // response frame goes out via `priority_tx`, not the normal
                // `outbound_tx` — see `priority_tx`'s own doc comment above.
                // `dispatch_session_request` picks between the two channels
                // itself, by method name.
                session_dispatch::dispatch_session_request(
                    self.sink,
                    self.session_manager,
                    &self.outbound_tx,
                    &self.priority_tx,
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
/// Race the priority and normal outbound channels, giving `priority_rx`
/// precedence whenever both have a frame ready (issue #191's priority path:
/// a `session/cancel` response must never queue behind a large
/// `session/update` flush already sitting in `outbound_rx`). Folded into one
/// leaf `select!` so [`LiveConnection::next_beat`]'s own `select!` doesn't
/// need a second arm for it — see that call site's own comment.
async fn next_outbound(
    priority_rx: &mut mpsc::UnboundedReceiver<Message>,
    outbound_rx: &mut mpsc::UnboundedReceiver<Message>,
) -> Option<Message> {
    tokio::select! {
        biased;
        msg = priority_rx.recv() => msg,
        msg = outbound_rx.recv() => msg,
    }
}

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

