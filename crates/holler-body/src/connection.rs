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

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::{Sink, SinkExt, Stream, StreamExt};
use holler_proto::log::{Component, Direction as LogDirection, Event, Severity};
use holler_proto::{
    Authenticate, Code, CorrelationId, Envelope, Hello, HelloRole, PingAck, Presence, SessionAd,
};
use tokio::signal::unix::{signal, SignalKind};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::identity::BodyIdentity;

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
/// `sessions` (issue #187) is this body's configured session list, already
/// validated + registered ([`crate::registry::SessionRegistry::presence_doc`]'s
/// output) — it is sent verbatim in every `session/presence` this loop emits,
/// replacing the always-empty `sessions:[]` issue #182 shipped (no session
/// config existed yet on that story).
pub fn run(state_root: &Path, sessions: Vec<SessionAd>) -> RunExit {
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
    let exit = rt.block_on(run_loop(state_root, &identity, &sessions));
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
async fn run_loop(state_root: &Path, identity: &BodyIdentity, sessions: &[SessionAd]) -> RunExit {
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
        let outcome = connect_and_serve(state_root, identity, sessions, attempt, &mut sigint, &mut sigterm).await;
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
    sessions: &[SessionAd],
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
    if hello_exchange(&mut sink, &mut stream, identity).await.is_err() {
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

    live_loop(state_root, &mut sink, &mut stream, identity, sessions, sigint, sigterm).await
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
async fn hello_exchange<Snk, St>(sink: &mut Snk, stream: &mut St, identity: &BodyIdentity) -> Result<(), ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
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
        harnesses: Some(Vec::new()),
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

/// The live session: heartbeat, answer pings, watch for detach/liveness/
/// signals, until the circuit ends one way or another.
async fn live_loop<Snk, St>(
    state_root: &Path,
    sink: &mut Snk,
    stream: &mut St,
    identity: &BodyIdentity,
    sessions: &[SessionAd],
    sigint: &mut tokio::signal::unix::Signal,
    sigterm: &mut tokio::signal::unix::Signal,
) -> Attempt
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    let detach_path = detach_request_path(state_root);
    let mut heartbeat = tokio::time::interval(heartbeat_interval());
    heartbeat.tick().await; // the first tick fires immediately; consume it.
    let mut detach_poll = tokio::time::interval(DETACH_POLL);
    let mut last_frame_at = tokio::time::Instant::now();

    if send_presence(sink, identity, sessions).await.is_err() {
        return Attempt::Dropped("send initial presence: socket closed".to_string());
    }

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                if send_presence(sink, identity, sessions).await.is_err() {
                    return Attempt::Dropped("send presence: socket closed".to_string());
                }
            }
            _ = detach_poll.tick() => {
                if detach_path.exists() {
                    close(sink).await;
                    let _ = mark_disconnected(state_root);
                    let _ = std::fs::remove_file(&detach_path);
                    return Attempt::Ended(RunExit::Ok);
                }
            }
            _ = any_signal(sigint, sigterm) => {
                close(sink).await;
                let _ = mark_disconnected(state_root);
                return Attempt::Ended(RunExit::Ok);
            }
            _ = tokio::time::sleep_until(last_frame_at + liveness_timeout()) => {
                return Attempt::Dropped("no frame from the hub within the liveness window".to_string());
            }
            frame = stream.next() => {
                match handle_frame(sink, frame, identity).await {
                    FrameOutcome::Continue => last_frame_at = tokio::time::Instant::now(),
                    FrameOutcome::Superseded => return Attempt::Ended(RunExit::Ok),
                    FrameOutcome::Dropped(reason) => return Attempt::Dropped(reason),
                }
            }
        }
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

async fn send_presence<Snk>(sink: &mut Snk, identity: &BodyIdentity, sessions: &[SessionAd]) -> Result<(), ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let presence = Presence {
        hostname: identity.hostname.clone(),
        sessions: sessions.to_vec(),
    };
    let params = serde_json::to_value(presence).map_err(|_| ())?;
    send(sink, &Envelope::notification("session/presence", Some(params))).await
}

enum FrameOutcome {
    Continue,
    Superseded,
    Dropped(String),
}

/// Handle one inbound WS message in the live loop.
async fn handle_frame<Snk>(
    sink: &mut Snk,
    frame: Option<Result<Message, WsError>>,
    identity: &BodyIdentity,
) -> FrameOutcome
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    match frame {
        Some(Ok(Message::Text(t))) => handle_text(sink, &t, identity).await,
        Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => FrameOutcome::Continue,
        Some(Ok(Message::Binary(_))) => FrameOutcome::Continue,
        Some(Ok(Message::Close(_))) => FrameOutcome::Dropped("hub closed the socket".to_string()),
        Some(Err(e)) => FrameOutcome::Dropped(format!("socket error: {e}")),
        None => FrameOutcome::Dropped("socket EOF".to_string()),
    }
}

/// Dispatch one decoded text frame (issue #182 step 8's inbound methods that
/// are in scope for this story — `circuit/ping`, `circuit/superseded`; every
/// other method a later story owns is answered `-32601` for now, since no
/// session/query dispatch exists yet).
async fn handle_text<Snk>(sink: &mut Snk, text: &str, identity: &BodyIdentity) -> FrameOutcome
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    let env = match holler_proto::decode(text) {
        Ok(e) => e,
        Err(_) => return FrameOutcome::Continue, // a malformed frame is logged upstream; not fatal to the circuit.
    };
    match env {
        Envelope::Request { id, method, .. } if method == "circuit/ping" => {
            let ack = PingAck { hostname: identity.hostname.clone(), ts: now_millis() };
            let Ok(cid) = CorrelationId::parse(&id) else { return FrameOutcome::Continue };
            let resp = Envelope::response(&cid, Some(serde_json::to_value(ack).unwrap_or_default()));
            match send(sink, &resp).await {
                Ok(()) => FrameOutcome::Continue,
                Err(()) => FrameOutcome::Dropped("send ping ack: socket closed".to_string()),
            }
        }
        Envelope::Notification { method, .. } if method == "circuit/superseded" => {
            warn("conn_superseded", vec![]);
            FrameOutcome::Superseded
        }
        Envelope::Request { id, .. } => {
            let Ok(cid) = CorrelationId::parse(&id) else { return FrameOutcome::Continue };
            let err = holler_proto::WireError::new(Code::MethodNotFound, "not implemented on this body yet", None);
            let _ = send(sink, &Envelope::error_frame(&cid, &err)).await;
            FrameOutcome::Continue
        }
        Envelope::Notification { .. } | Envelope::Response { .. } | Envelope::Error { .. } => FrameOutcome::Continue,
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
